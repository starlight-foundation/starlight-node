use memmap2::MmapMut;
use std::marker::PhantomData;
use crate::util::Error;

const FILE_SIZE: u64 = 4 * 1024 * 1024; // 4MB

/// A list backed by a set of memory-mapped files.
/// 
/// This struct maintains a vector of memory-mapped files, each 4MB in size,
/// that together provide the backing store for a list.
/// `ListStore` will append to the end of the last file until there is no more 
/// space, at which point it will create a new memory map.
pub struct ListStore<T> {
    memmaps: Vec<MmapMut>,
    directory: String,
    capacity: u64,
    len: u64,
    _phantom: PhantomData<T>,
}

impl<T> ListStore<T> {
    /// Opens a new `ListStore` in the given directory.
    pub fn open(directory: &str) -> Result<Self, Error> {
        std::fs::create_dir_all(directory)?;
        Ok(Self { 
            memmaps: Vec::new(),
            capacity: 0, 
            directory: directory.to_string(),
            len: 0,
            _phantom: PhantomData,
        })
    }

    /// Adds a new memory-mapped file to the list of memmaps.
    fn add_memmap(&mut self) {
        let path = format!("{}/memmap_{}.bin", self.directory, self.memmaps.len());
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .expect("Unable to open file");

        file.set_len(FILE_SIZE as u64).expect("Unable to set file size");
        
        let mmap = unsafe { 
            MmapMut::map_mut(&file).expect("Unable to memory map file") 
        };

        self.memmaps.push(mmap);
        self.capacity += FILE_SIZE / std::mem::size_of::<T>() as u64;
    }

    /// Appends an item to the end of the list.
    pub fn push(&mut self, item: T) {
        if self.len == self.capacity {
            self.add_memmap();
        }
        
        let memmap_index = self.len / (FILE_SIZE / std::mem::size_of::<T>() as u64);
        let offset = (self.len % (FILE_SIZE / std::mem::size_of::<T>() as u64)) * std::mem::size_of::<T>() as u64;
        
        unsafe {
            let ptr = self.memmaps[memmap_index as usize].as_mut_ptr().add(offset as usize) as *mut T;
            ptr.write(item);
        }
        
        self.len += 1;
    }
    
    /// Removes the last item from the list and returns it, or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            let memmap_index = self.len / (FILE_SIZE / std::mem::size_of::<T>() as u64);
            let offset = (self.len % (FILE_SIZE / std::mem::size_of::<T>() as u64)) * std::mem::size_of::<T>() as u64;
            
            unsafe {
                let ptr = self.memmaps[memmap_index as usize].as_ptr().add(offset as usize) as *const T;
                Some(ptr.read())
            }
        }
    }
    
    /// Returns a reference to the item at the given index, or `None` if out of bounds.
    pub fn get(&self, index: u64) -> Option<&T> {
        if index >= self.len {
            None  
        } else {
            let memmap_index = index / (FILE_SIZE / std::mem::size_of::<T>() as u64);
            let offset = (index % (FILE_SIZE / std::mem::size_of::<T>() as u64)) * std::mem::size_of::<T>() as u64;
            
            unsafe {
                let ptr = self.memmaps[memmap_index as usize].as_ptr().add(offset as usize) as *const T;
                Some(&*ptr)
            }
        }
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

