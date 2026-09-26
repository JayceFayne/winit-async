use smallvec::SmallVec;

pub struct Map<K, V> {
    inner: SmallVec<[(K, V); 8]>,
}

impl<K, V> Map<K, V> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: SmallVec::new_const(),
        }
    }
}

impl<K, V> Map<K, V>
where
    K: PartialEq,
{
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.iter().map(|e| &e.0).any(|id| id == key)
    }

    fn find(&self, key: &K) -> Option<usize> {
        self.inner
            .iter()
            .map(|e| &e.0)
            .enumerate()
            .find(|(_, id)| *id == key)
            .map(|e| e.0)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let pos = self.find(key)?;
        self.inner.get(pos).map(|e| &e.1)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let pos = self.find(key)?;
        Some(self.inner.remove(pos).1)
    }

    pub fn push(&mut self, key: K, value: V) {
        self.inner.push((key, value));
    }
}
