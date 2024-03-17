use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bitvec::vec::BitVec;
use once_cell::sync::Lazy;
use reed_solomon_erasure::{galois_8::{Field, ReedSolomon}, ReconstructShard};
use serde::{Deserialize, Serialize};

use crate::util::{UninitializedBitVec, UninitializedVec};

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

#[derive(Serialize, Deserialize, Clone, Default)]
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
    pub fn shred(data: &[u8], chunk_len: u32) -> Vec<Self> {
        if data.len() == 0 || chunk_len == 0 {
            return Vec::new();
        }

        // Calculate the number of data shreds based on the data length and chunk length
        let n_data_shreds = data.len().div_ceil(chunk_len as usize);
        
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
        let mut chunks = data.chunks(chunk_len as usize);
        
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
                    data: Vec::uninitialized(chunk_len as usize)
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
            Err(Ok(&mut self.0))
        } else {
            Ok(&mut self.0)
        }
    }
}

struct Batch {
    initialized: bool,
    n_provided: u32,
    n_data: u32,
    chunk_len: u32,
    shreds: Vec<BatchItem>
}
impl Batch {
    pub fn new() -> Self {
        Self {
            initialized: false,
            n_provided: 0,
            n_data: u32::MAX,
            chunk_len: u32::MAX,
            shreds: Vec::new()
        }
    }
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        let chunk_len = shred.data.len() as u32;
        if !self.initialized {
            self.n_data = shred.n_data;
            self.chunk_len = chunk_len;
            let n_total = DATA_TO_TOTAL[self.n_data as usize];
            self.shreds.reserve(n_total);
            self.shreds.extend((0..n_total).map(|_| BatchItem(Vec::new())));
            self.initialized = true;
        }
        if chunk_len != self.chunk_len {
            return false;
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
    pub fn data_size(&self) -> usize {
        self.n_data as usize * self.chunk_len as usize
    }
    pub fn can_reconstruct(&self) -> bool {
        self.initialized && self.n_provided >= self.n_data
    }
    pub fn try_reconstruct(&mut self, out: &mut Vec<u8>) -> bool {
        if !self.can_reconstruct() {
            return false;
        }
        let reed_solomon = REED_SOLOMON_CACHE.get(
            self.n_data as usize, 
            DATA_TO_TOTAL[self.n_data as usize] - self.n_data as usize
        );
        reed_solomon.reconstruct_data(&mut self.shreds).unwrap();
        for shred in self.shreds[..self.n_data as usize].iter() {
            out.extend_from_slice(&shred.0);
        }
        true
    }
}

pub struct ShredList {
    initialized: bool,
    max_data_size: u32,
    ready: BitVec,
    batches: Vec<Batch>
}

impl ShredList {
    pub fn new(max_data_size: u32) -> Self {
        let batches = Vec::new();
        Self {
            initialized: false,
            max_data_size,
            ready: BitVec::new(),
            batches
        }
    }
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        let n_batches = shred.n_batches as usize;
        let n_data = shred.n_data as usize;
        let chunk_len = shred.data.len();
        let data_size = n_batches * n_data * chunk_len;
        if (n_batches == 0 || n_data == 0 || chunk_len == 0)
        || data_size > self.max_data_size as usize {
            return false;
        }
        if !self.initialized {
            self.batches.reserve(n_batches);
            self.batches.extend((0..n_batches).map(|_| Batch::new()));
            self.ready = BitVec::uninitialized(n_batches);
            self.ready.as_raw_mut_slice().fill(0);
            self.initialized = true;
        }
        let batch_index = shred.batch_index as usize;
        if batch_index >= self.batches.len() {
            return false;
        }
        let batch = &mut self.batches[batch_index];
        if batch.try_provide(shred) {
            if batch.can_reconstruct() {
                self.ready.set(batch_index, true);
            }
            true
        } else {
            false
        }
    }
    pub fn can_reconstruct(&self) -> bool {
        self.initialized && self.ready.iter_zeros().next().is_none()
    }
    pub fn try_reconstruct(&mut self) -> Option<Vec<u8>> {
        if !self.can_reconstruct() {
            return None;
        }
        if self.batches.len() == 0 {
            return Some(Vec::new());
        }
        let mut data = Vec::with_capacity(
            self.batches.len() * self.batches[0].data_size()
        );
        for batch in self.batches.iter_mut() {
            assert_eq!(batch.try_reconstruct(&mut data), true);
        }
        Some(data)
    }
}

#[cfg(test)]
mod tests {
    use rand::RngCore;

    use super::*;

    #[test]
    fn test_shred_and_reconstruct() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunk_len = 2;

        // Shred the data
        let shreds = Shred::shred(&data, chunk_len);
        
        // Create a ShredList and provide the shreds
        let mut shred_list = ShredList::new(data.len() as u32);
        for shred in shreds {
            assert!(shred_list.try_provide(shred));
        }

        // Reconstruct the data
        let reconstructed_data = shred_list.try_reconstruct().unwrap();

        // Verify the reconstructed data matches the original data
        assert_eq!(data, reconstructed_data);
    }

    #[test]
    fn test_shred_and_reconstruct_with_missing_shreds() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunk_len = 2;

        // Shred the data
        let mut shreds = Shred::shred(&data, chunk_len);
        assert_eq!(shreds.len(), DATA_TO_TOTAL[5]);
        
        // Remove some shreds to simulate missing shreds
        shreds.remove(2);
        shreds.remove(3);
        shreds.remove(9);
        shreds.remove(10);
        shreds.remove(11);

        // Create a ShredList and provide the remaining shreds
        let mut shred_list = ShredList::new(data.len() as u32);
        for shred in shreds {
            assert!(shred_list.try_provide(shred));
        }

        // Reconstruct the data
        let reconstructed_data = shred_list.try_reconstruct().unwrap();

        // Verify the reconstructed data matches the original data
        assert_eq!(data, reconstructed_data);
    }

    #[test]
    fn test_shred_and_reconstruct_with_insufficient_shreds() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunk_len = 2;

        // Shred the data
        let mut shreds = Shred::shred(&data, chunk_len);

        // Remove too many shreds to make reconstruction impossible
        shreds.truncate(2);

        // Create a ShredList and provide the remaining shreds
        let mut shred_list = ShredList::new(data.len() as u32);
        for shred in shreds {
            assert!(shred_list.try_provide(shred));
        }

        // Try to reconstruct the data
        let reconstructed_data = shred_list.try_reconstruct();

        // Verify that reconstruction fails due to insufficient shreds
        assert!(reconstructed_data.is_none());
    }

    #[test]
    fn test_shred_and_reconstruct_with_large_data() {
        let data = {
            let mut v = vec![0; 1024 * 1024]; // 1 MB of data
            rand::thread_rng().fill_bytes(&mut v);
            v
        };
        let chunk_len = 1024;

        // Shred the large data
        let shreds = Shred::shred(&data, chunk_len);

        // Create a ShredList and provide the shreds
        let mut shred_list = ShredList::new(data.len() as u32);
        for shred in shreds {
            assert!(shred_list.try_provide(shred));
        }

        // Reconstruct the data
        let reconstructed_data = shred_list.try_reconstruct().unwrap();

        // Verify the reconstructed data matches the original data
        assert_eq!(data, reconstructed_data);
    }
}