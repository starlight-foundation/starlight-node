use std::{sync::Arc, time::Duration};

use crate::{error, keys::Public, process::{Handle, Mailbox, Message, Process}, protocol::{Task, Transaction, Verified}, state::{Bank, Batch}, util::Error};

pub struct TxExecutor {
    transactions: Vec<Verified<Transaction>>,
    db: Handle,
    state: Handle,
    bank: Arc<Bank>,
    batch: Batch
}

impl TxExecutor {
    pub fn new(transactions: Vec<Verified<Transaction>>, db: Handle, state: Handle, bank: Arc<Bank>, batch: Batch) -> Self {
        Self { transactions, db, state, bank, batch }
    }
}

impl Process for TxExecutor {
    const NAME: &'static str = "TxExecutor";
    const RESTART_ON_CRASH: bool = false;

    fn run(&mut self, mut mailbox: Mailbox, handle: Handle) -> Result<(), Error> {
        let requests: Vec<Public> = self.transactions.iter().map(|x| {
            [x.val.from, x.val.to]
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
        let tasks: Vec<Task> = responses
            .chunks_exact(2)
            .filter_map(|x| Some((x[0]?, x[1]?)))
            .zip(self.transactions.iter())
            .map(|((from_index, to_index), tx)| Task {
                nonce: tx.val.nonce,
                from_index,
                amount: tx.val.amount,
                to_index
            })
            .filter(|task| self.bank.queue_task(&task, self.batch).is_ok())
            .collect();
        Ok(())
    }
}