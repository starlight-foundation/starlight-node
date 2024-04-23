use criterion::{black_box, criterion_group, criterion_main, Criterion};
use starlight_node::keys::Hash;

fn hash_1m_byte_chunks(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];
    c.bench_function("hash 1M-byte chunks", |b| {
        b.iter(|| {
            black_box(Hash::digest(&data))
        })
    });
}

fn hash_64_byte_chunks(c: &mut Criterion) {
    let data = [0u8; 64];
    c.bench_function("hash 64-byte chunks", |b| {
        b.iter(|| {
            black_box(Hash::digest(&data))
        })
    });
}

criterion_group!(benches, hash_1m_byte_chunks, hash_64_byte_chunks);
criterion_main!(benches);