use std::sync::Arc;

use bincode::{Decode, Encode};

use crate::{keys::Public, network::{Endpoint, ShredNote, TelemetryNote}, protocol::{Open, OpenFull, Slot, Tx, TxEmpty, TxFull}, rpc::{RpcRequest, RpcResponse}, static_assert};

use super::Handle;

#[derive(Encode, Decode)]
pub enum Message {
    // Leader mode messages
    StartLeaderMode,
    EndLeaderMode,
    NewLeaderSlot(Slot),

    // Transaction messages
    TxEmpty(Box<TxEmpty>),
    TxFullList(Box<Vec<Box<TxFull>>>),

    // Shred note messages
    ShredNote(Box<ShredNote>),

    // Telemetry messages
    TelemetryNote(Box<TelemetryNote>),
    TelemetryInterval,

    // Open messages
    Open(Box<Open>),
    OpenList(Box<(Slot, Vec<Box<OpenFull>>)>),
    
    // RPC
    RpcRequest(Box<(Handle, u64, RpcRequest)>),
    RpcResponse(Box<(u64, RpcResponse)>),

    // Broadcast
    Broadcast(Box<(Arc<Vec<Endpoint>>, Vec<u8>)>),

    // Directory
    BatchedRetrieveRequest(Box<(Handle, Vec<Public>)>),
    BatchedRetrieveResponse(Box<Vec<Option<u64>>>),
    BatchedTryInsertRequest(Box<(Handle, Vec<(Public, u64)>)>),
    BatchedTryInsertResponse(Box<Vec<bool>>),

    // Interval
    Tick
}

static_assert!(std::mem::size_of::<Message>() == 16);