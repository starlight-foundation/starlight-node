use crate::{
    blocks::{Amount, Slot},
    keys::Public,
};

use super::Batch;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Account {
    pub(crate) batch: Batch,
    pub(crate) latest_balance: Amount,
    pub(crate) finalized_balance: Amount,
    pub(crate) nonce: u64,
    pub(crate) rep: Public,
}

impl Account {
    pub(crate) fn new(moving_balance: Amount) -> Self {
        Self {
            latest_balance: moving_balance,
            finalized_balance: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::burn(),
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
            batch: Batch::null(),
            nonce: 0,
            rep: Public::burn(),
        }
    }

    fn null() -> Self {
        Self {
            latest_balance: Amount::from_raw(u64::MAX - 1),
            finalized_balance: Amount::zero(),
            batch: Batch::null(),
            nonce: 0,
            rep: Public::burn(),
        }
    }
}
