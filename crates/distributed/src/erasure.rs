use crate::error::DistributedError;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const DEFAULT_DATA_SHARDS: usize = 4;
const DEFAULT_PARITY_SHARDS: usize = 2;
const DEFAULT_SHARD_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ErasureConfig {
    pub data_shards: usize,
    pub parity_shards: usize,
    pub shard_size: usize,
}

impl Default for ErasureConfig {
    fn default() -> Self {
        Self {
            data_shards: DEFAULT_DATA_SHARDS,
            parity_shards: DEFAULT_PARITY_SHARDS,
            shard_size: DEFAULT_SHARD_SIZE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shard {
    pub index: u8,
    pub data: Vec<u8>,
    pub is_parity: bool,
    pub checksum: [u8; 32],
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub trait ErasureCoder: Send + Sync {
    fn encode(&self, data: &[u8]) -> Result<Vec<Shard>, DistributedError>;
    fn decode(&self, shards: &[Option<Shard>]) -> Result<Vec<u8>, DistributedError>;
    fn reconstruct(&self, shards: &[Option<Shard>]) -> Result<Option<Shard>, DistributedError>;
}

pub struct XorErasureCoder {
    config: ErasureConfig,
}

impl XorErasureCoder {
    pub fn new(config: ErasureConfig) -> Self {
        Self { config }
    }
}

fn pad_to_multiple(data: &[u8], n: usize) -> Vec<u8> {
    if n == 0 {
        return data.to_vec();
    }
    let rem = data.len() % n;
    if rem == 0 {
        data.to_vec()
    } else {
        let mut padded = data.to_vec();
        padded.resize(data.len() + n - rem, 0);
        padded
    }
}

impl ErasureCoder for XorErasureCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<Shard>, DistributedError> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let data_shards = self.config.data_shards;
        let padded = pad_to_multiple(data, data_shards);
        let chunk_size = padded.len() / data_shards;
        if chunk_size == 0 {
            return Err(DistributedError::EncodingFailed {
                reason: "data too small for shard count".into(),
            });
        }

        let mut shards = Vec::with_capacity(data_shards + 1);
        let mut parity = vec![0u8; chunk_size];

        for i in 0..data_shards {
            let start = i * chunk_size;
            let chunk = &padded[start..start + chunk_size];
            for (p, &b) in parity.iter_mut().zip(chunk.iter()) {
                *p ^= b;
            }
            shards.push(Shard {
                index: i as u8,
                data: chunk.to_vec(),
                is_parity: false,
                checksum: sha256(chunk),
            });
        }

        shards.push(Shard {
            index: data_shards as u8,
            data: parity.clone(),
            is_parity: true,
            checksum: sha256(&parity),
        });

        Ok(shards)
    }

    fn decode(&self, shards: &[Option<Shard>]) -> Result<Vec<u8>, DistributedError> {
        let mut data_shard_count = 0;
        let mut parity_found = false;
        for s in shards {
            if let Some(ref s) = *s {
                if s.is_parity {
                    parity_found = true;
                } else {
                    data_shard_count += 1;
                }
            }
        }

        let expected_data = self.config.data_shards;

        if data_shard_count == expected_data {
            let mut indices: Vec<(u8, &Shard)> = Vec::new();
            for s in shards {
                if let Some(ref s) = *s
                    && !s.is_parity {
                        indices.push((s.index, s));
                    }
            }
            indices.sort_by_key(|(i, _)| *i);
            let mut result = Vec::new();
            for (_, s) in &indices {
                result.extend_from_slice(&s.data);
            }
            Ok(result)
        } else if data_shard_count == expected_data - 1 && parity_found {
            let parity_shard = shards
                .iter()
                .find_map(|s| s.as_ref().filter(|s| s.is_parity))
                .unwrap();

            let mut data_shards_map: Vec<(u8, &Shard)> = Vec::new();
            for s in shards {
                if let Some(ref s) = *s
                    && !s.is_parity {
                        data_shards_map.push((s.index, s));
                    }
            }
            data_shards_map.sort_by_key(|(i, _)| *i);

            let present_indices: HashSet<u8> = data_shards_map.iter().map(|(i, _)| *i).collect();
            let mut missing_index: Option<u8> = None;
            for i in 0..expected_data as u8 {
                if !present_indices.contains(&i) {
                    missing_index = Some(i);
                    break;
                }
            }

            let missing_idx = missing_index.ok_or(DistributedError::DecodingFailed {
                reason: "no missing index found".into(),
            })?;

            let mut recovered = parity_shard.data.clone();
            for (_, ds) in &data_shards_map {
                for (r, &b) in recovered.iter_mut().zip(ds.data.iter()) {
                    *r ^= b;
                }
            }

            let mut result = Vec::new();
            for i in 0..expected_data as u8 {
                if i == missing_idx {
                    result.extend_from_slice(&recovered);
                } else if let Some((_, s)) = data_shards_map.iter().find(|(idx, _)| *idx == i) {
                    result.extend_from_slice(&s.data);
                }
            }
            Ok(result)
        } else {
            Err(DistributedError::DecodingFailed {
                reason: format!(
                    "insufficient shards: {} data (need {}), parity={}",
                    data_shard_count, expected_data, parity_found
                ),
            })
        }
    }

    fn reconstruct(&self, shards: &[Option<Shard>]) -> Result<Option<Shard>, DistributedError> {
        let mut data_indices: Vec<u8> = Vec::new();
        let mut parity_opt: Option<&Shard> = None;
        for s in shards {
            if let Some(ref s) = *s {
                if s.is_parity {
                    parity_opt = Some(s);
                } else {
                    data_indices.push(s.index);
                }
            }
        }

        if data_indices.len() != self.config.data_shards - 1 || parity_opt.is_none() {
            return Ok(None);
        }

        let parity = parity_opt.unwrap();
        let mut missing_index: Option<u8> = None;
        let present: HashSet<u8> = data_indices.iter().copied().collect();
        for i in 0..self.config.data_shards as u8 {
            if !present.contains(&i) {
                missing_index = Some(i);
                break;
            }
        }

        let missing_idx = match missing_index {
            Some(idx) => idx,
            None => return Ok(None),
        };

        let mut recovered = parity.data.clone();
        for s in shards {
            if let Some(ref s) = *s
                && !s.is_parity {
                    for (r, &b) in recovered.iter_mut().zip(s.data.iter()) {
                        *r ^= b;
                    }
                }
        }

        Ok(Some(Shard {
            index: missing_idx,
            data: recovered.clone(),
            is_parity: false,
            checksum: sha256(&recovered),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_coder() -> XorErasureCoder {
        XorErasureCoder::new(ErasureConfig {
            data_shards: 4,
            parity_shards: 1,
            shard_size: 1024,
        })
    }

    #[test]
    fn test_encode_roundtrip() {
        let coder = default_coder();
        let data = b"hello distributed world!".to_vec();
        let encoded = coder.encode(&data).unwrap();
        assert_eq!(encoded.len(), 5);

        let shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_single_shard_loss_recovery() {
        let coder = default_coder();
        let data = b"single shard loss test data here".to_vec();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_two_shard_loss_recovery_fails() {
        let coder = default_coder();
        let data = b"two shards lost".to_vec();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[1] = None;

        let result = coder.decode(&shards);
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_verification() {
        let coder = default_coder();
        let data = b"checksum test".to_vec();
        let encoded = coder.encode(&data).unwrap();

        for shard in &encoded {
            let expected = sha256(&shard.data);
            assert_eq!(shard.checksum, expected);
        }
    }

    #[test]
    fn test_empty_data() {
        let coder = default_coder();
        let result = coder.encode(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_reconstruct_single_shard() {
        let coder = default_coder();
        let data = b"reconstruct missing shard".to_vec();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let original_shard = shards[2].clone().unwrap();
        shards[2] = None;

        let recovered = coder.reconstruct(&shards).unwrap().unwrap();
        assert_eq!(recovered.index, original_shard.index);
        assert_eq!(recovered.data, original_shard.data);
    }
}
