use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bincode::{Decode, Encode};
use bitvec::vec::BitVec;
use reed_solomon_erasure::{
    galois_8::{Field, ReedSolomon},
    ReconstructShard,
};

use crate::{
    keys::HashBuilder,
    util::{UninitBitVec, UninitVec},
};

// Maps number of data shreds to the optimal erasure batch size which has the
// same recovery probabilities as a 32:32 erasure batch.
const DATA_TO_TOTAL: [usize; 33] = [
    0, 18, 20, 22, 23, 25, 27, 28, 30, // 8
    32, 33, 35, 36, 38, 39, 41, 42, // 16
    43, 45, 46, 48, 49, 51, 52, 53, // 24
    55, 56, 58, 59, 60, 62, 63, 64, // 32
];

const DATA_SHREDS_PER_FULL_BATCH: usize = 32;
const TOTAL_SHREDS_PER_FULL_BATCH: usize = DATA_TO_TOTAL[DATA_SHREDS_PER_FULL_BATCH];

struct ReedSolomonCache {
    cache: Mutex<HashMap<(usize, usize), Arc<ReedSolomon>>>,
}

impl ReedSolomonCache {
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
    fn get(&self, data_shreds: usize, parity_shreds: usize) -> Arc<ReedSolomon> {
        self.cache
            .lock()
            .unwrap()
            .entry((data_shreds, parity_shreds))
            .or_insert_with(|| {
                let rs = ReedSolomon::new(data_shreds, parity_shreds).unwrap();
                Arc::new(rs)
            })
            .clone()
    }
}

#[static_init::dynamic]
static REED_SOLOMON_CACHE: ReedSolomonCache = ReedSolomonCache::new();

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

#[derive(Encode, Decode, Clone, Default)]
pub struct Shred {
    // The total number of batches in the shredded data
    n_batches: u32,
    // The number of data shreds in each batch
    n_data_shreds: u32,
    // The overall size of the original data
    overall_data_size: u32,
    // The index of the batch this shred belongs to
    batch_index: u32,
    // The index of this shred within its batch
    shred_index: u32,
    // The actual data contained in this shred
    data: Vec<u8>,
}

impl Shred {
    // Hashes the shred data into the provided HashBuilder
    pub fn hash_into(&self, hb: &mut HashBuilder) {
        // Update the HashBuilder with the number of batches (in little-endian bytes)
        hb.update(&self.n_batches.to_le_bytes());
        // Update the HashBuilder with the number of data shreds (in little-endian bytes)
        hb.update(&self.n_data_shreds.to_le_bytes());
        // Update the HashBuilder with the overall data size (in little-endian bytes)
        hb.update(&self.overall_data_size.to_le_bytes());
        // Update the HashBuilder with the batch index (in little-endian bytes)
        hb.update(&self.batch_index.to_le_bytes());
        // Update the HashBuilder with the shred index (in little-endian bytes)
        hb.update(&self.shred_index.to_le_bytes());
        // Update the HashBuilder with the actual shred data
        hb.update(&self.data);
    }

