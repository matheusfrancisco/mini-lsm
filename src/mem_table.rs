use std::sync::Arc;

use bytes::Bytes;
use crossbeam_skiplist::SkipMap;

use crate::skiplist::SkipList;

pub struct MemTable {
    map: Arc<SkipList<Bytes, Bytes>>, // for now I am using my custom skiplist, but I might switch to crossbeam's skiplist later for better performance and concurrency support.
    // map2: Arc<SkipMap<Bytes, Bytes>>,
    id: usize,
}

impl MemTable {
    pub fn create(id: usize) -> Self {
        Self {
            map: Arc::new(SkipList::new(4, Bytes::new(), Bytes::new())),
            //        map2: Arc::new(SkipMap::new()),
            id,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    //pub fn get_crossbeam(&self, key: &[u8]) -> Option<Bytes> {
    //    self.map2.get(key).map(|entry| entry.value().clone())
    //}
    /// Get a value by key.
    pub fn get(&self, key: &[u8]) -> Option<Bytes> {
        self.map.get_key_by_value(&Bytes::copy_from_slice(key))
    }

    pub fn for_testing_get_slice(&self, key: &[u8]) -> Option<Bytes> {
        self.get(key)
    }

    /// Put a key-value pair into the mem-table.
    ///
    /// This uses the custom skiplist implementation. The skiplist currently
    /// orders and searches by its `value` field, so we store `(value, key)`.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        let key = Bytes::copy_from_slice(key);
        let value = Bytes::copy_from_slice(value);

        let map = Arc::get_mut(&mut self.map)
            .ok_or_else(|| anyhow::anyhow!("Failed to get mutable reference to skiplist"))?;
        map.insert(value.clone(), key.clone());
        // Also insert into the crossbeam skiplist for testing purposes.
        //self.map2.insert(key, value);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    #[test]
    fn test_task1_memtable_get() {
        let mut memtable = super::MemTable::create(0);
        memtable.put(b"key1", b"value1").unwrap();
        memtable.put(b"key2", b"value2").unwrap();
        memtable.put(b"key3", b"value3").unwrap();
        let v = &memtable.for_testing_get_slice(b"key1").unwrap()[..];

        println!("string: value1, v: {}", std::str::from_utf8(v).unwrap());
        assert_eq!(b"value1", v);
        assert_eq!(
            &memtable.for_testing_get_slice(b"key2").unwrap()[..],
            b"value2"
        );
        assert_eq!(
            &memtable.for_testing_get_slice(b"key3").unwrap()[..],
            b"value3"
        );
    }
}
