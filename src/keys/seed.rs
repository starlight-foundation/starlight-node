use super::Hash;
// Derived from the keys module of github.com/feeless/feeless@978eba7.
use crate::hexify;
use crate::keys::private::Private;
use rand::RngCore;

/// 256 bit seed used to derive multiple addresses.
///
/// See https://docs.nano.org/integration-guides/the-basics/#seed for details.
#[derive(Clone, PartialEq)]
pub struct Seed(pub [u8; 32]);

hexify!(Seed, "seed");

impl Seed {
    const LEN: usize = 32;

    pub fn zero() -> Self {
        Self([0; Self::LEN])
    }

    /// Generate a secure random seed.
    pub fn random() -> Self {
        let mut seed = Seed::zero();
        rand::thread_rng().fill_bytes(&mut seed.0);
        seed
    }

    /// Derive a private key from the seed with an index.
    ///
    /// https://docs.nano.org/integration-guides/the-basics/#seed
    pub fn derive(&self, index: u32) -> Private {
        let mut buf = [0u8; Self::LEN + 4];
        buf[..Self::LEN].copy_from_slice(&self.0);
        buf[Self::LEN..].copy_from_slice(&index.to_be_bytes());
        Private(Hash::of_slice(&buf).to_bytes())
    }
}