    // Shreds the input data into multiple Shred instances
    pub fn shred(data: &[u8], chunk_len: u32) -> Vec<Self> {
        // If the input data is empty or the chunk length is zero, return an empty vector
        if data.len() == 0 || chunk_len == 0 {
            return Vec::new();
        }

        // If the chunk length is greater than the data length, set them equal to each other
        let chunk_len = std::cmp::min(chunk_len, data.len() as u32);

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
        let mut shreds = Vec::with_capacity(shred_count);

        // Initialize variables for batch index and start index
        let mut batch_index = 0;
        let mut start_index = 0;

        // Iterate over the shreds and process them in batches
        while start_index < shred_count {
            // Calculate the tentative end total index for the current batch
            let end_total_index_tentative = start_index + TOTAL_SHREDS_PER_FULL_BATCH;

            // Determine the number of total and data shreds for the current batch
            let (n_total, n_data) = if end_total_index_tentative > shred_count {
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
                    n_data_shreds: n_data as u32,
                    overall_data_size: data.len() as u32,
                    batch_index: batch_index as u32,
                    shred_index: (i - start_index) as u32,
                    data: {
                        let mut v = Vec::with_capacity(chunk_len as usize);
                        v.extend_from_slice(chunks.next().unwrap());
                        v.extend(std::iter::repeat(0).take(v.capacity() - v.len()));
                        v
                    },
                };
                shreds.push(shred);
            }

            // Populate the coding shreds with empty data
            for i in end_data_index..end_total_index {
                let shred = Self {
                    n_batches: n_batches as u32,
                    n_data_shreds: n_data as u32,
                    overall_data_size: data.len() as u32,
                    batch_index: batch_index as u32,
                    shred_index: (i - start_index) as u32,
                    // safety: not read by RSE
                    data: unsafe { Vec::uninit(chunk_len as usize) },
                };
                shreds.push(shred);
            }

            // Get the Reed-Solomon encoder from the cache based on the number of data and coding shreds
            let reed_solomon = REED_SOLOMON_CACHE.get(n_data, n_coding);

            // Encode the shreds using the Reed-Solomon encoder
            reed_solomon
                .encode(&mut shreds[start_index..end_total_index])
                .unwrap();

            // Update the start index and batch index for the next iteration
            start_index = end_total_index;
            batch_index += 1;
        }

        assert_eq!(shreds.len(), shred_count as usize);

        // Return the shredded data
        shreds
    }

    // Returns the batch index of this shred
    pub fn get_batch_index(&self) -> usize {
        self.batch_index as usize
    }

    // Returns the shred index of this shred within its batch
    pub fn get_shred_index(&self) -> usize {
        self.shred_index as usize
    }
}

struct BatchItem(Vec<u8>);

impl ReconstructShard<Field> for BatchItem {
    // Returns the length of the shard if it is not empty, otherwise returns None
    fn len(&self) -> Option<usize> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.len())
        }
    }

    // Returns a mutable reference to the shard data if it is not empty, otherwise returns None
    fn get(&mut self) -> Option<&mut [<Field as reed_solomon_erasure::Field>::Elem]> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.as_mut())
        }
    }

    // Returns a mutable reference to the shard data if it is not empty, otherwise initializes the shard with the given length and returns a mutable reference to it
    fn get_or_initialize(
        &mut self,
        len: usize,
    ) -> Result<
        &mut [<Field as reed_solomon_erasure::Field>::Elem],
        Result<&mut [<Field as reed_solomon_erasure::Field>::Elem], reed_solomon_erasure::Error>,
    > {
        if self.0.is_empty() {
            // safety: not read by RSE
            self.0 = unsafe { Vec::uninit(len) };
            Err(Ok(&mut self.0))
        } else {
            Ok(&mut self.0)
        }
    }
}

struct Batch {
    // Indicates whether the batch has been initialized
    initialized: bool,
    // The number of shreds provided to the batch
    n_provided: u32,
    // The number of data shreds in the batch
    n_data: u32,
    // The length of each shred in the batch
    chunk_len: u32,
    // The list of shreds in the batch
    shreds: Vec<BatchItem>,
}

impl Batch {
    // Creates a new empty batch
    pub fn new() -> Self {
        Self {
            initialized: false,
            n_provided: 0,
            n_data: u32::MAX,
            chunk_len: u32::MAX,
            shreds: Vec::new(),
        }
    }

    // Checks if the batch needs a specific shred
    pub fn need_shred(&self, shred_index: usize) -> bool {
        // If the batch is not initialized, always return true (shred is needed)
        if !self.initialized {
            return true;
        }

        // Check if the shred index is within the valid range
        match self.shreds.get(shred_index) {
            Some(shred) => shred.0.is_empty(), // Shred is needed if it is empty
            None => false,                     // Shred index is out of bounds, shred is not needed
        }
    }

