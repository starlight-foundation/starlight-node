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

// Maps number of data shreds to the optimal erasure batch size which has the
// same recovery probabilities as a 32:32 erasure batch.
const ERASURE_BATCH_LEN: [usize; 33] = [
    0, 18, 20, 22, 23, 25, 27, 28, 30, // 8
    32, 33, 35, 36, 38, 39, 41, 42, // 16
    43, 45, 46, 48, 49, 51, 52, 53, // 24
    55, 56, 58, 59, 60, 62, 63, 64, // 32
];

#[derive(Clone, Copy)]
struct BatchMeta {
    n_data: usize,
    n_coding: usize,
    n_total: usize
}
impl BatchMeta {
    const fn from_n_data(n_data: usize) -> Self {
        Self {
            n_data,
            n_coding: ERASURE_BATCH_LEN[n_data] - n_data,
            n_total: ERASURE_BATCH_LEN[n_data]
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Shred {
    slot: Slot,
    n_batches: u32,
    n_data: u32,
    batch_index: u32,
    shred_index: u32,
    data: Vec<u8>,
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

const DATA_SHREDS_PER_FULL_BATCH: usize = 32;
impl Shred {
    pub fn null() -> Self {
        Self {
            slot: Slot::max(),
            n_batches: 0,
            shred_size: 0,
            batch_index: 0,
            shred_index: 0,
            data: Vec::new(),
        }
    }
    pub fn is_null(&self) -> bool {
        self.slot == Slot::max()
    }
    pub fn shred(data: &[u8], shred_size: u32, slot: Slot) -> Vec<Self> {
        let n_data_shreds = data.len().div_ceil(shred_size as usize);
        let n_batches = n_data_shreds.div_ceil(DATA_SHREDS_PER_FULL_BATCH);
        let shred_count = {
            let n_full_batches = n_data_shreds / DATA_SHREDS_PER_FULL_BATCH;
            let n_shreds_per_normal_batch = BatchMeta::new(
                DATA_SHREDS_PER_FULL_BATCH
            ).n_total();
            let n_shreds_for_last_batch = BatchMeta::new(
                n_data_shreds % DATA_SHREDS_PER_FULL_BATCH
            ).n_total();
            (n_full_batches * n_shreds_per_normal_batch) + n_shreds_for_last_batch
        };
        let mut shreds = Vec::with_capacity(shred_count);
        let chunks = data
            .chunks(shred_size as usize)
            .map(|chunk| chunk.to_vec());
        let batches = chunks.chunks(DATA_SHREDS_PER_FULL_BATCH);
        for (batch_index, batch) in batches.into_iter().enumerate() {
            let start_index = shreds.len();
            let mut data_count = 0;
            for (shred_index, data) in batch.into_iter().enumerate() {
                let shred = Self {
                    slot,
                    n_batches: n_batches as u32,
                    n_data: n_data_shreds as u32,
                    batch_index: batch_index as u32,
                    shred_index: shred_index as u32,
                    data,
                };
                shreds.push(shred);
                data_count += 1;
            }
            let batch_meta = BatchMeta::new(data_count);
            unsafe {
                shreds.set_len(shreds.len() + batch_meta.n_coding());
            }
            let reed_solomon = REED_SOLOMON_CACHE.get(
                batch_meta.n_data, batch_meta.n_coding()
            );
            reed_solomon.encode(&mut shreds[start_index..]).unwrap();
        }
        shreds
    }
    pub fn get_slot(&self) -> Slot {
        self.slot
    }
}

struct Batch {
    n_provided: usize,
    shreds: Vec<Shred>
}
impl Batch {
    pub fn new(n_batches: u32) -> Self {
        let mut shreds = Vec::with_capacity(n_batches as usize);
        shreds.extend((0..n_batches).map(|_| Shred::null()));
        Self {
            n_provided: 0,
            shreds
        }
    }
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        let batch_index = shred.batch_index as usize;
        if batch_index >= self.shreds.len() {
            return false;
        }
        if self.shreds[batch_index].is_null() {
            self.shreds[batch_index] = shred;
            self.n_provided += 1;
            true
        } else {
            false
        }
    }
    pub fn try_reconstruct(&self) -> Option<Vec<u8>> {
        let n_data = self.shreds.iter().filter(|shred| !shred.is_null()).count();
        if n_data < BatchMeta::new(self.shreds.len()).n_data {
            return None;
        }
        let mut shreds = self.shreds.iter().filter(|shred| !shred.is_null()).collect::<Vec<_>>();
        let reed_solomon = REED_SOLOMON_CACHE.get(n_data, shreds.len() - n_data);
        reed_solomon.reconstruct(&mut shreds).unwrap();
        let data = shreds.iter().map(|shred| shred.data.clone()).flatten().collect();
        Some(data)
    }
}

pub struct ShredList {
    batches: Vec<Batch>
}

