//! Thread-safe key/value cache.

use std::collections::hash_map::{Entry, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

/// Cache that remembers the result for each key.
#[derive(Debug)]
pub struct Cache<K, V> {
    // todo! This is an example cache type. Build your own cache type that satisfies the
    // specification for `get_or_insert_with`.
    inner: RwLock<HashMap<K, Arc<Mutex<Option<V>>>>>,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// Retrieve the value or insert a new one created by `f`.
    ///
    /// An invocation to this function should not block another invocation with a different key. For
    /// example, if a thread calls `get_or_insert_with(key1, f1)` and another thread calls
    /// `get_or_insert_with(key2, f2)` (`key1≠key2`, `key1,key2∉cache`) concurrently, `f1` and `f2`
    /// should run concurrently.
    ///
    /// On the other hand, since `f` may consume a lot of resource (= money), it's undesirable to
    /// duplicate the work. That is, `f` should be run only once for each key. Specifically, even
    /// for concurrent invocations of `get_or_insert_with(key, f)`, `f` is called only once per key.
    ///
    /// Hint: the [`Entry`] API may be useful in implementing this function.
    ///
    /// [`Entry`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.entry
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        loop {
            {
                let inner_read = self.inner.read().unwrap();
                if let Some(value) = inner_read.get(&key) {
                    let value = value.lock().unwrap();
                    if let Some(ref value) = *value {
                        return value.clone();
                    }
                    continue;
                }
            }

            let value_arc = {
                let mut inner_write = self.inner.write().unwrap();
                match inner_write.entry(key.clone()) {
                    Entry::Occupied(entry) => {
                        entry.get().clone()
                    }
                    Entry::Vacant(entry) => {
                        let value_arc = Arc::new(Mutex::new(None));
                        entry.insert(Arc::clone(&value_arc));
                        value_arc
                    }
                }
            };

            if let Some(ref value) = *value_arc.lock().unwrap() {
                return value.clone();
            }

            let new_value = f(key.clone());
            let mut value = value_arc.lock().unwrap();
            *value = Some(new_value.clone());
            return new_value;

        }
    }

}
