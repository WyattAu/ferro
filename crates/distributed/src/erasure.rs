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
            let parity_shard = shards.iter().find_map(|s| s.as_ref().filter(|s| s.is_parity)).ok_or(
                DistributedError::DecodingFailed {
                    reason: "parity shard not found despite parity_found flag".into(),
                },
            )?;

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

/// LRC-style layered erasure coder: global Reed-Solomon parity plus one
/// group-local XOR parity shard per data group.
///
/// Layout: indices `0..k` are data shards, `k..k+l` are local parity shards
/// (group `g` covers data `[g*gs, (g+1)*gs)`), `k+l..k+l+r` are global RS
/// parity over the data shards. A single data-shard loss is repaired from
/// its group (`k/l` fragments read) instead of the full RS stripe (`k`
/// fragments); all other losses fall back to the MDS global code.
pub struct LrcErasureCoder {
    config: ErasureConfig,
    groups: usize,
}

impl LrcErasureCoder {
    /// Create a layered coder with `groups` local parity shards.
    ///
    /// # Panics
    /// Panics if `data_shards + groups + parity_shards > 255` or
    /// `data_shards` is not divisible by `groups`.
    pub fn new(config: ErasureConfig, groups: usize) -> Self {
        assert!(groups >= 1, "at least one local group required");
        assert!(
            config.data_shards.is_multiple_of(groups),
            "data_shards must be divisible by groups"
        );
        assert!(
            config.data_shards + groups + config.parity_shards <= 255,
            "GF(2^8) field supports at most 255 total shards"
        );
        Self { config, groups }
    }

    pub fn group_size(&self) -> usize {
        self.config.data_shards / self.groups
    }

    pub fn local_parity_indices(&self) -> std::ops::Range<usize> {
        self.config.data_shards..self.config.data_shards + self.groups
    }

    pub fn global_parity_indices(&self) -> std::ops::Range<usize> {
        self.config.data_shards + self.groups..self.config.data_shards + self.groups + self.config.parity_shards
    }

    fn group_of(&self, idx: usize) -> usize {
        idx / self.group_size()
    }

