use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use itertools::Itertools;

use once_cell::sync::Lazy;
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};

use crate::blocks::Slot;

struct ReedSolomonCache {
    cache: Mutex<HashMap<(usize, usize), Arc<ReedSolomon>>>,
}

impl ReedSolomonCache {
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
    fn get(
        &self,
        data_shards: usize,
        parity_shards: usize,
    ) -> Arc<ReedSolomon> {
        let key = (data_shards, parity_shards);
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&key) {
                return entry.clone();
            }
        }
        let entry = ReedSolomon::new(data_shards, parity_shards).unwrap();
        let entry = Arc::new(entry);
        {
            let entry = entry.clone();
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key, entry);
        }
        entry
    }
}

static REED_SOLOMON_CACHE: Lazy<ReedSolomonCache> = Lazy::new(|| ReedSolomonCache::new());

const DATA_SHREDS_PER_BATCH: usize = 32;

// Maps number of data shreds to the optimal erasure batch size which has the
// same recovery probabilities as a 32:32 erasure batch.
const ERASURE_BATCH_LEN: [usize; 33] = [
    0, 18, 20, 22, 23, 25, 27, 28, 30, // 8
    32, 33, 35, 36, 38, 39, 41, 42, // 16
    43, 45, 46, 48, 49, 51, 52, 53, // 24
    55, 56, 58, 59, 60, 62, 63, 64, // 32
];

#[derive(Serialize, Deserialize, Clone)]
pub struct Shred {
    slot: Slot,
    n_batches: u32,
    batch_index: u32,
    shred_index: u32,
    data: Box<[u8]>,
}

impl AsRef<[u8]> for Shred {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for Shred {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Shred {
    pub fn shred(data: &[u8], shred_size: usize, slot: Slot) -> Vec<Self> {
        let data_shred_count = data.len().div_ceil(shred_size);
        let batch_count = data_shred_count.div_ceil(DATA_SHREDS_PER_BATCH);
        let shred_count = {
            let normal_batch_count = data_shred_count / DATA_SHREDS_PER_BATCH;
            let shreds_per_normal_batch = ERASURE_BATCH_LEN[DATA_SHREDS_PER_BATCH];
            let last_batch_data_shred_count = data_shred_count % DATA_SHREDS_PER_BATCH;
            let shreds_per_last_batch = ERASURE_BATCH_LEN[last_batch_data_shred_count];
            (normal_batch_count * shreds_per_normal_batch) + shreds_per_last_batch
        };
        let mut shreds = Vec::with_capacity(shred_count);
        let chunks = data
            .chunks(shred_size)
            .map(|chunk| chunk.to_vec().into_boxed_slice());
        let batches = chunks.chunks(DATA_SHREDS_PER_BATCH);
        for (batch_index, batch) in batches.into_iter().enumerate() {
            let start_index = shreds.len();
            let mut data_count = 0;
            for (shred_index, data) in batch.into_iter().enumerate() {
                let shred = Self {
                    slot,
                    n_batches: batch_count as u32,
                    batch_index: batch_index as u32,
                    shred_index: shred_index as u32,
                    data,
                };
                shreds.push(shred);
                data_count += 1;
            }
            let total_count = ERASURE_BATCH_LEN[data_count];
            let parity_count = total_count - data_count;
            unsafe {
                shreds.set_len(shreds.len() + parity_count);
            }
            let reed_solomon = REED_SOLOMON_CACHE.get(data_count, parity_count);
            reed_solomon.encode(&mut shreds[start_index..]).unwrap();
        }
        shreds
    }
}