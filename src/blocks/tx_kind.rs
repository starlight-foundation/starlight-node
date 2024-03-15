#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u64)]
pub enum TxKind {
    Normal = 0,
    ChangeRepresentative = 1,
}
impl TxKind {
    pub fn to_bytes(self) -> [u8; 8] {
        (self as u64).to_le_bytes()
    }
}
