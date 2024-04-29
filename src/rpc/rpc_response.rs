use crate::keys::Work;

pub enum RpcResponse {
    AccountBalance(u64),
    WorkGenerate(Work)
}