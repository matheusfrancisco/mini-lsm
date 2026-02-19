use std::{
    collections::HashMap,
    os::macos::raw::stat,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Arg;
use parking_lot::{Mutex, RwLock};

use crate::mem_table::MemTable;

/// Represents the state of the storage engine.
#[derive(Clone)]
pub struct LsmStorageState {
    /// The current memtable.
    pub memtable: Arc<MemTable>,
    /// Immutable memtables, from latest to earliest.
    pub imm_memtables: Vec<Arc<MemTable>>,
    /// L0 SSTs, from latest to earliest.
    pub l0_sstables: Vec<usize>,
    /// SsTables sorted by key range; L1 - L_max for leveled compaction, or tiers for tiered
    /// compaction.
    pub levels: Vec<(usize, Vec<usize>)>,
    // SST objects.
    //pub sstables: HashMap<usize, Arc<SsTable>>, not implemented yet
}

impl LsmStorageState {
    pub fn create() -> Self {
        Self {
            memtable: Arc::new(MemTable::create(0)),
            imm_memtables: Vec::new(),
            l0_sstables: Vec::new(),
            levels: Vec::new(),
        }
    }
}

/// The storage interface of the LSM tree.
pub(crate) struct LsmStorageInner {
    pub(crate) state: Arc<RwLock<Arc<LsmStorageState>>>,
    pub(crate) state_lock: Mutex<()>,
    path: PathBuf,
    // not implemented yet
    //pub(crate) block_cache: Arc<BlockCache>,
    //next_sst_id: AtomicUsize,
    //pub(crate) options: Arc<LsmStorageOptions>,
    //pub(crate) compaction_controller: CompactionController,
    //pub(crate) manifest: Option<Manifest>,
    //pub(crate) mvcc: Option<LsmMvccInner>,
    //pub(crate) compaction_filters: Arc<Mutex<Vec<CompactionFilter>>>,
}

impl LsmStorageInner {
    /// Start the storage engine by either loading an existing directory or creating a new one if the directory does
    pub(crate) fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let state = LsmStorageState::create(); // TODO: add the options in the future

        // TODO: add compaction controller.
        //
        // For simplicity, we will not implement loading existing state from disk in this example.
        // In a real implementation, you would need to read the manifest and load the memtable and SSTables.
        Ok(Self {
            state: Arc::new(RwLock::new(Arc::new(state))),
            state_lock: Mutex::new(()),
            path: path.to_path_buf(),
        })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        let guard = self.state.read();
        guard.memtable.put(key, value)
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let guard = self.state.read();
        guard.memtable.get(key).map(|v| v.to_vec())
    }

    pub fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
        let guard = self.state.read();
        guard.memtable.put(key, b"") // Using empty value to represent deletion for simplicity.
    }
}

#[cfg(test)]
mod tests {
    use crate::lsm_storage::LsmStorageInner;

    #[test]
    fn test_put() {
        let storage = LsmStorageInner::open("test_data").unwrap();
        storage.put(b"key1", b"value1").unwrap();
        storage.put(b"key2", b"value2").unwrap();

        assert_eq!(storage.get(b"key1").unwrap(), b"value1".to_vec());
        assert_eq!(storage.get(b"key2").unwrap(), b"value2".to_vec());

        storage.put(b"key1", b"value2").unwrap();
        assert_eq!(storage.get(b"key1").unwrap(), b"value2".to_vec());

        storage.delete(b"key1").unwrap();
        assert_eq!(storage.get(b"key1").unwrap(), b"".to_vec());
    }

    #[test]
    fn test_get_nonexistent_key() {
        let storage = LsmStorageInner::open("test_data").unwrap();
        assert!(storage.get(b"nonexistent").is_none());
    }
}

