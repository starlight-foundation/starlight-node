use super::Amount;

#[repr(C)]
pub struct Task {
    pub nonce: u64,
    pub from_index: u64,
    pub amount: Amount,
    pub to_index: u64
}

impl Task {
    pub fn is_change_representative(&self) -> bool {
        self.amount == Amount::zero()
    }
}

