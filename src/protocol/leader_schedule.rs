use rand::{
    distributions::{Distribution, WeightedIndex},
    SeedableRng,
};
use rand_chacha::ChaChaRng;

use crate::{keys::Public, static_assert, util::DefaultInitVec};

use super::{Amount, Epoch, Slot};

const NUM_CONSECUTIVE_LEADER_SLOTS: usize = 4;
static_assert!((Epoch::LEN % NUM_CONSECUTIVE_LEADER_SLOTS) == 0);

pub struct LeaderSchedule {
    pub epoch: Epoch,
    pub leaders: Vec<Public>,
}

impl LeaderSchedule {
    pub fn empty() -> Self {
        Self {
            epoch: Epoch::max(),
            leaders: Vec::default_init(Epoch::LEN / NUM_CONSECUTIVE_LEADER_SLOTS),
        }
    }
    pub fn fill(
        &mut self,
        mut voters: Vec<Public>,
        get_weight: impl Fn(&Public) -> Amount,
        epoch: Epoch,
    ) {
        voters.sort();
        let mut seed = [0u8; 32];
        seed[0..8].copy_from_slice(&epoch.to_bytes());
        let mut rng = ChaChaRng::from_seed(seed);
        let weighted_index =
            WeightedIndex::new(voters.iter().map(|x| get_weight(x).to_raw())).unwrap();
        for i in 0..self.leaders.len() {
            self.leaders[i] = voters[weighted_index.sample(&mut rng)];
        }
        self.epoch = epoch;
    }
    pub fn get(&self, slot: Slot) -> Option<Public> {
        if self.epoch == Epoch::max() {
            return None;
        }
        let idx = self.epoch.index_of(slot)?;
        Some(self.leaders[idx / 4])
    }
}
