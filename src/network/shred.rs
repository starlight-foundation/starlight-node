use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;
use reed_solomon_erasure::{galois_8::{Field, ReedSolomon}, ReconstructShard};
use serde::{Deserialize, Serialize};

use crate::util::UninitializedVec;

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
const DATA_TO_TOTAL: [usize; 33] = [
    0, 18, 20, 22, 23, 25, 27, 28, 30, // 8
    32, 33, 35, 36, 38, 39, 41, 42, // 16
    43, 45, 46, 48, 49, 51, 52, 53, // 24
    55, 56, 58, 59, 60, 62, 63, 64, // 32
];

#[derive(Serialize, Deserialize, Clone)]
pub struct Shred {
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
const TOTAL_SHREDS_PER_FULL_BATCH: usize = DATA_TO_TOTAL[DATA_SHREDS_PER_FULL_BATCH];

impl Shred {
    pub fn shred(data: &[u8], shred_size: u32) -> Vec<Self> {
        // Calculate the number of data shreds based on the data length and shred size
        let n_data_shreds = data.len().div_ceil(shred_size as usize);
        
        // Calculate the number of batches based on the number of data shreds
        let n_batches = n_data_shreds.div_ceil(DATA_SHREDS_PER_FULL_BATCH);
        
        // Calculate the number of data shreds for the last batch
        let n_data_shreds_for_last_batch = n_data_shreds % DATA_SHREDS_PER_FULL_BATCH;
        
        // Calculate the total number of shreds for the last batch
        let n_total_shreds_for_last_batch = DATA_TO_TOTAL[n_data_shreds_for_last_batch];
        
        // Calculate the total number of shreds
        let shred_count = {
            let n_full_batches = n_data_shreds / DATA_SHREDS_PER_FULL_BATCH;
            let n_shreds_for_last_batch = DATA_TO_TOTAL[n_data_shreds_for_last_batch];
            (n_full_batches * TOTAL_SHREDS_PER_FULL_BATCH) + n_shreds_for_last_batch
        };
        
        // Create an iterator to chunk the data into shreds
        let mut chunks = data.chunks(shred_size as usize);
        
        // Create a vector to store the shreds with the calculated length
        let mut shreds = Vec::uninitialized(shred_count);
        
        // Initialize variables for batch index and start index
        let mut batch_index = 0;
        let mut start_index = 0;
        
        // Iterate over the shreds and process them in batches
        while start_index < shreds.len() {
            // Calculate the tentative end total index for the current batch
            let end_total_index_tentative = start_index + TOTAL_SHREDS_PER_FULL_BATCH;
            
            // Determine the number of total and data shreds for the current batch
            let (n_total, n_data) = if end_total_index_tentative > shreds.len() {
                (n_total_shreds_for_last_batch, n_data_shreds_for_last_batch)
            } else {
                (TOTAL_SHREDS_PER_FULL_BATCH, DATA_SHREDS_PER_FULL_BATCH)
            };

            // Calculate the number of coding shreds for the current batch
            let n_coding = n_total - n_data;
            
            // Calculate the end total index and end data index for the current batch
            let end_total_index = start_index + n_total;
            let end_data_index = start_index + n_data;
            
            // Populate the data shreds with the actual data
            for i in start_index..end_data_index {
                let shred = Self {
                    n_batches: n_batches as u32,
                    n_data: n_data as u32,
                    batch_index: batch_index as u32,
                    shred_index: (i - start_index) as u32,
                    data: chunks.next().unwrap().to_vec(),
                };
                shreds[i] = shred;
            }
            
            // Populate the coding shreds with empty data
            for i in end_data_index..end_total_index {
                let shred = Self {
                    n_batches: n_batches as u32,
                    n_data: n_data as u32,
                    batch_index: batch_index as u32,
                    shred_index: (i - start_index) as u32,
                    data: Vec::uninitialized(shred_size as usize)
                };
                shreds[i] = shred;
            }
            
            // Get the Reed-Solomon encoder from the cache based on the number of data and coding shreds
            let reed_solomon = REED_SOLOMON_CACHE.get(
                n_data, n_coding
            );
            
            // Encode the shreds using the Reed-Solomon encoder
            reed_solomon.encode(&mut shreds[start_index..end_total_index]).unwrap();
            
            // Update the start index and batch index for the next iteration
            start_index = end_total_index;
            batch_index += 1;
        }
        
        // Return the shredded data
        shreds
    }
}

struct BatchItem(Vec<u8>);

impl ReconstructShard<Field> for BatchItem {
    fn len(&self) -> Option<usize> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.len())
        }
    }

    fn get(&mut self) -> Option<&mut [<Field as reed_solomon_erasure::Field>::Elem]> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.as_mut())
        }
    }

    fn get_or_initialize(
        &mut self,
        len: usize,
    ) -> Result<&mut [<Field as reed_solomon_erasure::Field>::Elem], Result<&mut [<Field as reed_solomon_erasure::Field>::Elem], reed_solomon_erasure::Error>> {
        if self.0.is_empty() {
            self.0 = Vec::uninitialized(len);
        }
        Ok(&mut self.0)
    }
}

struct Batch {
    n_provided: usize,
    n_data: usize,
    shreds: Vec<BatchItem>
}
impl Batch {
    pub fn new() -> Self {
        let shreds = Vec::new();
        Self {
            n_provided: 0,
            n_data: usize::MAX,
            shreds
        }
    }
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        if shred.n_batches > 
        if self.n_data == usize::MAX {
            self.n_data = shred.n_data as usize;
            let n_total = DATA_TO_TOTAL[self.n_data];
            self.shreds.reserve(n_total);
            self.shreds.extend((0..n_total).map(|_| BatchItem(Vec::new())));
        }
        let shred_index = shred.shred_index as usize;
        if shred_index >= self.shreds.len() {
            return false;
        }
        if self.shreds[shred_index].0.is_empty() {
            self.shreds[shred_index] = BatchItem(shred.data);
            self.n_provided += 1;
            true
        } else {
            false
        }
    }
    pub fn try_reconstruct(&self) -> Option<Vec<u8>> {
        if self.n_provided < self.n_data {
            return None;
        }
        let reed_solomon = REED_SOLOMON_CACHE.get(
            self.n_data, DATA_TO_TOTAL[self.n_data] - self.n_data
        );
        reed_solomon.reconstruct_data(&mut self.shreds).unwrap();
        
        let data = self.shreds.iter().map(|shred| shred.clone()).flatten().collect();
        Some(data)
    }
}



pub struct ShredList {
    batches: Vec<Batch>
}

