use std::marker::PhantomData;
use memmap2::MmapRaw;

use crate::util::Error;

pub struct ObjectStore<T: Copy> {
    memmap: MmapRaw,
    _phantom: PhantomData<T>,                                   
}

impl<T: Copy> ObjectStore<T> {
    pub fn open(file_path: &str, default: T) -> Result<Self, Error> {
        let exists = std::fs::metadata(file_path).is_ok();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)?;

        // Ensure the file is at least as large as T
        file.set_len(std::mem::size_of::<T>() as u64)?;

        let memmap = unsafe { MmapRaw::map_raw(&file)? };

        Ok(Self {
            memmap,
            _phantom: PhantomData,
        })
    }

    pub fn get(&self) -> T {
        let data = unsafe { std::slice::from_raw_parts(self.memmap.as_ptr(), std::mem::size_of::<T>()) };
        let value: T = unsafe { std::ptr::read(data.as_ptr() as *const _) };
        value
    }

    pub fn put(&mut self, value: T) -> Result<(), Error> {
        let bytes = crate::util::view_as_bytes(&value);
        let data = unsafe { std::slice::from_raw_parts_mut(self.memmap.as_mut_ptr(), std::mem::size_of::<T>()) };
        data.copy_from_slice(bytes);
        self.memmap.flush()?;
        Ok(())
    }
}
