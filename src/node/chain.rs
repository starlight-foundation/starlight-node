use std::rc::Rc;

use crate::{error, keys::Hash, protocol::Amount, util::Error};

use super::{Bank, Block, Dag};

pub struct Chain {
    /// The account state of the longest chain
    bank: Bank,
    /// All finalized blocks
    finalized: Vec<Rc<Block>>,
    /// Last finalized block (root) plus all blocks that are not yet finalized
    active: Dag<Hash, Rc<Block>>,
}

impl Chain {
    pub fn new(genesis_block: Rc<Block>) -> Result<Self, Error> {
        if !genesis_block.is_genesis() {
            return Err(error!("invalid genesis block"));
        }
        Ok(Self {
            active: Dag::new(genesis_block.hash, genesis_block.clone()),
            bank: Bank::new(genesis_block.leader),
            finalized: vec![genesis_block],
        })
    }
    pub fn finalize_hash(&mut self, h: Hash) -> Result<(), Error> {
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
    pub fn add_block(&mut self, block: Rc<Block>) -> Result<(), Error> {
        let previous = block.previous;
        let hash = block.hash;
        let (&prev_longest_chain, _) = self.active.get_longest_chain();
        let building_on_longest_chain = prev_longest_chain == previous;
        self.active.insert(hash, block, previous)?;
        let added_block_is_longest_chain = self.active.get_longest_chain().0 == &hash;
        if !building_on_longest_chain && added_block_is_longest_chain {
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
        }

        Ok(())
    }
}
