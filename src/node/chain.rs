use std::{
    cell::RefCell, collections::{HashMap, HashSet}, hash::Hash, rc::Rc
};

use crate::{error, util::Error};

struct Block<B> {
    height: u64,
    prev: Option<BlockRef<B>>,
    next: Option<BlockRef<B>>,
    sibling: Option<BlockRef<B>>,
    block: B,
}

type BlockRef<B> = Rc<RefCell<Block<B>>>;

pub trait ChainBlock<H> {
    fn get_hash(&self) -> H;
    fn get_previous_hash(&self) -> Option<H>;
}

pub struct Chain<H: Hash + Eq + Clone, B: ChainBlock<H>> {
    // All blocks in the blockchain.
    blocks: HashMap<H, BlockRef<B>>,
    // All blocks at the head of the blockchain.
    heads: HashMap<H, BlockRef<B>>,
    // The block at the head of the longest chain.
    longest_chain: Option<BlockRef<B>>,
}

impl<H: Hash + Eq + Clone, B: ChainBlock<H>> Chain<H, B> {
    /// Creates a new Chain.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            heads: HashMap::new(),
            longest_chain: None,
        }
    }

    /// Block B must not be modified while the reference exists
    unsafe fn cvt_ref<'a, 'b>(&'a self, block_ref: &'b BlockRef<B>) -> &'a B {
        let block = &block_ref.borrow().block;
        let block = block as *const B;
        let block = unsafe { &*block };
        block
    }

    /// Get the block corresponding to the longest chain
    /// Safety: The lifetime prevents modification of any block while the reference is active
    pub fn get_longest_chain(&self) -> Option<&B> {
        self.longest_chain
            .as_ref()
            .map(|block_ref| unsafe { self.cvt_ref(block_ref) })
    }

    /// Insert a block pair into the blockchain if it does not already exist.
    /// If `block.prev` exists, it must reference a valid block in the chain.
    /// Updates `self.longest_chain` if necessary.
    pub fn try_insert(&mut self, block: B) -> Result<(), Error> {
        let hash = block.get_hash();
        if self.blocks.contains_key(&hash) {
            return Err(error!("Block already exists in the chain"));
        }

        let prev_hash_maybe = block.get_previous_hash();
        let block_ref = Rc::new(RefCell::new(Block {
            height: 0,
            prev: None,
            next: None,
            sibling: None,
            block,
        }));

        if let Some(prev_hash) = &prev_hash_maybe {
            let prev_block_ref = self.blocks.get(prev_hash).ok_or_else(
                || error!("can't find block.prev in chain")
            )?;
            block_ref.borrow_mut().prev = Some(Rc::clone(prev_block_ref));
            block_ref.borrow_mut().height = prev_block_ref.borrow().height + 1;
            block_ref.borrow_mut().sibling = prev_block_ref.borrow_mut().next.take();
            prev_block_ref.borrow_mut().next = Some(Rc::clone(&block_ref));
        }

        self.blocks.insert(hash.clone(), Rc::clone(&block_ref));

        if let Some(prev_hash) = &prev_hash_maybe {
            self.heads.remove(prev_hash);
        }
        self.heads.insert(hash, Rc::clone(&block_ref));

        if self.longest_chain.is_none()
            || block_ref.borrow().height > self.longest_chain.as_ref().unwrap().borrow().height
        {
            self.longest_chain = Some(block_ref);
        }

        Ok(())
    }
    /// Iterate over all ancestors of the block denoted by `hash`, starting at `hash.prev` and working backwards.
    /// Returns `None` if the block denoted by `hash` does not exist.
    /// Safety: No blocks can be modified while the iterator is active
    pub fn try_iter_ancestors(&self, hash: H) -> Option<impl Iterator<Item = &B>> {
        let block = self.blocks.get(&hash)?;
        Some(std::iter::successors(block.borrow().prev.clone(), |x| {
            x.borrow().prev.clone()
        }).map(|x| unsafe { self.cvt_ref(&x) }))
    }

    /// Iterate over all descendants of the block denoted by `hash`.
    /// Returns `None` if the block denoted by `hash` does not exist.
    /// Safety: No blocks can be modified while the iterator is active
    pub fn try_iter_descendants(&self, hash: H) -> Option<impl Iterator<Item = &B>> {
        let block = self.blocks.get(&hash)?;
        let mut descendants = vec![Rc::clone(&block)];
        let mut next_blocks = vec![block.borrow().next.clone()];

        while !next_blocks.is_empty() {
            let mut new_next_blocks = Vec::new();
            for next_block in next_blocks.drain(..) {
                if let Some(next_block) = next_block {
                    descendants.push(Rc::clone(&next_block));
                    new_next_blocks.push(next_block.borrow().next.clone());
                    if let Some(sibling) = next_block.borrow().sibling.clone() {
                        new_next_blocks.push(Some(sibling));
                    }
                }
            }
            next_blocks = new_next_blocks;
        }

        Some(descendants.into_iter().skip(1).map(|x| unsafe { self.cvt_ref(&x) }))
    }

    /// Get the common ancestor of the two blocks denoted by `hash1` and `hash2`, or None if one does not exist.
    /// Returns `None` if the block denoted by `hash` does not exist.
    /// Safety: No blocks can be modified while the reference is active
    pub fn get_common_ancestor(&self, hash1: H, hash2: H) -> Option<&B> {
        let mut block1 = self.blocks.get(&hash1)?.clone();
        let mut block2 = self.blocks.get(&hash2)?.clone();

        while !Rc::ptr_eq(&block1, &block2) {
            if block1.borrow().height > block2.borrow().height {
                let prev = block1.borrow().prev.as_ref()?.clone();
                block1 = prev;
            } else {
                let prev = block2.borrow().prev.as_ref()?.clone();
                block2 = prev;
            }
        }

        Some(unsafe { self.cvt_ref(&block1) })
    }

    /// If the block B denoted by `hash` exists in the blockchain, set the corresponding block as the "root" block.
    /// This removes all blocks that are not descendants of B, or B itself.
    pub fn try_set_root(&mut self, hash: H) -> bool {
        let block_ref = match self.blocks.get(&hash) {
            Some(block_ref) => block_ref,
            None => return false,
        };

        let mut descendants: HashSet<H> = self
            .try_iter_descendants(hash)
            .unwrap()
            .map(|x| x.get_hash())
            .collect();
        descendants.insert(block_ref.borrow().block.get_hash());

        self.blocks
            .retain(|_, block_ref| descendants.contains(&block_ref.borrow().block.get_hash()));
        self.heads
            .retain(|_, block_ref| descendants.contains(&block_ref.borrow().block.get_hash()));
        if let Some(longest_chain_hash) = self
            .longest_chain
            .as_ref()
            .map(|x| x.borrow().block.get_hash())
        {
            if !self.heads.contains_key(&longest_chain_hash) {
                self.longest_chain = self
                    .heads
                    .values()
                    .max_by(|a, b| a.borrow().height.cmp(&b.borrow().height))
                    .map(|x| x.clone());
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Define a simple block struct for testing
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    struct TestBlock {
        hash: u64,
        prev_hash: Option<u64>,
    }

    impl ChainBlock<u64> for TestBlock {
        fn get_hash(&self) -> u64 {
            self.hash
        }

        fn get_previous_hash(&self) -> Option<u64> {
            self.prev_hash
        }
    }

    // Helper function to create a test block
    fn create_block(hash: u64, prev_hash: Option<u64>) -> TestBlock {
        TestBlock { hash, prev_hash }
    }

    #[test]
    fn test_new_chain() {
        let chain: Chain<u64, TestBlock> = Chain::new();
        assert!(chain.blocks.is_empty());
        assert!(chain.heads.is_empty());
        assert!(chain.longest_chain.is_none());
    }

    #[test]
    fn test_try_insert() {
        let mut chain = Chain::new();

        // Insert a block with no previous hash
        let block1 = create_block(1, None);
        assert!(chain.try_insert(block1.clone()).is_ok());
        assert_eq!(chain.blocks.len(), 1);
        assert_eq!(chain.heads.len(), 1);
        assert!(chain.longest_chain.is_some());

        // Insert a block with a previous hash
        let block2 = create_block(2, Some(1));
        assert!(chain.try_insert(block2.clone()).is_ok());
        assert_eq!(chain.blocks.len(), 2);
        assert_eq!(chain.heads.len(), 1);
        assert!(chain.longest_chain.is_some());

        // Insert a block with an invalid previous hash
        let block3 = create_block(3, Some(4));
        assert!(chain.try_insert(block3).is_err());
        assert_eq!(chain.blocks.len(), 2);
        assert_eq!(chain.heads.len(), 1);
    }

    #[test]
    fn test_get_longest_chain() {
        let mut chain = Chain::new();

        // Insert blocks to form a chain
        let block1 = create_block(1, None);
        let block2 = create_block(2, Some(1));
        let block3 = create_block(3, Some(2));
        chain.try_insert(block1).unwrap();
        chain.try_insert(block2).unwrap();
        chain.try_insert(block3.clone()).unwrap();

        assert_eq!(chain.get_longest_chain(), Some(&block3));
    }

    #[test]
    fn test_try_iter_ancestors() {
        let mut chain = Chain::new();

        // Insert blocks to form a chain
        let block1 = create_block(1, None);
        let block2 = create_block(2, Some(1));
        let block3 = create_block(3, Some(2));
        chain.try_insert(block1).unwrap();
        chain.try_insert(block2).unwrap();
        chain.try_insert(block3).unwrap();

        // Iterate over ancestors of block3
        let ancestors: Vec<&TestBlock> = chain
            .try_iter_ancestors(3)
            .unwrap()
            .collect();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].hash, 2);
        assert_eq!(ancestors[1].hash, 1);

        // Try to iterate over ancestors of a non-existent block
        assert!(chain.try_iter_ancestors(4).is_none());
    }

    #[test]
    fn test_try_iter_descendants() {
        let mut chain = Chain::new();

        // Insert blocks to form a chain
        let block1 = create_block(1, None);
        let block2 = create_block(2, Some(1));
        let block3 = create_block(3, Some(2));
        chain.try_insert(block1).unwrap();
        chain.try_insert(block2).unwrap();
        chain.try_insert(block3).unwrap();

        // Iterate over descendants of block1
        let descendants: Vec<&TestBlock> = chain.try_iter_descendants(1).unwrap().collect();
        assert_eq!(descendants.len(), 2);
        assert_eq!(descendants[0].hash, 2);
        assert_eq!(descendants[1].hash, 3);

        // Try to iterate over descendants of a non-existent block
        assert!(chain.try_iter_descendants(4).is_none());
    }

    #[test]
    fn test_get_common_ancestor() {
        let mut chain = Chain::new();

        // Insert blocks to form a chain
        let block1 = create_block(1, None);
        let block2 = create_block(2, Some(1));
        let block3 = create_block(3, Some(2));
        let block4 = create_block(4, Some(2));
        chain.try_insert(block1).unwrap();
        chain.try_insert(block2).unwrap();
        chain.try_insert(block3).unwrap();
        chain.try_insert(block4).unwrap();

        // Get common ancestor of block3 and block4
        assert_eq!(chain.get_common_ancestor(3, 4), Some(&block2));

        // Get common ancestor of block1 and block3
        assert_eq!(chain.get_common_ancestor(1, 3), Some(&block1));

        // Get common ancestor of a non-existent block
        assert!(chain.get_common_ancestor(1, 5).is_none());
    }

    #[test]
    fn test_try_set_root() {
        let mut chain = Chain::new();

        // Insert blocks to form a chain
        let block1 = create_block(1, None);
        let block2 = create_block(2, Some(1));
        let block3 = create_block(3, Some(2));
        let block4 = create_block(4, Some(2));
        chain.try_insert(block1).unwrap();
        chain.try_insert(block2).unwrap();
        chain.try_insert(block3).unwrap();
        chain.try_insert(block4).unwrap();

        // Set block2 as the root
        assert!(chain.try_set_root(2));
        assert_eq!(chain.blocks.len(), 3);
        assert_eq!(chain.heads.len(), 2);
        assert_eq!(chain.longest_chain.as_ref().unwrap().borrow().block.hash, 3);

        // Try to set a non-existent block as the root
        assert!(!chain.try_set_root(5));
    }
}