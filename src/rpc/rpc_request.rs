use bincode::Decode;

use crate::keys::{Difficulty, Hash, Public};

#[derive(Decode)]
pub enum RpcRequest {
    AccountBalance(Public),
    WorkGenerate(Hash, Option<Difficulty>)
}