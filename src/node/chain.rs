use std::rc::Rc;

use crate::{protocol::Amount, error, keys::Hash, util::Error};

use super::{Bank, Block, Dag};

pub struct Chain {
    /// The account state of the longest chain
    bank: Bank,
    /// All finalized blocks
    finalized: Vec<Rc<Block>>,
    /// Last finalized block (root) plus all blocks that are not yet finalized
    active: Dag<Hash, Rc<Block>>
}

impl Chain {
    pub fn new(genesis_block: Rc<Block>) -> Result<Self, Error> {
        if !genesis_block.is_genesis() {
            return Err(error!("invalid genesis block"));
        }
        let mut active = Dag::new();
        active.insert(genesis_block.hash, genesis_block.clone(), None).unwrap();
        Ok(Self {
            bank: Bank::new(genesis_block.leader),
            finalized: vec![genesis_block],
            active
        })
    }
    pub fn add_block(&mut self, block: Rc<Block>) -> Result<bool, Error> {
        let hash = block.hash;
        let previous = block.previous;
        let prev_longest_chain = self.active.get_longest_chain().unwrap();
        match self.active.insert(hash, block, Some(previous)) {
            Ok(true) => {},
            v => return v,
        }
        let new_longest_chain = self.active.get_longest_chain().unwrap();
        
        Ok(true)
    }

}