    // Tries to provide a shred to the batch
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        // Extract the chunk length from the shred
        let chunk_len = shred.data.len() as u32;

        // If the batch is not initialized, initialize it with the shred's information
        if !self.initialized {
            self.n_data = shred.n_data_shreds;
            self.chunk_len = chunk_len;
            let n_total = DATA_TO_TOTAL[self.n_data as usize];
            self.shreds.reserve(n_total);
            self.shreds
                .extend((0..n_total).map(|_| BatchItem(Vec::new())));
            self.initialized = true;
        }

        // Check if the shred's chunk length matches the batch's chunk length
        if chunk_len != self.chunk_len {
            return false;
        }

        // Extract the shred index from the shred
        let shred_index = shred.shred_index as usize;

        // Check if the shred index is within the valid range
        if shred_index >= self.shreds.len() {
            return false;
        }

        // If the shred is not already provided, store it in the batch and increment the provided count
        if self.shreds[shred_index].0.is_empty() {
            self.shreds[shred_index] = BatchItem(shred.data);
            self.n_provided += 1;
            true
        } else {
            false
        }
    }

    // Returns the size of the data in the batch
    pub fn data_size(&self) -> usize {
        self.n_data as usize * self.chunk_len as usize
    }

    // Checks if the batch can be reconstructed
    pub fn can_reconstruct(&self) -> bool {
        self.initialized && self.n_provided >= self.n_data
    }

    // Tries to reconstruct the data in the batch and appends it to the provided output vector
    pub fn try_reconstruct(&mut self, out: &mut Vec<u8>) -> bool {
        // Check if the batch can be reconstructed
        if !self.can_reconstruct() {
            return false;
        }

        // Get the Reed-Solomon encoder from the cache based on the number of data and coding shreds
        let reed_solomon = REED_SOLOMON_CACHE.get(
            self.n_data as usize,
            DATA_TO_TOTAL[self.n_data as usize] - self.n_data as usize,
        );

        // Reconstruct the data shreds using the Reed-Solomon encoder
        reed_solomon.reconstruct_data(&mut self.shreds).unwrap();

        // Append the reconstructed data shreds to the output vector
        for shred in self.shreds[..self.n_data as usize].iter() {
            out.extend_from_slice(&shred.0);
        }

        true
    }
}

pub struct ShredList {
    // Indicates whether the ShredList has been initialized
    initialized: bool,
    // The maximum allowed size of the reconstructed data
    max_data_size: u32,
    // The claimed size of the original data
    claimed_data_size: u32,
    // A bitvector indicating which batches are ready for reconstruction
    ready: BitVec,
    // The list of batches containing the shreds
    batches: Vec<Batch>,
}

impl ShredList {
    // Creates a new ShredList with the specified maximum data size
    pub fn new(max_data_size: u32) -> Self {
        Self {
            initialized: false,
            max_data_size,
            claimed_data_size: 0,
            ready: BitVec::new(),
            batches: Vec::new(),
        }
    }

    // Tries to provide a shred to the ShredList
    pub fn try_provide(&mut self, shred: Shred) -> bool {
        // Extract relevant information from the shred
        let n_batches = shred.n_batches as usize;
        let n_data = shred.n_data_shreds as usize;
        let chunk_len = shred.data.len();
        let data_size_bound = n_batches * n_data * chunk_len;
        let claimed_data_size = shred.overall_data_size;

        // Check if the shred is valid and within the allowed data size bounds
        if data_size_bound > self.max_data_size as usize
            || (n_batches == 0 || n_data == 0 || chunk_len == 0)
        {
            return false;
        }

        // Initialize the ShredList if it hasn't been initialized yet
        if !self.initialized {
            self.batches.reserve(n_batches);
            self.batches.extend((0..n_batches).map(|_| Batch::new()));
            self.claimed_data_size = claimed_data_size;
            // safety: immediately zeroed
            self.ready = unsafe { BitVec::uninit(n_batches) };
            self.ready.as_raw_mut_slice().fill(0);
            self.initialized = true;
        }

        // Extract the batch index from the shred
        let batch_index = shred.batch_index as usize;

        // Check if the batch index is within the valid range
        if batch_index >= self.batches.len() {
            return false;
        }

        // Get a mutable reference to the corresponding batch
        let batch = &mut self.batches[batch_index];

        // Try to provide the shred to the batch
        if batch.try_provide(shred) {
            // If the batch can be reconstructed after providing the shred, mark it as ready
            if batch.can_reconstruct() {
                self.ready.set(batch_index, true);
            }
            true
        } else {
            false
        }
    }

