// Derived from the keys module of github.com/feeless/feeless@978eba7.
use super::Hash;
use crate::hexify;
use crate::keys::public::Public;
use crate::keys::signature::Signature;
use ed25519_dalek_blake2_feeless::{ExpandedSecretKey, PublicKey, SecretKey};
use rand::RngCore;

/// 256 bit private key which can generate a public key.
#[derive(Clone, Copy)]
#[repr(align(8))]
pub struct Private(pub(super) [u8; 32]);

hexify!(Private, "private key");

impl Private {
    pub const LEN: usize = 32;

    /// The zero private key
    pub(super) const fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn random() -> Self {
        let mut private = Private::zero();
        rand::thread_rng().fill_bytes(&mut private.0);
        private
    }

    /// Generate the public key for this private key.
    ///
    /// If you wish to convert this private key to a Nano address you will need to take another
    /// step:
    /// ```
    /// use feeless::Private;
    /// use std::str::FromStr;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let s = "0000000000000000000000000000000000000000000000000000000000000000";
    /// let address = Private::from_str(s)?.to_public().to_address();
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_public(&self) -> Public {
        Public::from(self.internal_public())
    }

    fn to_ed25519_dalek(&self) -> SecretKey {
        SecretKey::from_bytes(&self.0).unwrap()
    }

    fn internal_public(&self) -> PublicKey {
        PublicKey::from(&self.to_ed25519_dalek())
    }

    pub fn to_address(&self) -> String {
        self.to_public().to_address()
    }

    pub fn sign(&self, hash: &Hash) -> Signature {
        let dalek = self.to_ed25519_dalek();
        let public = PublicKey::from(&dalek);
        let expanded_secret = ExpandedSecretKey::from(&dalek);
        let internal_signed = expanded_secret.sign(hash.as_bytes(), &public);
        Signature::from_bytes(internal_signed.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::Hash;
    use crate::keys::seed::Seed;

    #[test]
    fn signing() {
        let hash = Hash::digest(&[1, 2, 3, 4, 5]);
        let private = Seed::random().derive(0);
        let public = private.to_public();
        let signature = private.sign(&hash);
        assert!(public.verify(&hash, &signature).is_ok());
    }
}
