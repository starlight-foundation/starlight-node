use std::sync::Arc;

use bincode::{Decode, Encode};

use crate::{network::{Endpoint, ShredNote, TelemetryNote}, protocol::{Open, Slot, Transaction, Verified}, rpc::{RpcRequest, RpcResponse}, static_assert};

use super::Handle;

#[derive(Encode, Decode)]
pub enum Message {
    // Leader mode messages
    StartLeaderMode,
    EndLeaderMode,
    NewLeaderSlot(Slot),

    // Transaction messages
    Transaction(Box<Transaction>),
    TransactionList(Box<(Slot, Vec<Box<Verified<Transaction>>>)>),

    // Shred note messages
    ShredNote(Box<ShredNote>),

    // Telemetry messages
    TelemetryNote(Box<TelemetryNote>),
    TelemetryInterval,

    // Open messages
    Open(Box<Open>),
    OpenList(Box<(Slot, Vec<Box<Verified<Open>>>)>),
    
    // RPC
    RpcRequest(Box<(Handle, u64, RpcRequest)>),
    RpcResponse(Box<(u64, RpcResponse)>),

    // Broadcast
    Broadcast(Box<(Arc<Vec<Endpoint>>, Vec<u8>)>),

    // Tick
    Tick,

    // Process messages
    Shutdown
}

static_assert!(std::mem::size_of::<Message>() == 16);