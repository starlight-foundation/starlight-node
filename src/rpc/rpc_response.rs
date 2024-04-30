use bincode::Encode;

use crate::keys::Work;

#[derive(Encode)]
pub enum RpcResponse {
    AccountBalance(u64),
    WorkGenerate(Work)
}