    // Checks if the ShredList has enough shreds to reconstruct the original data
    pub fn can_reconstruct(&self) -> bool {
        // Check if the ShredList is initialized and if all batches are ready for reconstruction
        self.initialized && self.ready.iter_zeros().next().is_none()
    }

    // Tries to reconstruct the original data from the provided shreds
    pub fn try_reconstruct(&mut self) -> Option<Vec<u8>> {
        // Check if the ShredList can be reconstructed
        if !self.can_reconstruct() {
            return None;
        }

        // If there are no batches, return an empty vector
        if self.batches.len() == 0 {
            return Some(Vec::new());
        }

        // Create a vector to store the reconstructed data
        let mut data = Vec::with_capacity(self.batches.len() * self.batches[0].data_size());

        // Iterate over the batches and reconstruct the data for each batch
        for batch in self.batches.iter_mut() {
            assert_eq!(batch.try_reconstruct(&mut data), true);
        }

        // Truncate the reconstructed data to the claimed data size
        data.truncate(self.claimed_data_size as usize);

        // Return the reconstructed data
        Some(data)
    }

    // Checks if a specific shred is needed for reconstruction
    pub fn need_shred(&self, batch_index: usize, shred_index: usize) -> bool {
        // If the ShredList is not initialized, always return true (shred is needed)
        if !self.initialized {
            return true;
        }

        // Check if the batch is already ready for reconstruction
        match self.ready.get(batch_index).map(|x| !!x) {
            Some(true) => return false, // Batch is ready, shred is not needed
            Some(false) => {}
            None => return false, // Batch index is out of bounds, shred is not needed
        }

        // Get a reference to the corresponding batch
        let batch = &self.batches[batch_index];

        // Check if the batch needs the specific shred
        if batch.need_shred(shred_index) {
            return true; // Shred is needed by the batch
        }

        false // Shred is not needed
    }
}

#[cfg(test)]
mod tests {
    use rand::RngCore;

    use super::*;

    const MAX_DATA_SIZE: u32 = 1024 * 1024 * 8; // 8 MB

    #[test]
    fn test_shred_and_reconstruct() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunk_len = 2;

        // Shred the data
        let shreds = Shred::shred(&data, chunk_len);

        // Create a ShredList and provide the shreds
        let mut shred_list = ShredList::new(MAX_DATA_SIZE);
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
        let mut shred_list = ShredList::new(MAX_DATA_SIZE);
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
        let mut shred_list = ShredList::new(MAX_DATA_SIZE);
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
        let chunk_len = 12;

        // Shred the large data
        let shreds = Shred::shred(&data, chunk_len);
        assert!(shreds.len() >= (data.len() / chunk_len as usize));

        // Create a ShredList and provide the shreds
        let mut shred_list = ShredList::new(MAX_DATA_SIZE);
        for shred in shreds {
            assert!(shred_list.try_provide(shred));
        }

        // Reconstruct the data
        let reconstructed_data = shred_list.try_reconstruct().unwrap();

        // Verify the reconstructed data matches the original data
        assert_eq!(data, reconstructed_data);
    }
}
