use std::{sync::Arc, time::Duration};

use crate::{error, keys::Public, process::{Handle, Mailbox, Message, Process}, protocol::{Task, Tx, TxFull, TxHalf}, state::{Bank, Batch}, util::Error};

pub struct TxFiller {
    tx_half_list: Vec<Box<TxHalf>>,
    db: Handle,
    state: Handle
}

impl TxFiller {
    pub fn new(tx_half_list: Vec<Box<TxHalf>>, db: Handle, state: Handle) -> Self {
        Self { tx_half_list, db, state }
    }
}

impl Process for TxFiller {
    const NAME: &'static str = "TxExecutor";
    const RESTART_ON_CRASH: bool = false;

    fn run(&mut self, mut mailbox: Mailbox, handle: Handle) -> Result<(), Error> {
        let requests: Vec<Public> = self.tx_half_list.iter().map(|x| {
            [x.tx.from, x.tx.to]
        }).flatten().collect();
        let requests_len = requests.len();
        self.db.send(Message::BatchedRetrieveRequest(Box::new((handle.clone(), requests))));
        let responses = loop {
            match mailbox.recv_timeout(Duration::from_millis(500)).ok_or(
                error!("database didn't respond, this executor isn't going to bother!")
            )? {
                Message::BatchedRetrieveResponse(v) => break v,
                _ => continue
            }
        };
        if responses.len() != requests_len {
            return Err(error!(
                "db response len ({}) != db request len ({})",
                responses.len(),
                requests_len
            ));
        }
        let tx_full_list: Vec<Box<TxFull>> = responses
            .chunks_exact(2)
            .filter_map(|x| Some((x[0]?, x[1]?)))
            .zip(self.tx_half_list.drain(..))
            .map(|((from_index, to_index), tx_half)| {
                tx_half.provide(from_index, to_index)
            })
            .collect();
        self.state.send(Message::TxFullList(Box::new(tx_full_list)));
        Ok(())
    }
}