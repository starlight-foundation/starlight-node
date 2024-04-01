use crate::{
    error,
    keys::{Hash, Private, Public, Signature},
    protocol::{Slot, Transaction, Vote},
    util::Error,
};

pub struct Block {
    pub leader: Public,
    pub signature: Signature,
    pub slot: Slot,
    pub previous: Hash,
    pub hash: Hash,
    pub transactions: Vec<Transaction>,
    pub votes: Vec<Vote>,
}

fn merkle_row(hashes: &[Hash]) -> Vec<Hash> {
    assert_eq!(hashes.len() % 2, 0);
    let mut row = Vec::with_capacity(hashes.len() / 2 + ((hashes.len() / 2) % 2));
    let mut buf = [0u8; 64];
    for pair in hashes.chunks(2) {
        buf[0..32].copy_from_slice(&pair[0].as_bytes());
        buf[32..64].copy_from_slice(&pair[1].as_bytes());
        row.push(Hash::digest(&buf));
    }
    if row.len() < row.capacity() {
        row.push(Hash::zero());
    }
    row
}

fn merkle_root(hashes: Vec<Hash>) -> Hash {
    let mut row = hashes;
    while row.len() > 1 {
        row = merkle_row(&row);
    }
    row[0]
}

impl Block {
    fn hash_inner(slot: Slot, previous: &Hash, tx_hash: &Hash, vote_hash: &Hash) -> Hash {
        let mut buf = [0u8; 104];
        buf[0..8].copy_from_slice(&slot.to_bytes());
        buf[8..40].copy_from_slice(&previous.as_bytes());
        buf[40..72].copy_from_slice(&tx_hash.as_bytes());
        buf[72..104].copy_from_slice(&vote_hash.as_bytes());
        Hash::digest(&buf)
    }
    pub fn genesis(private: Private) -> Self {
        let zero_hash = Hash::zero();
        let hash = Self::hash_inner(Slot::zero(), &zero_hash, &zero_hash, &zero_hash);
        let signature = private.sign(&hash);
        Self {
            leader: private.to_public(),
            signature,
            slot: Slot::zero(),
            previous: zero_hash,
            hash,
            transactions: Vec::new(),
            votes: Vec::new(),
        }
    }
    pub fn is_genesis(&self) -> bool {
        self.transactions.is_empty()
            && self.votes.is_empty()
            && self.slot == Slot::zero()
            && self.previous == Hash::zero()
    }
    pub fn verify_and_hash(&self) -> Result<Hash, Error> {
        let tx_hash = match self.transactions.len() {
            0 => Hash::zero(),
            1 => self.transactions[0].verify_and_hash()?,
            _ => {
                let mut tx_hashes =
                    Vec::with_capacity(self.transactions.len() + self.transactions.len() % 2);
                for tx in self.transactions.iter() {
                    tx_hashes.push(tx.verify_and_hash()?);
                }
                if tx_hashes.len() < tx_hashes.capacity() {
                    tx_hashes.push(Hash::zero());
                }
                merkle_root(tx_hashes)
            }
        };
        let vote_hash = match self.votes.len() {
            0 => Hash::zero(),
            1 => self.votes[0].verify_and_hash()?,
            _ => {
                let mut vote_hashes = Vec::with_capacity(self.votes.len() + self.votes.len() % 2);
                for vote in self.votes.iter() {
                    vote_hashes.push(vote.verify_and_hash()?);
                }
                if vote_hashes.len() < vote_hashes.capacity() {
                    vote_hashes.push(Hash::zero());
                }
                merkle_root(vote_hashes)
            }
        };
        let block_hash = Self::hash_inner(self.slot, &self.previous, &tx_hash, &vote_hash);
        if self.hash != block_hash {
            return Err(error!("claimed block hash and actual hash do not match"));
        }
        self.leader.verify(&block_hash, &self.signature)?;
        Ok(block_hash)
    }
}
