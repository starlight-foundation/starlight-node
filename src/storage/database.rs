use std::fs;
use heed::{bytemuck::Pod, types::OwnedType, Database as HeedDatabase, Env, EnvOpenOptions};
use crate::util::Error;

/// Simple key-value storage built upon LMDB.
pub struct Database<K: Pod, V: Pod> {
    env: Env,
    db: HeedDatabase<OwnedType<K>, OwnedType<V>>
}

impl<K: Pod, V: Pod> Database<K, V> {
    pub fn open(directory: &str) -> Result<Self, Error> {
        fs::create_dir_all(directory)?;
        let env = EnvOpenOptions::new().open(directory)?;
        Ok(Self {
            db: env.create_database(None)?,
            env
        })
    }
    pub fn len(&self) -> u64 {
        self.db.len(&self.env.read_txn().unwrap()).unwrap()
    }
    pub fn get(&self, k: &K) -> Option<V> {
        let rtxn = self.env.read_txn().unwrap();
        self.db.get(&rtxn, k).unwrap()
    }
    pub fn contains_key(&self, k: &K) -> bool {
        self.get(k).is_some()
    }
    pub fn put(&self, k: &K, v: &V) {
        let mut wtxn = self.env.write_txn().unwrap();
        self.db.put(&mut wtxn, k, v).unwrap();
        wtxn.commit().unwrap();
    }
    pub fn remove(&self, k: &K) {
        let mut wtxn = self.env.write_txn().unwrap();
        self.db.delete(&mut wtxn, k).unwrap();
        wtxn.commit().unwrap();
    }
}

