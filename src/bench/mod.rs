use std::time::{Duration, Instant};
use rand::seq::SliceRandom;

use crate::{keys::Public, storage::Database};

use super::keys::Hash;
use std::hint::black_box;

fn hashing() {
    // Benchmark hashing of 1M-byte chunks
    let iterations = 5_000;
    let data_1m = vec![0u8; 1024 * 1024];
    let start_1m = Instant::now();
    for _ in 0..iterations {
        black_box(Hash::digest(&data_1m));
    }
    let elapsed_1m = start_1m.elapsed();
    let avg_time_1m = elapsed_1m / iterations;

    // Report results to stdout
    println!("Hashing 1M-byte chunks:");
    println!("  Total time: {:?}", elapsed_1m);
    println!("  Average time per iteration: {:?}", avg_time_1m);
    println!();

    // Benchmark hashing of 64-byte chunks
    let iterations = 10_000_000;
    let data_64 = [0u8; 64];
    let start_64 = Instant::now();
    for _ in 0..iterations {
        black_box(Hash::digest(&data_64));
    }
    let elapsed_64 = start_64.elapsed();
    let avg_time_64 = elapsed_64 / iterations;

    // Report results to stdout
    println!("Hashing 64-byte chunks:");
    println!("  Total time: {:?}", elapsed_64);
    println!("  Average time per iteration: {:?}", avg_time_64);
}

fn database() {
    let dir = "./test_directory";
    let db: Database<Public, u64> = Database::open(dir).unwrap();
    let key_count = 10_000_000;
    let mut keys: Vec<Public> = (0..key_count)
        .map(|_| Public::random())
        .collect();
    for key in keys.iter() {
        db.put(key, &rand::random());
    }
    keys.shuffle(&mut rand::thread_rng());
    let start = Instant::now();
    for key in keys {
        black_box(db.get(&key));
    }
    let elapsed = start.elapsed();
    let avg = elapsed / key_count;
    println!("Database read performance:");
    println!("  Total time: {:?}", elapsed);
    println!("  Average time per read: {:?}", avg);
    println!("  Reads per second: {}", Duration::from_secs(1).as_nanos() / avg.as_nanos());
    std::fs::remove_dir_all(dir).unwrap();
}

pub fn start() {
    println!("Starting Starlight benchmark suite");
    println!();
    hashing();
    database();
}