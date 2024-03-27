use crate::{
    protocol::{Amount, Slot},
    keys::Public,
};

use super::Batch;

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub batch: Batch,
    pub latest_balance: Amount,
    pub finalized_balance: Amount,
    pub weight: Amount,
    pub nonce: u64,
    pub rep: Public,
}

impl Account {
    pub fn with_weight(weight: Amount) -> Self {
        Self {
            weight,
            latest_balance: Amount::zero(),
            finalized_balance: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::zero(),
        }
    }
    pub fn with_latest_balance(latest_balance: Amount) -> Self {
        Self {
            latest_balance,
            finalized_balance: Amount::zero(),
            weight: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::zero(),
        }
    }
    pub fn genesis(public: Public) -> Self {
        Self {
            latest_balance: Amount::initial_supply(),
            finalized_balance: Amount::initial_supply(),
            weight: Amount::initial_supply(),
            batch: Batch::null(),
            nonce: 0,
            rep: public,
        }
    }
}

impl leapfrog::Value for Account {
    fn is_redirect(&self) -> bool {
        self.latest_balance == Amount::from_raw(u64::MAX)
    }

    fn is_null(&self) -> bool {
        self.latest_balance == Amount::from_raw(u64::MAX - 1)
    }

    fn redirect() -> Self {
        Self {
            latest_balance: Amount::from_raw(u64::MAX),
            finalized_balance: Amount::zero(),
            weight: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::zero(),
        }
    }

    fn null() -> Self {
        Self {
            latest_balance: Amount::from_raw(u64::MAX - 1),
            finalized_balance: Amount::zero(),
            weight: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::zero(),
        }
    }
}