    fn local_parity_of_group<'a>(&self, shards: &'a [Option<Shard>], group: usize) -> Option<&'a Shard> {
        shards.get(self.config.data_shards + group).and_then(|s| s.as_ref())
    }

    fn compute_local_parity(&self, group: usize, data_shard_bufs: &[Vec<u8>]) -> Vec<u8> {
        let gs = self.group_size();
        let mut parity = vec![0u8; data_shard_bufs[0].len()];
        for buf in &data_shard_bufs[(group * gs)..((group + 1) * gs)] {
            for (p, &b) in parity.iter_mut().zip(buf.iter()) {
                *p ^= b;
            }
        }
        parity
    }

    /// Decode data shards locally where possible, then fall back to RS.
    fn decode_with_path(&self, shards: &[Option<Shard>]) -> Result<(Vec<u8>, DecodePath), DistributedError> {
        let data_shards = self.config.data_shards;
        let total = data_shards + self.groups + self.config.parity_shards;
        if shards.len() != total {
            return Err(DistributedError::DecodingFailed {
                reason: format!("expected {total} shard slots, got {}", shards.len()),
            });
        }

        let mut bufs: Vec<Option<Vec<u8>>> = (0..data_shards).map(|_| None).collect();
        for (i, b) in bufs.iter_mut().enumerate() {
            if let Some(ref s) = shards[i]
                && !s.is_parity
            {
                *b = Some(s.data.clone());
            }
        }

        let missing: Vec<usize> = (0..data_shards).filter(|&i| bufs[i].is_none()).collect();

        if missing.len() > 1 {
            return self.decode_via_rs(shards, bufs);
        }

        if let Some(&idx) = missing.first() {
            let group = self.group_of(idx);
            let gs = self.group_size();
            let siblings_present = ((group * gs)..((group + 1) * gs))
                .filter(|&i| i != idx)
                .all(|i| bufs[i].is_some());
            let parity = self.local_parity_of_group(shards, group);
            if siblings_present {
                let Some(parity) = parity else {
                    return self.decode_via_rs(shards, bufs);
                };
                let mut recovered = parity.data.clone();
                for (i, buf) in bufs.iter().enumerate().skip(group * gs).take(gs) {
                    if i != idx
                        && let Some(d) = buf
                    {
                        for (r, &b) in recovered.iter_mut().zip(d.iter()) {
                            *r ^= b;
                        }
                    }
                }
                bufs[idx] = Some(recovered);
                let result = self.assemble(&bufs);
                return Ok((result, DecodePath::Local));
            }
            return self.decode_via_rs(shards, bufs);
        }

        Ok((self.assemble(&bufs), DecodePath::Direct))
    }

    fn decode_via_rs(
        &self,
        shards: &[Option<Shard>],
        mut bufs: Vec<Option<Vec<u8>>>,
    ) -> Result<(Vec<u8>, DecodePath), DistributedError> {
        let data_shards = self.config.data_shards;
        let k_plus_r = data_shards + self.config.parity_shards;

        let mut rs_slots: Vec<Option<Vec<u8>>> = (0..k_plus_r).map(|_| None).collect();
        for (i, b) in bufs.iter().enumerate() {
            rs_slots[i] = b.clone();
        }
        for (j, gi) in self.global_parity_indices().enumerate() {
            if let Some(ref s) = shards[gi] {
                rs_slots[data_shards + j] = Some(s.data.clone());
            }
        }

        let present = rs_slots.iter().filter(|s| s.is_some()).count();
        if present < data_shards {
            return Err(DistributedError::DecodingFailed {
                reason: format!("insufficient shards for RS decode: {present}/{k_plus_r} (need {data_shards})"),
            });
        }

        let rs = ReedSolomon::<Field>::new(data_shards, self.config.parity_shards).map_err(|e| {
            DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
            }
        })?;
        rs.reconstruct(&mut rs_slots)
            .map_err(|e| DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon reconstruct error: {e}"),
            })?;

        for (i, slot) in rs_slots.iter().take(data_shards).enumerate() {
            match slot {
                Some(d) => bufs[i] = Some(d.clone()),
                None => {
                    return Err(DistributedError::DecodingFailed {
                        reason: "data shard not reconstructed".to_string(),
                    });
                }
            }
        }

        Ok((self.assemble(&bufs), DecodePath::Global))
    }

    fn assemble(&self, bufs: &[Option<Vec<u8>>]) -> Vec<u8> {
        let mut result = Vec::new();
        for b in bufs.iter().take(self.config.data_shards) {
            if let Some(ref d) = *b {
                result.extend_from_slice(d);
            }
        }
        result
    }

    /// Verify every present shard against its checksum.
    ///
    /// Returns the indices of shards that are missing or corrupt.
    pub fn verify(&self, shards: &[Option<Shard>]) -> Vec<usize> {
        let mut bad = Vec::new();
        for (i, s) in shards.iter().enumerate() {
            match s {
                None => bad.push(i),
                Some(s) if s.checksum != sha256(&s.data) => bad.push(i),
                Some(_) => {}
            }
        }
        bad
    }

    /// Verify and repair a shard set in place.
    ///
    /// Each missing or checksum-corrupt shard is re-derived via the cheapest
    /// applicable path (`reconstruct`: local group parity first, then the
    /// global MDS code). Repaired indices are returned; shards that cannot be
    /// repaired within the code's loss budget are left `None`.
    pub fn scrub(&self, shards: &mut [Option<Shard>]) -> Result<Vec<usize>, DistributedError> {
        let bad = self.verify(shards);
        let mut repaired = Vec::new();
        for idx in bad {
            if shards[idx].is_some() {
                shards[idx] = None;
            }
            if let Some(rebuilt) = self.reconstruct(shards)?
                && rebuilt.index as usize == idx
            {
                shards[idx] = Some(rebuilt);
                repaired.push(idx);
            }
        }
        Ok(repaired)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodePath {
    Direct,
    Local,
    Global,
}

impl ErasureCoder for LrcErasureCoder {
    fn encode(&self, data: &[u8]) -> Result<Vec<Shard>, DistributedError> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        let data_shards = self.config.data_shards;
        let global_parity = self.config.parity_shards;
        let padded = pad_to_multiple(data, data_shards);
        let chunk_size = padded.len() / data_shards;
        if chunk_size == 0 {
            return Err(DistributedError::EncodingFailed {
                reason: "data too small for shard count".into(),
            });
        }

        let mut data_bufs: Vec<Vec<u8>> = (0..data_shards).map(|_| vec![0u8; chunk_size]).collect();
        for (i, buf) in data_bufs.iter_mut().enumerate() {
            let start = i * chunk_size;
            buf.copy_from_slice(&padded[start..start + chunk_size]);
        }

        let mut rs_bufs: Vec<Vec<u8>> = data_bufs.clone();
        if global_parity > 0 {
            rs_bufs.resize(data_shards + global_parity, vec![0u8; chunk_size]);
            let rs = ReedSolomon::<Field>::new(data_shards, global_parity).map_err(|e| {
                DistributedError::EncodingFailed {
                    reason: format!("Reed-Solomon init error: {e}"),
                }
            })?;
            let mut slices: Vec<&mut [u8]> = rs_bufs.iter_mut().map(|s| s.as_mut_slice()).collect();
            rs.encode(&mut slices).map_err(|e| DistributedError::EncodingFailed {
                reason: format!("Reed-Solomon encode error: {e}"),
            })?;
        }

        let mut result = Vec::with_capacity(data_shards + self.groups + global_parity);
        for (i, buf) in data_bufs.iter().enumerate() {
            result.push(Shard {
                index: i as u8,
                checksum: sha256(buf),
                is_parity: false,
                data: buf.clone(),
            });
        }
        for g in 0..self.groups {
            let parity = self.compute_local_parity(g, &data_bufs);
            result.push(Shard {
                index: (data_shards + g) as u8,
                checksum: sha256(&parity),
                is_parity: true,
                data: parity,
            });
        }
        for j in 0..global_parity {
            let parity_buf = rs_bufs[data_shards + j].clone();
            result.push(Shard {
                index: (data_shards + self.groups + j) as u8,
                checksum: sha256(&parity_buf),
                is_parity: true,
                data: parity_buf,
            });
        }
        Ok(result)
    }

    fn decode(&self, shards: &[Option<Shard>]) -> Result<Vec<u8>, DistributedError> {
        self.decode_with_path(shards).map(|(data, _)| data)
    }

    fn reconstruct(&self, shards: &[Option<Shard>]) -> Result<Option<Shard>, DistributedError> {
        let total = self.config.data_shards + self.groups + self.config.parity_shards;
        if shards.len() != total {
            return Ok(None);
        }

        let target = shards.iter().position(|s| s.is_none());
        let Some(target_idx) = target else {
            return Ok(None);
        };

        let data_shards = self.config.data_shards;
        if target_idx < data_shards {
            let group = self.group_of(target_idx);
            let gs = self.group_size();
            let siblings_present = ((group * gs)..((group + 1) * gs))
                .filter(|&i| i != target_idx)
                .all(|i| shards[i].is_some());
            if siblings_present && let Some(parity) = self.local_parity_of_group(shards, group) {
                let mut recovered = parity.data.clone();
                for (i, slot) in shards.iter().enumerate().skip(group * gs).take(gs) {
                    if i != target_idx
                        && let Some(s) = slot
                    {
                        for (r, &b) in recovered.iter_mut().zip(s.data.iter()) {
                            *r ^= b;
                        }
                    }
                }
                return Ok(Some(Shard {
                    index: target_idx as u8,
                    checksum: sha256(&recovered),
                    is_parity: false,
                    data: recovered,
                }));
            }
        }

        let mut slots: Vec<Option<Shard>> = Vec::with_capacity(shards.len());
        for (i, s) in shards.iter().enumerate() {
            if i == target_idx {
                slots.push(None);
            } else {
                slots.push(s.clone());
            }
        }
        let data: Vec<Option<&Shard>> = slots.iter().take(data_shards).map(|s| s.as_ref()).collect();
        let mut rs_slots: Vec<Option<Vec<u8>>> = (0..data_shards + self.config.parity_shards).map(|_| None).collect();
        for (i, d) in data.iter().enumerate() {
            if let Some(s) = d {
                rs_slots[i] = Some(s.data.clone());
            }
        }
        for (j, gi) in self.global_parity_indices().enumerate() {
            if let Some(ref s) = shards[gi] {
                rs_slots[data_shards + j] = Some(s.data.clone());
            }
        }
        if target_idx >= data_shards + self.groups {
            let j = target_idx - (data_shards + self.groups);
            rs_slots[data_shards + j] = None;
        }

        let present = rs_slots.iter().filter(|s| s.is_some()).count();
        if present < data_shards {
            return Ok(None);
        }

        let rs = ReedSolomon::<Field>::new(data_shards, self.config.parity_shards).map_err(|e| {
            DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
            }
        })?;
        rs.reconstruct(&mut rs_slots)
            .map_err(|e| DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon reconstruct error: {e}"),
            })?;

        let slot_index = if target_idx >= data_shards + self.groups {
            data_shards + (target_idx - data_shards - self.groups)
        } else {
            target_idx
        };
        match &rs_slots[slot_index] {
            Some(data) => Ok(Some(Shard {
                index: target_idx as u8,
                checksum: sha256(data),
                is_parity: target_idx >= data_shards,
                data: data.clone(),
            })),
            None => Ok(None),
        }
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
        let rs =
            ReedSolomon::<Field>::new(data_shards, parity_shards).map_err(|e| DistributedError::EncodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
            })?;

        let mut shards_mut: Vec<&mut [u8]> = shards_buf.iter_mut().map(|s| s.as_mut_slice()).collect();
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

        let rs =
            ReedSolomon::<Field>::new(data_shards, parity_shards).map_err(|e| DistributedError::DecodingFailed {
                reason: format!("Reed-Solomon init error: {e}"),
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
            expected
                [original.index as usize * (expected.len() / 4)..(original.index as usize + 1) * (expected.len() / 4)]
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

    // === LRC (layered: global RS + group-local XOR parity) tests ===

    fn lrc_coder() -> LrcErasureCoder {
        LrcErasureCoder::new(
            ErasureConfig {
                data_shards: 4,
                parity_shards: 2,
                shard_size: 1024,
            },
            2,
        )
    }

    fn lrc_coder_8_2_2() -> LrcErasureCoder {
        LrcErasureCoder::new(
            ErasureConfig {
                data_shards: 8,
                parity_shards: 2,
                shard_size: 1024,
            },
            2,
        )
    }

    #[test]
    fn test_lrc_encode_layout() {
        let coder = lrc_coder();
        let data = b"hello layered erasure world!!!!".to_vec();
        let encoded = coder.encode(&data).unwrap();
        assert_eq!(encoded.len(), 4 + 2 + 2);
        for (i, s) in encoded.iter().enumerate() {
            assert_eq!(s.index as usize, i);
            assert_eq!(s.is_parity, i >= 4);
        }
        assert_eq!(coder.group_size(), 2);
        assert_eq!(coder.local_parity_indices(), 4..6);
        assert_eq!(coder.global_parity_indices(), 6..8);
    }

    #[test]
    fn test_lrc_roundtrip() {
        let coder = lrc_coder();
        let mut data = b"round trip through layered parity!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();
        let shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        let (decoded, path) = coder.decode_with_path(&shards).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(path, DecodePath::Direct);
    }

    #[test]
    fn test_lrc_single_loss_uses_local_path() {
        let coder = lrc_coder();
        let mut data = b"local repair beats full stripe!!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[1] = None;
        let (decoded, path) = coder.decode_with_path(&shards).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(path, DecodePath::Local);

        shards[3] = None;
        shards[1] = Some(encoded[1].clone());
        let (decoded2, path2) = coder.decode_with_path(&shards).unwrap();
        assert_eq!(decoded2, data);
        assert_eq!(path2, DecodePath::Local);
    }

    #[test]
    fn test_lrc_two_losses_same_group_global_path() {
        let coder = lrc_coder();
        let mut data = b"two losses in one group need RS!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[1] = None;
        let (decoded, path) = coder.decode_with_path(&shards).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(path, DecodePath::Global);
    }

    #[test]
    fn test_lrc_data_and_local_loss_global_path() {
        let coder = lrc_coder();
        let mut data = b"one data one local parity gone!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[2] = None;
        shards[5] = None;
        let (decoded, path) = coder.decode_with_path(&shards).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(path, DecodePath::Global);
    }

    #[test]
    fn test_lrc_three_data_losses_fail() {
        let coder = lrc_coder();
        let data = b"three data losses exceed budget!".to_vec();
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[1] = None;
        shards[2] = None;
        assert!(coder.decode(&shards).is_err());
    }

    #[test]
    fn test_lrc_reconstruct_local_beats_global() {
        let coder = lrc_coder_8_2_2();
        let data = vec![7u8; 8 * 512];
        let encoded = coder.encode(&data).unwrap();

        let original = encoded[3].clone();
        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[3] = None;

        let recovered = coder.reconstruct(&shards).unwrap().unwrap();
        assert_eq!(recovered.index, 3);
        assert_eq!(recovered.data, original.data);
        assert!(!recovered.is_parity);
    }

    #[test]
    fn test_lrc_reconstruct_global_parity_shard() {
        let coder = lrc_coder();
        let mut data = b"rebuild a global parity shard!!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let original = encoded[6].clone();
        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[6] = None;

        let recovered = coder.reconstruct(&shards).unwrap().unwrap();
        assert_eq!(recovered.index, 6);
        assert_eq!(recovered.data, original.data);
        assert!(recovered.is_parity);
    }

    #[test]
    fn test_lrc_checksums() {
        let coder = lrc_coder();
        let mut data = b"every shard carries a checksum!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();
        for shard in &encoded {
            assert_eq!(shard.checksum, sha256(&shard.data));
        }
    }

    #[test]
    fn test_lrc_verify_detects_corruption() {
        let coder = lrc_coder();
        let data = vec![9u8; 4 * 256];
        let encoded = coder.encode(&data).unwrap();
        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[2].as_mut().unwrap().data[0] ^= 0xff;

        assert_eq!(coder.verify(&shards), vec![2]);
    }

    #[test]
    fn test_lrc_scrub_repairs_single_corrupt_shard() {
        let coder = lrc_coder();
        let mut data = b"scrub restores the broken shard!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[1].as_mut().unwrap().data[0] ^= 0xff;

        let repaired = coder.scrub(&mut shards).unwrap();
        assert_eq!(repaired, vec![1]);
        assert_eq!(shards[1].as_ref().unwrap().data, encoded[1].data);
        assert!(coder.verify(&shards).is_empty());
    }

    #[test]
    fn test_lrc_scrub_repairs_missing_shard() {
        let coder = lrc_coder();
        let mut data = b"missing shards come back from p!!".to_vec();
        data.resize(data.len() + (4 - data.len() % 4) % 4, 0);
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[3] = None;

        let repaired = coder.scrub(&mut shards).unwrap();
        assert_eq!(repaired, vec![3]);
        assert_eq!(shards[3].as_ref().unwrap().data, encoded[3].data);
    }

    #[test]
    fn test_lrc_scrub_beyond_budget_leaves_hole() {
        let coder = lrc_coder();
        let data = vec![3u8; 4 * 256];
        let encoded = coder.encode(&data).unwrap();

        let mut shards: Vec<Option<Shard>> = encoded.iter().map(|s| Some(s.clone())).collect();
        shards[0] = None;
        shards[1] = None;
        shards[2] = None;

        let repaired = coder.scrub(&mut shards).unwrap();
        assert!(repaired.is_empty());
        assert!(shards[0].is_none());
        assert_eq!(coder.verify(&shards), vec![0, 1, 2]);
    }

    #[test]
    fn test_lrc_rejects_uneven_groups() {
        let result = std::panic::catch_unwind(|| {
            LrcErasureCoder::new(
                ErasureConfig {
                    data_shards: 5,
                    parity_shards: 2,
                    shard_size: 1024,
                },
                2,
            )
        });
        assert!(result.is_err());
    }
}
