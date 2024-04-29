use crate::keys::{Difficulty, Hash, Public};

pub enum RpcRequest {
    AccountBalance(Public),
    WorkGenerate(Hash, Option<Difficulty>)
}