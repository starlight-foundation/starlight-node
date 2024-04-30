use bincode::{Decode, Encode};

use crate::keys::Work;

#[derive(Encode, Decode)]
pub enum RpcResponse {
    AccountBalance(u64),
    WorkGenerate(Work)
}