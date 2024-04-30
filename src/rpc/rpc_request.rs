use bincode::{Decode, Encode};

use crate::keys::{Difficulty, Hash, Public};

#[derive(Encode, Decode)]
pub enum RpcRequest {
    AccountBalance(Public),
    WorkGenerate(Hash, Option<Difficulty>)
}