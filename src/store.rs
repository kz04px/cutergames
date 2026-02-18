// use std::cell::RefCell;
// use std::rc::Rc;
use std::collections::HashMap;

// std::rc::Rc<std::cell::RefCell<V>>

// struct RingBuffer<V, s: usize> {
//     data: [V; s],
// }

pub struct Store<K: Default, V> {
    // data: RingBuffer<V, 3>,
    data: Vec<V>,
    lut: HashMap<K, usize>,
    order: Vec<usize>,
    capacity: usize,
}

impl<K: Default, V> Store<K, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::<V>::with_capacity(capacity),
            lut: HashMap::new(),
            order: vec![],
            capacity,
        }
    }

    fn move_to_front(&mut self, idx: usize) {
        let _item = self.data.swap_remove(idx);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.data.len() == self.capacity
    }

    #[must_use]
    pub fn contains_key(&self, _key: K) -> bool {
        true
    }

    #[must_use]
    pub fn get(&mut self, _key: K) -> Option<V> {
        None
    }

    #[must_use]
    pub fn get_or_insert_with(&mut self, _key: K, _func: &dyn FnOnce() -> V) -> Option<V> {
        None
    }

    pub fn insert(&mut self, _key: K, _data: V) {
        if self.is_full() {
            // self.pop();
        }
        // self.data.push(data);
        // self.data.insert(key, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success() {
        let mut store = Store::<i32, String>::with_capacity(4);

        assert_eq!(store.len(), 0);
        assert_eq!(store.capacity(), 4);
        assert!(store.is_empty());
        assert!(!store.contains_key(0));
        assert!(!store.contains_key(1));
        assert_eq!(store.get(0), None);
        assert_eq!(store.get(1), None);

        store.insert(0, "Test String".to_string());

        assert_eq!(store.len(), 1);
        assert_eq!(store.capacity(), 4);
        assert!(!store.is_empty());
        assert!(store.contains_key(0));
        assert!(!store.contains_key(1));
        assert_eq!(store.get(0), Some("Test String".to_string()));
        assert_eq!(store.get(1), None);
    }
}
