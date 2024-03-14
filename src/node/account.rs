use crate::{blocks::{Amount, Slot}, keys::Public};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Account {
    pub(crate) moving_balance: Amount,
    pub(crate) finalized_balance: Amount,
    pub(crate) slot: Slot,
    pub(crate) nonce: u64,
    pub(crate) rep: Public
}

impl Account {
    pub(crate) fn new(moving_balance: Amount) -> Self {
        Self {
            moving_balance,
            finalized_balance: Amount::zero(),
            slot: Slot::genesis(),
            nonce: 0,
            rep: Public::burn()
        }
    }
}

impl leapfrog::Value for Account {
    fn is_redirect(&self) -> bool {
        self.moving_balance == Amount::from_raw(u64::MAX)
    }

    fn is_null(&self) -> bool {
        self.moving_balance == Amount::from_raw(u64::MAX - 1)
    }

    fn redirect() -> Self {
        Self {
            moving_balance: Amount::from_raw(u64::MAX),
            finalized_balance: Amount::zero(),
            slot: Slot::genesis(),
            nonce: 0,
            rep: Public::burn()
        }
    }

    fn null() -> Self {
        Self {
            moving_balance: Amount::from_raw(u64::MAX - 1),
            finalized_balance: Amount::zero(),
            slot: Slot::genesis(),
            nonce: 0,
            rep: Public::burn()
        }
    }
}