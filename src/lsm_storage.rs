use std::{
    collections::HashMap,
    os::macos::raw::stat,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::Result;
use clap::Arg;
use parking_lot::{Mutex, MutexGuard, RwLock};

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

pub struct LsmStorageOptions {
    // Block size in bytes
    pub block_size: usize,
    // SST size in bytes, also the approximate memtable capacity limit
    pub target_sst_size: usize,
    // Maximum number of memtables in memory, flush to L0 when exceeding this limit
    pub num_memtable_limit: usize,
    pub enable_wal: bool,
}

impl LsmStorageOptions {
    pub fn default_for_week1_test() -> Self {
        Self {
            block_size: 4096, // 4KB block size, which is a common choice for LSM trees and matches the typical disk block size.
            target_sst_size: 2 << 20, // 2MB SSTable size, which is a reasonable size for testing
            // and allows us to see the effects of flushing and compaction without needing a large amount of data.
            enable_wal: false,
            num_memtable_limit: 50, // This is a high limit for testing purposes to avoid triggering flushes during the test,
                                    // but in a real implementation, you would want to set this to a lower value (e.g., 4 or 8) to control memory usage and trigger flushes more frequently.
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
    next_sst_id: AtomicUsize,
    pub(crate) options: Arc<LsmStorageOptions>,
    //pub(crate) compaction_controller: CompactionController,
    //pub(crate) manifest: Option<Manifest>,
    //pub(crate) mvcc: Option<LsmMvccInner>,
    //pub(crate) compaction_filters: Arc<Mutex<Vec<CompactionFilter>>>,
}

impl LsmStorageInner {
    /// Start the storage engine by either loading an existing directory or creating a new one if the directory does
    pub(crate) fn open(path: impl AsRef<Path>, options: LsmStorageOptions) -> anyhow::Result<Self> {
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
            next_sst_id: AtomicUsize::new(1),
            options: options.into(),
        })
    }
    pub(crate) fn next_sst_id(&self) -> usize {
        self.next_sst_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        let size;
        {
            let guard = self.state.read();
            guard.memtable.put(key, value)?;
            size = guard.memtable.approximate_size();
        }

        self.try_freeze(size)?;
        Ok(())
    }

    fn try_freeze(&self, estimated_size: usize) -> Result<()> {
        if estimated_size >= self.options.target_sst_size {
            let state_lock = self.state_lock.lock();
            let guard = self.state.read();
            // the memtable could have already been frozen, check again to ensure we really need to freeze
            if guard.memtable.approximate_size() >= self.options.target_sst_size {
                drop(guard);
                self.force_freeze_memtable(&state_lock)?;
            }
        }
        Ok(())
    }
    /// Force freeze the current memtable to an immutable memtable
    pub fn force_freeze_memtable(&self, _state_lock_observer: &MutexGuard<'_, ()>) -> Result<()> {
        let memtable_id = self.next_sst_id();
        let memtable = Arc::new(MemTable::create(memtable_id));

        let old_memtable;
        {
            let mut guard = self.state.write();
            // Swap the current memtable with a new one.
            let mut snapshot = guard.as_ref().clone();
            old_memtable = std::mem::replace(&mut snapshot.memtable, memtable);
            // Add the memtable to the immutable memtables.
            snapshot.imm_memtables.insert(0, old_memtable.clone());
            // Update the snapshot.
            *guard = Arc::new(snapshot);
        }

        Ok(())
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
