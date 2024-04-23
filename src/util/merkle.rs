use crate::keys::Hash;

fn merkle_row_direct(hashes: &[Hash]) -> Vec<Hash> {
    assert_eq!(hashes.len() % 2, 0);
    let mut row = Vec::with_capacity(hashes.len() / 2 + ((hashes.len() / 2) % 2));
    let mut buf = [0u8; 64];
    for pair in hashes.chunks(2) {
        buf[0..32].copy_from_slice(&pair[0].as_bytes());
        buf[32..64].copy_from_slice(&pair[1].as_bytes());
        row.push(Hash::digest(&buf));
    }
    if row.len() < row.capacity() {
        row.push(Hash::zero());
    }
    row
}

pub fn merkle_root_direct(mut hashes: Vec<Hash>) -> Hash {
    if hashes.len() % 2 != 0 {
        hashes.push(Hash::zero());
    }
    let mut row = hashes;
    while row.len() > 1 {
        row = merkle_row_direct(&row);
    }
    row[0]
}

pub fn merkle_root<T, E, F: Fn(&T) -> Result<Hash, E>>(ts: &[T], f: F) -> Result<Hash, E> {
    match ts.len() {
        0 => Ok(Hash::zero()),
        1 => f(&ts[0]),
        n => {
            let mut tx_hashes = Vec::with_capacity(ts.len() + ts.len() % 2);
            for item in ts.iter() {
                tx_hashes.push(f(item)?);
            }
            if tx_hashes.len() < tx_hashes.capacity() {
                tx_hashes.push(Hash::zero());
            }
            Ok(merkle_root_direct(tx_hashes))
        }
    }
}
