use crate::{
    blocks::{Slot, Tx, Vote},
    keys::{Hash, HashBuilder, Public, Signature},
};

pub struct Block {
    pub slot: Slot,
    pub previous: Hash,
    pub leader: Public,
    pub signature: Signature,
    pub transactions: Box<[Tx]>,
    pub votes: Box<[Vote]>,
}

fn merkle_row(hashes: &[Hash]) -> Vec<Hash> {
    assert_eq!(hashes.len() % 2, 0);
    let mut row = Vec::with_capacity(hashes.len() / 2 + ((hashes.len() / 2) % 2));
    for pair in hashes.chunks(2) {
        let mut hb = HashBuilder::new();
        hb.update(&pair[0].as_bytes());
        hb.update(&pair[1].as_bytes());
        row.push(hb.finalize());
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
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
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
        let block_hash = {
            let mut hb = HashBuilder::new();
            hb.update(&self.slot.to_bytes());
            hb.update(self.previous.as_bytes());
            hb.update(self.leader.as_bytes());
            hb.update(tx_hash.as_bytes());
            hb.update(vote_hash.as_bytes());
            hb.finalize()
        };
        self.leader.verify(&block_hash, &self.signature)?;
        Ok(block_hash)
    }
}
