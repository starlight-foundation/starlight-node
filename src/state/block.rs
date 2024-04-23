use crate::{
    error,
    keys::{Hash, Private, Public, Signature},
    protocol::{Open, Slot, Transaction, Vote},
    util::{self, Error},
};

pub struct Block {
    /// The leader of the current block
    pub leader: Public,
    /// The signature of the leader of the current block
    pub signature: Signature,
    /// The slot of this block
    pub slot: Slot,
    /// The hash of the previous block
    pub previous: Hash,
    /// The hash of the current block
    pub hash: Hash,
    /// The merkle tree root of all accounts at this block
    pub state_hash: Hash,
    /// The account open requests in this block.
    pub opens: Vec<Open>,
    /// The transactions in this block.
    pub transactions: Vec<Transaction>,
    /// The votes in this block.
    pub votes: Vec<Vote>,
}

fn hash_block(
    slot: Slot,
    previous: &Hash,
    opens_hash: &Hash,
    transactions_hash: &Hash,
    vote_hash: &Hash,
) -> Hash {
    let mut buf = [0u8; 136];
    buf[0..8].copy_from_slice(&slot.to_bytes());
    buf[8..40].copy_from_slice(&previous.as_bytes());
    buf[40..72].copy_from_slice(&opens_hash.as_bytes());
    buf[72..104].copy_from_slice(&transactions_hash.as_bytes());
    buf[104..136].copy_from_slice(&vote_hash.as_bytes());
    Hash::digest(&buf)
}

impl Block {
    pub fn sign(
        private: Private,
        slot: Slot,
        previous: Hash,
        opens: Vec<Open>,
        open_hashes: Vec<Hash>,
        transactions: Vec<Transaction>,
        tx_hashes: Vec<Hash>,
        votes: Vec<Vote>,
        vote_hashes: Vec<Hash>,
    ) -> Result<Self, Error> {
        let opens_hash = util::merkle_root(
            &opens,
            |open| open.verify_and_hash(),
        )?;
        let transactions_hash = util::merkle_root(
            &transactions,
            |tr| tr.verify_and_hash(),
        )?;
        let vote_hash = util::merkle_root(
            &votes,
            |vote| vote.verify_and_hash(),
        )?;
        let hash = hash_block(
            slot,
            &previous,
            &opens_hash,
            &transactions_hash,
            &vote_hash,
        );
        let signature = private.sign(&hash);
    }
    pub fn genesis(private: Private) -> Self {
        let zero_hash = Hash::zero();
        let hash = hash_block(
            Slot::zero(),
            &zero_hash,
            &zero_hash,
            &zero_hash,
            &zero_hash,
        );
        let signature = private.sign(&hash);
        Self {
            leader: private.to_public(),
            signature,
            slot: Slot::zero(),
            previous: zero_hash,
            hash,
            state_hash: zero_hash,
            opens: Vec::new(),
            transactions: Vec::new(),
            votes: Vec::new(),
        }
    }
    pub fn is_genesis(&self) -> bool {
        self.opens.is_empty()
            && self.transactions.is_empty()
            && self.votes.is_empty()
            && self.slot == Slot::zero()
            && self.previous == Hash::zero()
            && self.state_hash == Hash::zero()
    }
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
        let opens_hash = util::merkle_root(
            &self.opens,
            |open| open.verify_and_hash(),
        )?;
        let transactions_hash = util::merkle_root(
            &self.transactions,
            |tr| tr.verify_and_hash(),
        )?;
        let vote_hash = util::merkle_root(
            &self.votes,
            |vote| vote.verify_and_hash(),
        )?;
        let block_hash = hash_block(
            self.slot,
            &self.previous,
            &opens_hash,
            &transactions_hash,
            &vote_hash,
        );
        if self.hash != block_hash {
            return Err(error!("claimed block hash and actual hash do not match"));
        }
        self.leader.verify(&block_hash, &self.signature)?;
        Ok(block_hash)
    }
}
