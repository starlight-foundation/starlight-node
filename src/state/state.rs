use std::{sync::{Arc, Mutex}, time::Duration};

//use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{keys::Public, storage::Database};

use crate::{error, keys::{Hash, Identity, Private}, process::{self, Handle, Mailbox, Message, Process}, protocol::{Amount, Open, OpenFull, Slot, Task, Tx, TxFull}, util::Error};

use super::{Bank, Block, Dag};

struct BlockEntry {
    block: Arc<Block>,
    tasks: Vec<Task>
}

pub struct State {
    /// My identity
    id: Identity,
    /// Are we in leader mode?
    leader_mode: bool,
    /// The database of the longest chain
    db: Database<Public, u64>,
    /// The account state of the longest chain
    bank: Arc<Bank>,
    /// All finalized blocks
    finalized: Vec<Arc<Block>>,
    /// Last finalized block (root) plus all blocks that are not yet finalized
    active: Dag<Hash, Box<BlockEntry>>,
    /// The slot of the block-in-construction
    cur_slot: Option<Slot>,
    /// The transactions of the block-in-construction
    cur_txs: Option<Vec<Box<TxFull>>>,
    /// The opens of the block-in-construction
    cur_opens: Option<Vec<Box<OpenFull>>>
}

impl State {
    pub fn new(identity: Identity, data_dir: &str, genesis_block: Arc<Block>) -> Result<Self, Error> {
        if !genesis_block.is_genesis() {
            return Err(error!("invalid genesis block"));
        }
        Ok(Self {
            id: identity,
            leader_mode: false,
            active: Dag::new(genesis_block.hash, Box::new(BlockEntry {
                block: genesis_block.clone(),
                tasks: vec![],
            })),
            bank: Arc::new(Bank::open(data_dir, genesis_block.leader)?),
            finalized: vec![genesis_block],
            cur_slot: None,
            cur_txs: None,
            cur_opens: None
        })
    }
    /*pub fn finalize_hash(&mut self, h: Hash) -> Result<(), Error> {
        // find the common ancestor of the longest chain and the block to finalize
        let (&longest_chain, _) = self.active.get_longest_chain();
        let common_ancestor = *self.active.get_common_ancestor(h, longest_chain).unwrap().0;
        // is the block to finalize on the longest chain?
        if common_ancestor == h {
            // good :) just finalize all the blocks up to and including that one and we're done!
            let path_from_root_to_block =
                self.active.get_path(*self.active.get_root().0, h).unwrap();
            for h in path_from_root_to_block.iter().skip(1) {
                let block = self.active.get(h).unwrap();
                self.bank.finalize_block(block);
                self.finalized.push(block.clone());
            }
            self.active.set_root(h).unwrap();
            return Ok(());
        }
        // it's not! ok then:
        // a) revert all the blocks up to our common ancestor
        let path_from_common_ancestor_to_longest_chain = self
            .active
            .get_path(common_ancestor, longest_chain)
            .unwrap();
        for h in path_from_common_ancestor_to_longest_chain
            .iter()
            .skip(1)
            .rev()
        {
            self.bank.revert_block(self.active.get(h).unwrap());
        }
        // b) finalize all the blocks from the one after the last finalized (root) to our block,
        //    processing them first if they haven't been yet
        let path_from_root_to_block = self.active.get_path(*self.active.get_root().0, h).unwrap();
        // we've processed all blocks from the root to the common ancestor
        let mut passed_common_ancestor = false;
        for h in path_from_root_to_block.iter().skip(1) {
            let block = self.active.get(h).unwrap();
            if passed_common_ancestor {
                // oh well, if the network finalizes a wrong block, might as well panic lol
                self.bank.process_block(block).unwrap();
            }
            self.bank.finalize_block(block);
            self.finalized.push(block.clone());
            if h == &common_ancestor {
                // start processing the new blocks!
                passed_common_ancestor = true;
            }
        }
        // c) set the new root
        self.active.set_root(h).unwrap();
        // d) process the blocks on the new longest chain
        let (&new_longest_chain, _) = self.active.get_longest_chain();
        let path_from_common_ancestor_to_new_longest_chain = self
            .active
            .get_path(common_ancestor, new_longest_chain)
            .unwrap();
        for h in path_from_common_ancestor_to_new_longest_chain
            .iter()
            .skip(1)
        {
            self.bank
                .process_block(self.active.get(h).unwrap())
                .unwrap();
        }
        Ok(())
    }
    pub fn add_block(&mut self, block: Arc<Block>, tasks: Vec<Task>) -> Result<(), Error> {
        let previous = block.previous;
        let hash = block.hash;
        let (&prev_longest_chain, _) = self.active.get_longest_chain();
        let building_on_longest_chain = prev_longest_chain == previous;
        self.active.insert(hash, Box::new(BlockEntry {
            block: block.clone(),
            tasks: tasks
        }), previous)?;
        let added_block_is_longest_chain = self.active.get_longest_chain().0 == &hash;
        if building_on_longest_chain {
            // easy! we're building on a new longest chain -- just process the block and we're done!
            self.bank.process_block(&block).unwrap();
            return Ok(());
        }
        if !added_block_is_longest_chain {
            // adding our block didn't make it the head of the longest chain.
            // so we're totally good :) no processing necessary
            return Ok(());
        }
        // the addition of our block made it the head of the longest chain!
        // 1) revert to common ancestor
        let (&common_ancestor, _) = self
            .active
            .get_common_ancestor(prev_longest_chain, previous)
            .unwrap();
        let path_from_ancestor_to_prev_longest_chain = self
            .active
            .get_path(common_ancestor, prev_longest_chain)
            .unwrap();
        for h in path_from_ancestor_to_prev_longest_chain
            .iter()
            .skip(1)
            .rev()
        {
            self.bank.revert_block(self.active.get(h).unwrap());
        }
        // 2) apply all blocks from block after common ancestor to new longest chain
        let path_from_ancestor_to_new_longest_chain =
            self.active.get_path(common_ancestor, hash).unwrap();
        match (|| -> Result<(), (usize, &Hash)> {
            for (i, h) in path_from_ancestor_to_new_longest_chain
                .iter()
                .enumerate()
                .skip(1)
            {
                self.bank
                    .process_block(self.active.get(h).unwrap())
                    .map_err(|_| (i, h))?;
            }
            Ok(())
        })() {
            Ok(()) => {
                // good, everything went OK
            }
            Err((invalid_block_index, invalid_block)) => {
                // oh, one of the blocks in our longest chain was actually invalid
                // a) remove it and all its descendants
                let to_remove: Vec<Hash> = self
                    .active
                    .iter_node_and_descendants(*invalid_block)
                    .unwrap()
                    .map(|(&h, _)| h)
                    .collect();
                for h in to_remove {
                    self.active.remove(h).unwrap();
                }
                // b) revert the blocks we already processed
                for h in path_from_ancestor_to_new_longest_chain[..invalid_block_index]
                    .iter()
                    .skip(1)
                {
                    self.bank.revert_block(self.active.get(h).unwrap());
                }
                // c) now the old longest chain is the longest chain again, process its blocks
                for h in path_from_ancestor_to_prev_longest_chain.iter().skip(1) {
                    self.bank
                        .process_block(self.active.get(h).unwrap())
                        .unwrap();
                }
            }
        }

        Ok(())
    }
    pub async fn try_create_block(&mut self) -> Result<(), ()> {
        let slot = self.cur_slot.ok_or(())?;
        let opens_queued = self.cur_opens.take().ok_or(())?;
        let txs_queued = self.cur_txs.take().ok_or(())?;
        
        let mut bank = self.bank.lock().unwrap();
        let batch = bank.new_batch();
        
        // Process + extract all the valid opens
        let mut opens = Vec::with_capacity(opens_queued.len());
        let mut open_hashes = Vec::with_capacity(opens_queued.len());
        for open in opens_queued.iter() {
            if bank.process_open(&open.val, batch).is_ok() {
                opens.push(open.val);
                open_hashes.push(open.hash);
            }
        }

        drop(bank);

        // Process and extract all the valid transactions
        let (send, recv) = oneshot::channel();
        let bank = self.bank.clone();
        rayon::spawn(move || {
            let bank = bank.lock().unwrap();
            let ((txs, tx_hashes), tasks): ((Vec<Transaction>, Vec<Hash>), Vec<Task>) = txs_queued
                .par_iter()
                .filter_map(|tx| -> Option<((Transaction, Hash), Task)>{
                    let task = bank.convert_transaction(&tx.val).ok()?;
                    bank.queue_task(&task, batch).ok()?;
                    Some(((tx.val, tx.hash), task))
                })
                .unzip();
            send.send((txs, tx_hashes, tasks)).unwrap();
        });
        let (txs, tx_hashes, tasks) = recv.unwrap();
        let (send, recv) = oneshot::channel();
        let bank = self.bank.clone();
        rayon::spawn(move || {
            let bank = bank.lock().unwrap();
            tasks.par_iter().for_each(|task| {
                bank.finish_task(task);
            });
            send.send(tasks).unwrap();
        });
        let tasks = recv.unwrap();
        
        // Create our block
        let block = Block::sign(
            self.id,
            slot,
            *self.active.get_longest_chain().0,
            opens,
            open_hashes,
            txs,
            tx_hashes,
            vec![],
            vec![]
        );
        self.add_block(Arc::new(block), tasks);
        Ok(())
    }*/
}

impl Process for State {
    const NAME: &'static str = "State";
    const RESTART_ON_CRASH: bool = true;
    fn run(&mut self, mut mailbox: Mailbox, _: Handle) -> Result<(), Error> {
        /*loop {
            let msg = mailbox.recv();
            match msg {
                Message::TransactionList(v) => {
                    let (slot, txs) = *v;
                    match self.cur_slot {
                        Some(cs) if cs < slot => continue,
                        _ => {}
                    }
                    self.cur_slot = Some(slot);
                    self.cur_txs = Some(txs);
                    self.try_create_block();
                },
                Message::OpenList(v) => {
                    let (slot, opens) = *v;
                    match self.cur_slot {
                        Some(cs) if cs < slot => continue,
                        _ => {}
                    }
                    self.cur_slot = Some(slot);
                    self.cur_opens = Some(opens);
                    self.try_create_block();
                },
                _ => {}
            }
        }*/
        loop {
            process::sleep(Duration::from_secs(1));
        }
    }
}
