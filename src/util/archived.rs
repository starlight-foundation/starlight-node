use std::marker::PhantomData;

use bincode::{Encode, Decode};

pub trait ArchivableTo<S> {
    fn archive(self) -> S;
    fn unarchive(source: S) -> Self;
}

/// Archived version of a type `T`, represented as a source `S`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode)]
pub struct Archived<T: ArchivableTo<S>, S> {
    source: S,
    _phantom: PhantomData<T>,
}

impl<T: ArchivableTo<S>, S> Archived<T, S> {
    pub fn new(v: T) -> Self {
        Self {
            source: v.archive(),
            _phantom: PhantomData,
        }
    }
    pub fn get(self) -> T {
        T::unarchive(self.source)
    }
}
