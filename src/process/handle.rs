use std::{collections::HashMap, sync::Mutex};

use bincode::{de::{BorrowDecoder, Decoder}, enc::Encoder, error::{DecodeError, EncodeError}, BorrowDecode, Decode, Encode};
use kanal::Sender;

use super::Message;

#[derive(Clone)]
pub struct Handle(pub(super) Sender<Message>);

impl Handle {
    pub fn send(&self, msg: Message) {
        _ = self.0.send(msg);
    }
}


static HANDLES: Mutex<HashMap<Handle, usize>> = Mutex::new(HashMap::new());

impl Encode for Handle {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Err(EncodeError::UnexpectedEnd)
    }
}

impl Decode for Handle {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Err(DecodeError::LimitExceeded)
    }
}

impl<'de> BorrowDecode<'de> for Handle {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Err(DecodeError::LimitExceeded)
    }
}