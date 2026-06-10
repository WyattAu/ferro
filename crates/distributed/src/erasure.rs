use crate::error::DistributedError;
use reed_solomon_erasure::ReedSolomon;
use reed_solomon_erasure::galois_8::Field;
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
                    && !s.is_parity
                {
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
                .ok_or(DistributedError::DecodingFailed {
                    reason: "parity shard not found despite parity_found flag".into(),
                })?;

            let mut data_shards_map: Vec<(u8, &Shard)> = Vec::new();
            for s in shards {
                if let Some(ref s) = *s
                    && !s.is_parity
                {
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

        let parity = parity_opt.ok_or(DistributedError::DecodingFailed {
            reason: "parity shard not found".into(),
        })?;
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
                && !s.is_parity
            {
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

/// Reed-Solomon erasure coder using GF(2^8) Vandermonde matrix.
///
/// Supports recovery from up to `parity_shards` lost shards.
/// Data is split into `data_shards` equal-sized blocks, then `parity_shards`
/// parity blocks are computed using Reed-Solomon encoding over GF(2^8).
pub struct ReedSolomonErasureCoder {
    config: ErasureConfig,
}

impl ReedSolomonErasureCoder {
    /// Create a new Reed-Solomon erasure coder.
    ///
    /// # Panics
    /// Panics if `data_shards + parity_shards > 255` (GF(2^8) field limit).
    pub fn new(config: ErasureConfig) -> Self {
        assert!(
            config.data_shards + config.parity_shards <= 255,
            "GF(2^8) field supports at most 255 total shards"
        );
        Self { config }
    }
}

impl ErasureCoder for ReedSolomonErasureCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<Shard>, DistributedError> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let data_shards = self.config.data_shards;
        let parity_shards = self.config.parity_shards;
        let total = data_shards + parity_shards;

        let padded = pad_to_multiple(data, data_shards);
        let chunk_size = padded.len() / data_shards;
        if chunk_size == 0 {
            return Err(DistributedError::EncodingFailed {
                reason: "data too small for shard count".into(),
            });
        }

        // Build flat shard buffer: data_shards rows of chunk_size bytes
        let mut shards_buf: Vec<Vec<u8>> = (0..total).map(|_| vec![0u8; chunk_size]).collect();

        // Copy data into first data_shards rows
        for (i, shard) in shards_buf.iter_mut().enumerate().take(data_shards) {
            let start = i * chunk_size;
            shard.copy_from_slice(&padded[start..start + chunk_size]);
        }

        // Encode parity shards in-place
        let rs = ReedSolomon::<Field>::new(data_shards, parity_shards).map_err(|e| {
            DistributedError::EncodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
            }
        })?;

        let mut shards_mut: Vec<&mut [u8]> =
            shards_buf.iter_mut().map(|s| s.as_mut_slice()).collect();
        rs.encode(&mut shards_mut)
            .map_err(|e| DistributedError::EncodingFailed {
                reason: format!("Reed-Solomon encode error: {e}"),
            })?;

        // Build Shard structs
        let mut result = Vec::with_capacity(total);
        for (i, shard) in shards_buf.into_iter().enumerate() {
            let checksum = sha256(&shard);
            result.push(Shard {
                index: i as u8,
                data: shard,
                is_parity: i >= data_shards,
                checksum,
            });
        }

        Ok(result)
    }

    fn decode(&self, shards: &[Option<Shard>]) -> Result<Vec<u8>, DistributedError> {
        let data_shards = self.config.data_shards;
        let parity_shards = self.config.parity_shards;
        let total = data_shards + parity_shards;

        if shards.len() < data_shards {
            return Err(DistributedError::DecodingFailed {
                reason: format!("too few shards provided: {}", shards.len()),
            });
        }

        // Build Option<Vec<u8>> array for reconstruct
        let mut shards_opt: Vec<Option<Vec<u8>>> = (0..total).map(|_| None).collect();
        let mut missing_count = 0usize;
        for (i, s) in shards.iter().enumerate() {
            if let Some(ref s) = *s {
                shards_opt[i] = Some(s.data.clone());
            } else {
                missing_count += 1;
            }
        }

        // Only run reconstruct if there are missing shards
        if missing_count > 0 {
            let rs = ReedSolomon::<Field>::new(data_shards, parity_shards).map_err(|e| {
                DistributedError::DecodingFailed {
                    reason: format!("Reed-Solomon init error: {e}"),
                }
            })?;

            rs.reconstruct(&mut shards_opt)
                .map_err(|e| DistributedError::DecodingFailed {
                    reason: format!("Reed-Solomon reconstruct error: {e}"),
                })?;
        }

        // Reassemble data from first data_shards rows
        let mut result = Vec::new();
        for shard in shards_opt.iter().take(data_shards) {
            match shard {
                Some(data) => result.extend_from_slice(data),
                None => {
                    return Err(DistributedError::DecodingFailed {
                        reason: "data shard not reconstructed".to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    fn reconstruct(&self, shards: &[Option<Shard>]) -> Result<Option<Shard>, DistributedError> {
        let data_shards = self.config.data_shards;
        let parity_shards = self.config.parity_shards;
        let total = data_shards + parity_shards;

        let mut shards_opt: Vec<Option<Vec<u8>>> = (0..total).map(|_| None).collect();
        for (i, s) in shards.iter().enumerate() {
            if let Some(ref s) = *s {
                shards_opt[i] = Some(s.data.clone());
            }
        }

        // Find a missing index to reconstruct
        let missing_idx = shards_opt.iter().position(|s| s.is_none());

        let Some(target_idx) = missing_idx else {
            return Ok(None); // No missing shards
        };

        let rs = ReedSolomon::<Field>::new(data_shards, parity_shards).map_err(|e| {
            DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
            }
        })?;

        rs.reconstruct(&mut shards_opt)
            .map_err(|e| DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon reconstruct error: {e}"),
            })?;

        match &shards_opt[target_idx] {
            Some(data) => Ok(Some(Shard {
                index: target_idx as u8,
                data: data.clone(),
                is_parity: target_idx >= data_shards,
                checksum: sha256(data),
            })),
            None => Ok(None),
        }
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

    fn rs_coder() -> ReedSolomonErasureCoder {
        ReedSolomonErasureCoder::new(ErasureConfig {
            data_shards: 4,
            parity_shards: 2,
            shard_size: 1024,
        })
    }

    fn rs_coder_6_3() -> ReedSolomonErasureCoder {
        ReedSolomonErasureCoder::new(ErasureConfig {
            data_shards: 6,
            parity_shards: 3,
            shard_size: 4096,
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

    // === Reed-Solomon tests ===

    #[test]
    fn test_rs_encode_roundtrip() {
        let coder = rs_coder();
        let mut data = b"hello reed-solomon erasure world!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();
        assert_eq!(encoded.len(), 6); // 4 data + 2 parity

        let shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_rs_single_shard_loss() {
        let coder = rs_coder();
        let mut data = b"recover one shard with Reed-Solomon!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0); // pad to multiple of 4
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[1] = None; // lose a data shard

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_rs_two_shard_loss() {
        let coder = rs_coder();
        let mut data = b"recover TWO shards with Reed-Solomon!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[1] = None; // lose data shard
        shards[2] = None; // lose another data shard

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_rs_parity_shard_loss() {
        let coder = rs_coder();
        let mut data = b"losing parity is fine asdf".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[4] = None; // lose parity shard
        shards[5] = None; // lose other parity shard

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_rs_mixed_loss() {
        let coder = rs_coder();
        let mut data = b"mixed data and parity loss1234".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None; // lose data
        shards[5] = None; // lose parity

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_rs_three_shard_loss_fails() {
        let coder = rs_coder();
        let data = b"three shards lost".to_vec();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[1] = None;
        shards[2] = None;

        let result = coder.decode(&shards);
        assert!(result.is_err());
    }

    #[test]
    fn test_rs_reconstruct_missing_data_shard() {
        let coder = rs_coder();
        let mut data = b"reconstruct data shard!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let expected = data.clone();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let original = shards[2].clone().unwrap();
        shards[2] = None;

        let recovered = coder.reconstruct(&shards).unwrap().unwrap();
        assert_eq!(recovered.index, original.index);
        assert_eq!(
            recovered.data,
            expected[original.index as usize * (expected.len() / 4)
                ..(original.index as usize + 1) * (expected.len() / 4)]
        );
        assert!(!recovered.is_parity);
    }

    #[test]
    fn test_rs_reconstruct_missing_parity_shard() {
        let coder = rs_coder();
        let mut data = b"reconstruct parity shard!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let original = shards[5].clone().unwrap();
        shards[5] = None;

        let recovered = coder.reconstruct(&shards).unwrap().unwrap();
        assert_eq!(recovered.index, original.index);
        assert_eq!(recovered.data, original.data);
        assert!(recovered.is_parity);
    }

    #[test]
    fn test_rs_checksums() {
        let coder = rs_coder();
        let mut data = b"checksum verification for RS".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        for shard in &encoded {
            assert_eq!(shard.checksum, sha256(&shard.data));
        }
    }

    #[test]
    fn test_rs_empty_data() {
        let coder = rs_coder();
        let result = coder.encode(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_rs_6_3_roundtrip() {
        let coder = rs_coder_6_3();
        let data = vec![0u8; 6000]; // 6 * 1000 = 6000, fits evenly
        let encoded = coder.encode(&data).unwrap();
        assert_eq!(encoded.len(), 9); // 6 data + 3 parity

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[1] = None;
        shards[3] = None;
        shards[7] = None;

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_rs_large_data() {
        let coder = rs_coder();
        let data = vec![42u8; 1024 * 1024]; // 1 MB
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[4] = None;

        let decoded = coder.decode(&shards).unwrap();
        assert_eq!(decoded.len(), 1024 * 1024);
        assert!(decoded.iter().all(|&b| b == 42));
    }

    #[test]
    fn test_rs_config_limits() {
        // 254 total shards should work
        let coder = ReedSolomonErasureCoder::new(ErasureConfig {
            data_shards: 252,
            parity_shards: 3,
            shard_size: 1024,
        });
        let small_data = vec![1u8; 252];
        let encoded = coder.encode(&small_data).unwrap();
        assert_eq!(encoded.len(), 255);
    }
}
