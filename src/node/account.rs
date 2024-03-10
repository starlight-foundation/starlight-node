#[derive(Clone, Copy, Debug)]
pub(crate) struct Account {
    pub(crate) balance: u64,
    pub(crate) nonce: u64,
    pub(crate) slot: u64
}

impl Account {
    pub(crate) fn new(balance: u64) -> Self {
        Self {
            balance,
            nonce: 0,
            slot: 0
        }
    }
}

impl leapfrog::Value for Account {
    fn is_redirect(&self) -> bool {
        self.slot == u64::MAX
    }

    fn is_null(&self) -> bool {
        self.slot == u64::MAX - 1
    }

    fn redirect() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            slot: u64::MAX
        }
    }

    fn null() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            slot: u64::MAX - 1
        }
    }
}