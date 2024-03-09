// Derived from the keys module of github.com/feeless/feeless@978eba7.
use crate::{bail, error};
use crate::keys::{public::Public, signature::Signature};
use crate::error::Error;
use std::fmt::{Display, Formatter};
use std::str::{FromStr, Split};

#[derive(Debug)]
pub struct Armor {
    message: String,
    public: Public,
    signature: Signature,
}

impl Armor {
    const BEGIN_MESSAGE: &'static str = "-----BEGIN NANO SIGNED MESSAGE-----";
    const BEGIN_ADDRESS: &'static str = "-----BEGIN NANO ADDRESS-----";
    const BEGIN_SIGNATURE: &'static str = "-----BEGIN NANO SIGNATURE-----";
    const END_SIGNATURE: &'static str = "-----END NANO SIGNATURE-----";

    pub fn new(message: String, public: Public, signature: Signature) -> Self {
        Self {
            message,
            public,
            signature,
        }
    }

    pub fn verify(&self) -> Result<(), ()> {
        self.public.verify(self.message.as_bytes(), &self.signature)
    }
}

impl Display for Armor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::BEGIN_MESSAGE)?;
        f.write_str("\n")?;
        f.write_str(&self.message.to_string())?;
        f.write_str("\n")?;
        f.write_str(Self::BEGIN_ADDRESS)?;
        f.write_str("\n")?;
        f.write_str(&self.public.to_address())?;
        f.write_str("\n")?;
        f.write_str(Self::BEGIN_SIGNATURE)?;
        f.write_str("\n")?;
        f.write_str(&self.signature.to_string())?;
        f.write_str("\n")?;
        f.write_str(Self::END_SIGNATURE)
    }
}

impl FromStr for Armor {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut iter = s.split("\n");

        decode_static(Self::BEGIN_MESSAGE, iter.next(), "begin message")?;
        let message = decode_part(&mut iter, "Missing message")?;

        decode_static(Self::BEGIN_ADDRESS, iter.next(), "begin address")?;
        let address_str = decode_part(&mut iter, "Missing address")?;
        let public = Public::from_address(&address_str)?;

        decode_static(Self::BEGIN_SIGNATURE, iter.next(), "begin signature")?;
        let signature_str = decode_part(&mut iter, "Missing signature")?;
        let signature = Signature::from_str(&signature_str)?;

        decode_static(Self::END_SIGNATURE, iter.next(), "end signature")?;

        Ok(Self::new(message, public, signature))
    }
}

fn decode_static(expected: &str, got: Option<&str>, what: &str) -> Result<(), Error> {
    if let Some(begin) = got {
        let begin = begin.trim();
        if begin != expected {
            bail!(
                "Incorrect {}: Expecting: {} Got: {}",
                what, expected, begin
            );
        }
    } else {
        bail!("invalid armor: Missing {}", what);
    }

    Ok(())
}

fn decode_part(iter: &mut Split<&str>, what: &str) -> Result<String, Error> {
    Ok(iter
        .next()
        .ok_or_else(|| error!("invalid armor: {}", what))?
        .trim()
        .to_owned())
}
