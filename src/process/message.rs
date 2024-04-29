use crate::{network::{ShredNote, TelemetryNote}, protocol::{Open, Slot, Transaction, Verified}, rpc::Command, static_assert};

#[derive(Clone)]
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
    Command(Box<Command>)
}

static_assert!(std::mem::size_of::<Message>() == 16);