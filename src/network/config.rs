use super::Version;

pub const VERSION: Version = Version::new(0, 1, 0);
pub const MTU: usize = 1280;
pub const PEER_UPDATE: u64 = 15;
pub const PEER_TIMEOUT: u64 = 2 * PEER_UPDATE;
pub const MAGIC_NUMBER: [u8; 8] = [0x3f, 0xd1, 0x0f, 0xe2, 0x5e, 0x76, 0xfa, 0xe6];
pub fn fanout(n: usize) -> usize {
    if n < 8 {
        n
    } else if n < 16 {
        n / 2
    } else if n < 32 {
        n / 3
    } else if n < 64 {
        n / 4
    } else {
        (n as f64).powf(0.58) as usize
    }
}
