use std::{collections::HashMap, sync::{atomic::AtomicU64, Mutex}};

use bincode::{de::{BorrowDecoder, Decoder}, enc::Encoder, error::{DecodeError, EncodeError}, BorrowDecode, Decode, Encode};
use kanal::Sender;

use crate::static_assert;

use super::Message;

#[derive(Clone)]
pub struct Handle(pub(super) Sender<Message>);

impl Handle {
    pub fn send(&self, msg: Message) {
        // we don't ever want to block when sending!
        _ = self.0.try_send(msg);
    }
}

static_assert!(std::mem::size_of::<Handle>() == std::mem::size_of::<usize>());

impl Handle {
    // safety: guaranteed by above static assert
    const fn to_usize(&self) -> usize {
        unsafe {
            std::mem::transmute_copy(self)
        }
    }
}

#[static_init::dynamic]
static HANDLE_MAP: Mutex<HashMap<u64, Handle>> = Mutex::new(HashMap::new());

impl Handle {
    pub(super) fn deactivate(&self) {
        HANDLE_MAP.lock().unwrap().remove(&(self.to_usize() as u64));
    }
}

impl Encode for Handle {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let h = self.to_usize() as u64;
        HANDLE_MAP.lock().unwrap().insert(h, self.clone());
        h.encode(encoder)
    }
}

impl Decode for Handle {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let h = u64::decode(decoder)?;
        Ok(HANDLE_MAP.lock().unwrap().get(&h).ok_or(
            DecodeError::Other("handle not found")
        )?.clone())
    }
}

impl<'de> BorrowDecode<'de> for Handle {
    fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let h = u64::borrow_decode(decoder)?;
        Ok(HANDLE_MAP.lock().unwrap().get(&h).ok_or(
            DecodeError::Other("handle not found")
        )?.clone())
    }
}