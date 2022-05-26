#[derive(Debug, Default)]
pub struct SparseVec<V> {
    data: Vec<Option<V>>,
}

impl<V> SparseVec<V> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    #[inline]
    pub fn insert(&mut self, value: V, index: usize) {
        if self.data.len() <= index {
            self.data.resize_with(index + 1, || None);
        }
        self.data[index] = Some(value);
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.data.get(index).map(|v| v.is_some()).unwrap_or(false)
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&V> {
        self.data.get(index).and_then(|v| v.as_ref())
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.data.get_mut(index).and_then(|v| v.as_mut())
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<V> {
        self.data.get_mut(index).and_then(|value| value.take())
    }
}

#[derive(Debug, Default)]
pub struct SparseSet<V> {
    dense: Vec<V>,
    indices: Vec<usize>,
    sparse: SparseVec<usize>,
}

impl<V> SparseSet<V> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            indices: Vec::new(),
            sparse: SparseVec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
            sparse: SparseVec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    #[inline]
    pub fn insert(&mut self, value: V) -> usize {
        let new_index = self.dense.len();
        self.sparse.insert(new_index, new_index);
        self.indices.push(new_index);
        self.dense.push(value);
        new_index
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&V> {
        self.sparse.get(index).map(|dense_index| {
            // SAFETY: dense_index always exists
            unsafe { self.dense.get_unchecked(*dense_index) }
        })
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.sparse.get_mut(index).map(|dense_index| {
            // SAFETY: dense_index always exists
            unsafe { self.dense.get_unchecked_mut(*dense_index) }
        })
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.sparse.contains(index)
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        if let Some(dense_index) = self.sparse.remove(index) {
            let is_last = dense_index == self.dense.len() - 1;
            let val = self.dense.swap_remove(dense_index);
            self.indices.swap_remove(dense_index);
            if !is_last {
                let swapped_index = self.indices[dense_index];
                *self.sparse.get_mut(swapped_index).unwrap() = dense_index;
            }
            Some(val)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sparse_vec_insert() {
        let mut sv = SparseVec::with_capacity(10);
        assert_eq!(sv.capacity(), 10);

        sv.insert(0, 0);
        sv.insert(2, 2);
        sv.insert(5, 5);
        assert!(sv.contains(0));
        assert!(sv.contains(2));
        assert!(sv.contains(5));
    }

    #[test]
    fn sparse_vec_remove() {
        let mut sv = SparseVec::with_capacity(10);
        sv.insert(0, 0);
        sv.insert(2, 2);
        sv.insert(5, 5);

        assert!(sv.contains(0));
        assert!(sv.contains(2));
        assert!(sv.contains(5));
        assert!(!sv.contains(1));
        assert!(!sv.contains(3));
        assert!(!sv.contains(4));

        let removed = sv.remove(0);
        assert_eq!(removed, Some(0));
        assert!(!sv.contains(0));

        let removed = sv.remove(2);
        assert_eq!(removed, Some(2));
        assert!(!sv.contains(2));

        let removed = sv.remove(5);
        assert_eq!(removed, Some(5));
        assert!(!sv.contains(5));
    }

    #[test]
    fn sparse_vec_get() {
        let mut sv = SparseVec::with_capacity(10);
        sv.insert(0, 0);
        assert!(sv.contains(0));

        let i_0 = sv.get(0);
        assert_eq!(i_0, Some(&0));

        sv.remove(0);
        assert!(!sv.contains(0));

        let i_0 = sv.get(0);
        assert_eq!(i_0, None);
    }

    #[test]
    fn sparse_vec_get_mut() {
        let mut sv = SparseVec::with_capacity(10);
        sv.insert(0, 0);
        assert!(sv.contains(0));

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, Some(&mut 0));

        *i_0.unwrap() = 1;

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, Some(&mut 1));

        sv.remove(0);
        assert!(!sv.contains(0));

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, None);
    }

    #[test]
    fn sparse_set_insert() {
        let mut ss = SparseSet::with_capacity(10);
        assert_eq!(ss.capacity(), 10);

        let index_0 = ss.insert(0);
        let index_1 = ss.insert(1);
        let index_2 = ss.insert(2);

        assert_eq!(ss.len(), 3);
        assert!(!ss.is_empty());

        assert_eq!(index_0, 0);
        assert_eq!(index_1, 1);
        assert_eq!(index_2, 2);

        assert!(ss.contains(index_0));
        assert!(ss.contains(index_1));
        assert!(ss.contains(index_2));
    }

    #[test]
    fn sparse_set_remove() {
        let mut ss = SparseSet::with_capacity(10);

        let index_0 = ss.insert(0);
        let index_1 = ss.insert(1);
        let index_2 = ss.insert(2);

        ss.remove(index_0);
        assert!(!ss.contains(index_0));

        ss.remove(index_1);
        assert!(!ss.contains(index_1));

        ss.remove(index_2);
        assert!(!ss.contains(index_2));
    }

    #[test]
    fn sparse_set_get() {
        let mut sv = SparseSet::with_capacity(10);
        sv.insert(0);
        assert!(sv.contains(0));

        let i_0 = sv.get(0);
        assert_eq!(i_0, Some(&0));

        sv.remove(0);
        assert!(!sv.contains(0));

        let i_0 = sv.get(0);
        assert_eq!(i_0, None);
    }

    #[test]
    fn sparse_set_get_mut() {
        let mut sv = SparseSet::with_capacity(10);
        sv.insert(0);
        assert!(sv.contains(0));

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, Some(&mut 0));

        *i_0.unwrap() = 1;

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, Some(&mut 1));

        sv.remove(0);
        assert!(!sv.contains(0));

        let i_0 = sv.get_mut(0);
        assert_eq!(i_0, None);
    }

    #[test]
    fn sparse_set_insert_remove() {
        let mut ss = SparseSet::with_capacity(10);

        let indexes: Vec<usize> = (0..10).map(|i| ss.insert(i)).collect();
        assert_eq!(ss.len(), 10);

        for i in indexes.iter().step_by(2) {
            ss.remove(*i);
            assert!(!ss.contains(*i));
        }

        for i in indexes.iter().skip(1).step_by(2) {
            ss.remove(*i);
            assert!(!ss.contains(*i));
        }

        assert!(ss.is_empty());
    }
}
