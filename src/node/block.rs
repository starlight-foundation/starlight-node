use crate::{blocks::{Hash, HashBuilder, Transaction, Vote}, keys::{Public, Signature}};

pub struct Block {
    pub slot: u64,
    pub previous: Hash,
    pub leader: Public,
    pub signature: Signature,
    pub transactions: Box<[Transaction]>,
    pub votes: Box<[Vote]>
}

fn merkle_row(hashes: &[Hash]) -> Box<[Hash]> {
    assert_eq!(hashes.len() % 2, 0);
    let mut row = Vec::with_capacity(hashes.len() / 2);
    for pair in hashes.chunks(2) {
        let mut hb = HashBuilder::new();
        hb.update(&pair[0].0);
        hb.update(&pair[1].0);
        row.push(hb.finalize());
    }
    row.into_boxed_slice()
}

impl Block {
    pub fn verify_and_hash(&self) -> Result<Hash, ()> {
        let mut tx_hashes = Vec::with_capacity(
            self.transactions.len() + self.transactions.len() % 2
        );
        for tx in self.transactions.iter() {
            tx_hashes.push(tx.verify_and_hash()?);
        }
        if self.transactions.len() % 2 != 0 {
            tx_hashes.push(*tx_hashes.last().unwrap());
        }
        let mut tx_row = merkle_row(&tx_hashes);
        while tx_row.len() > 1 {
            tx_row = merkle_row(&tx_row);
        }
        let tx_hash = tx_row[0];
        let mut vote_hashes = Vec::with_capacity(
            self.votes.len() + self.votes.len() % 2
        );
        for vote in self.votes.iter() {
            vote_hashes.push(vote.verify_and_hash()?);
        }
        if self.votes.len() % 2 != 0 {
            vote_hashes.push(*vote_hashes.last().unwrap());
        }
        let mut vote_row = merkle_row(&vote_hashes);
        while vote_row.len() > 1 {
            vote_row = merkle_row(&vote_row);
        }
        let vote_hash = vote_row[0];
        let block_hash = {
            let mut hb = HashBuilder::new();
            hb.update(&self.slot.to_le_bytes());
            hb.update(&self.previous.0);
            hb.update(&self.leader.0);
            hb.update(&tx_hash.0);
            hb.update(&vote_hash.0);
            hb.finalize()
        };
        self.leader.verify(&block_hash.0, &self.signature)?;
        Ok(block_hash)
    }
}
