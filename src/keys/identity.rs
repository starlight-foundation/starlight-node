use super::{Private, Public};

#[derive(Clone, Copy)]
pub struct Identity {
    pub public: Public,
    pub private: Private
}