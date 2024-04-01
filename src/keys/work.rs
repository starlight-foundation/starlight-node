// Derived from the pow module of github.com/feeless/feeless@978eba7.
use super::Difficulty;
use super::Hash;
use crate::error;
use crate::hexify;
use crate::util::Error;
use blake2b_simd::Params;
use rand::RngCore;
use serde::Deserialize;
use serde::Serialize;
use std::convert::TryFrom;

/// The result of some proof of work (PoW). Can verify and inefficiently generate PoW using the CPU.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(align(8))]
pub struct Work(pub [u8; 8]);

hexify!(Work, "work");

#[static_init::dynamic]
static PARAMS: Params = {
    let mut params = Params::new();
    params.hash_length(8);
    params
};

impl Work {
    const LEN: usize = 8;

    pub fn zero() -> Self {
        Self([0u8; Self::LEN])
    }

    pub fn random() -> Self {
        let mut s = Self([0u8; Self::LEN]);
        rand::thread_rng().fill_bytes(&mut s.0);
        s
    }

    pub fn hash(work_and_subject: &[u8]) -> [u8; Self::LEN] {
        PARAMS.hash(work_and_subject).as_bytes().try_into().unwrap()
    }

    /// Block and generate forever until we find a solution.
    pub fn generate(subject: &Hash, threshold: Difficulty) -> Self {
        let mut work_and_subject = [0u8; 40];
        // We can place the subject in the second part of the slice which will not change.
        work_and_subject[Self::LEN..].copy_from_slice(subject.as_bytes());

        // Fill the first 8 bytes with the random work.
        let work_slice = &mut work_and_subject[0..Self::LEN];
        rand::thread_rng().fill_bytes(work_slice);

        loop {
            // Pick a random byte position and increment.
            // I'm guessing this is slightly faster than using fill_bytes for a new set of numbers.
            // TODO: Bench this guess.
            let idx = (rand::random::<u8>() % (Self::LEN as u8)) as usize;
            let c = work_and_subject[idx];
            work_and_subject[idx] = if c == 0xff { 0 } else { c + 1 };

            let b = Self::hash(&work_and_subject);
            let difficulty = Difficulty::from_le_fixed(&b);

            if difficulty >= threshold {
                break;
            }
        }

        let work_slice = &work_and_subject[0..Self::LEN];
        let mut work_bytes = Vec::from(work_slice);
        work_bytes.reverse();
        let work = Work::try_from(work_bytes.as_slice()).unwrap();
        work
    }

    pub fn verify(&self, subject: &Hash, threshold: Difficulty) -> Result<(), Error> {
        let difficulty = self.difficulty(subject);
        if difficulty >= threshold {
            Ok(())
        } else {
            Err(error!("not enough work"))
        }
    }

    pub fn difficulty(&self, subject: &Hash) -> Difficulty {
        let mut work_and_subject = [0u8; 40];

        // For some reason this is reversed!
        let mut reversed_work = self.0;
        reversed_work.reverse();

        work_and_subject[0..Self::LEN].copy_from_slice(&reversed_work);
        work_and_subject[Self::LEN..].copy_from_slice(subject.as_bytes());
        let hash = Self::hash(&work_and_subject);
        Difficulty::from_le_fixed(&hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Seed;
    use std::str::FromStr;

    #[test]
    fn verify() {
        // Each hash is incremented by one.
        let fixtures = vec![
            (
                "2387767168f9453db0eca227c79d7e7a31b78cafb58bd9cdee630881c70979b8",
                "c3f097857cc7106b",
                "fffffff867b3146b",
                true,
            ),
            (
                "2387767168f9453db0eca227c79d7e7a31b78cafb58bd9cdee630881c70979b9",
                "ec4f0960a70fdcbe",
                "fffffffde26451db",
                true,
            ),
            (
                "2387767168f9453db0eca227c79d7e7a31b78cafb58bd9cdee630881c70979ba",
                "b58e13f297179bc2",
                "fffffffb6fc1b4a6",
                true,
            ),
            // This is the same as above except the work is just zeros,
            // causing a totally different difficulty, and not enough work in this case.
            (
                "2387767168f9453db0eca227c79d7e7a31b78cafb58bd9cdee630881c70979ba",
                "0000000000000000",
                "357abcab02726362",
                false,
            ),
        ];

        let threshold = Difficulty::from_str("ffffffc000000000").unwrap();
        for fixture in fixtures {
            let (hash, work, expected_difficulty, is_enough_work) = &fixture;
            let hash = Hash::from_str(hash).unwrap();
            let work = Work::from_str(work).unwrap();
            let expected_difficulty = Difficulty::from_str(expected_difficulty).unwrap();
            let difficulty = work.difficulty(&hash);
            assert_eq!(difficulty, expected_difficulty, "{:?}", &fixture);
            assert_eq!(
                work.verify(&hash, threshold).is_ok(),
                *is_enough_work,
                "{:?}",
                &fixture
            );
        }
    }

    #[test]
    fn generate_work() {
        // Let's use a low difficulty so it doesn't take forever.
        let threshold = Difficulty::from_str("ffff000000000000").unwrap();
        dbg!(&threshold);

        let hash = Hash::random();
        dbg!(&hash);
        let work = Work::generate(&hash, threshold);
        dbg!(&work);
        assert!(work.verify(&hash, threshold).is_ok());
    }
}
