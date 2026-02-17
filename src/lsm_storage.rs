use std::{
    collections::HashMap,
    os::macos::raw::stat,
    path::{Path, PathBuf},
    sync::Arc,
};

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

impl LsmStorageState {
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        // TOOO: In a real implementation, you would need to check the memtable size and flush
        // it to an SSTable when it exceeds a certain threshold.
        // For simplicity, we will not implement memtable flushing
        // and compaction in this example, at this point
        // we will do this latter on..
        //
        Arc::get_mut(&mut self.memtable).unwrap().put(key, value)
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

        //to keep this I will need to make my
        //skiplist insert use self instead mut self
        //or remove the arc arround
        //    guard.put(key, value)
        Ok(())
    }
}
