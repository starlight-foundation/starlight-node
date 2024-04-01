use priority_queue::DoublePriorityQueue;
use std::{
    cmp::Ordering,
    fmt::Debug,
    hash::{Hash, Hasher},
};

// Struct to hold a key-value pair
struct KeyValue<K, V> {
    key: K,
    value: V,
}

#[derive(Debug)]
// Struct to hold a key-value pair along with its index in the list
struct KeyPriorityIndex<K: Ord, P: Ord> {
    key: K,
    priority: P,
    index: usize,
}

// Implement equality comparison for KeyPriorityIndex based on the key
impl<K: Ord, P: Ord> PartialEq for KeyPriorityIndex<K, P> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: Ord, P: Ord> Eq for KeyPriorityIndex<K, P> {}

// Implement ordering for KeyPriorityIndex based on the priority, then the key
impl<K: Ord, P: Ord> Ord for KeyPriorityIndex<K, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.key.cmp(&other.key),
            ord => ord,
        }
    }
}

impl<K: Ord, P: Ord> PartialOrd for KeyPriorityIndex<K, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Ord + Hash, P: Ord> Hash for KeyPriorityIndex<K, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

pub trait CenterMapValue<P: Ord> {
    fn priority(&self) -> P;
}

/// CenterMap struct to maintain a sorted list of key-value pairs around a center value
/// Elements are sorted first by priority, then by key
pub struct CenterMap<K: Hash + Eq + Clone + Ord, P: Ord, V: CenterMapValue<P>> {
    less: DoublePriorityQueue<K, KeyPriorityIndex<K, P>>, // Set of key-value pairs less than the center
    greater: DoublePriorityQueue<K, KeyPriorityIndex<K, P>>, // Set of key-value pairs greater than the center
    list: Vec<KeyValue<K, V>>,                               // List of all key-value pairs
    center: P,                                               // The center priority
    max_less: usize,    // Maximum number of elements less than the center
    max_greater: usize, // Maximum number of elements greater than the center
}

impl<K: Hash + Eq + Clone + Ord, P: Ord + Clone, V: CenterMapValue<P>> CenterMap<K, P, V> {
    pub fn new(center: P, max_less: usize, max_greater: usize) -> Self {
        Self {
            less: DoublePriorityQueue::new(),
            greater: DoublePriorityQueue::new(),
            list: Vec::new(),
            center,
            max_less,
            max_greater,
        }
    }

    // Insert a new key-value pair into the CenterMap
    pub fn insert(&mut self, key: K, value: V) -> bool {
        let priority = value.priority();
        if priority < self.center {
            // If the priority is less than the center
            if self.less.len() < self.max_less {
                // If there's still room on the "less" side
                let index = self.list.len();
                self.list.push(KeyValue {
                    key: key.clone(),
                    value,
                });
                self.less.push(
                    key.clone(),
                    KeyPriorityIndex {
                        key,
                        priority,
                        index,
                    },
                );
                true
            } else {
                // If the "less" side is full
                let (_, lowest) = self.less.peek_min().unwrap();
                if priority < lowest.priority {
                    // If the new priority is smaller than the smallest on the "less" side, don't insert it
                    return false;
                }
                // Remove the smallest priority from the "less" side
                let (_, lowest) = self.less.pop_min().unwrap();
                // Replace it with the new value in the list
                self.list[lowest.index].value = value;
                // Insert the new priority into the "less" set
                self.less.push(
                    key.clone(),
                    KeyPriorityIndex {
                        key,
                        priority,
                        index: lowest.index,
                    },
                );
                true
            }
        } else {
            // If the priority is greater than or equal to the center
            if self.greater.len() < self.max_greater {
                // If there's still room on the "greater" side
                let index = self.list.len();
                self.list.push(KeyValue {
                    key: key.clone(),
                    value,
                });
                self.greater.push(
                    key.clone(),
                    KeyPriorityIndex {
                        key,
                        priority,
                        index,
                    },
                );
                true
            } else {
                // If the "greater" side is full
                let (_, greatest) = self.greater.peek_max().unwrap();
                if priority > greatest.priority {
                    // If the new priority is larger than the largest on the "greater" side, don't insert it
                    return false;
                }
                // Remove the largest priority from the "greater" side
                let (_, greatest) = self.greater.pop_max().unwrap();
                // Replace it with the new value in the list
                self.list[greatest.index].value = value;
                // Insert the new priority into the "greater" set
                self.greater.push(
                    key.clone(),
                    KeyPriorityIndex {
                        key,
                        priority,
                        index: greatest.index,
                    },
                );
                true
            }
        }
    }

    fn update_index(&mut self, index: usize) {
        let kv = &self.list[index];
        let kpi = KeyPriorityIndex {
            key: kv.key.clone(),
            priority: kv.value.priority(),
            index,
        };
        // Insert into the appropriate set based on its priority relative to center
        if kv.value.priority() < self.center {
            self.less.change_priority(&kv.key, kpi);
        } else {
            self.greater.change_priority(&kv.key, kpi);
        }
    }

    pub fn remove_index(&mut self, index: usize) -> V {
        // Remove the KeyValue at the found index and return its value
        let KeyValue { key, value } = self.list.swap_remove(index);
        if value.priority() < self.center {
            self.less.remove(&key);
        } else {
            self.greater.remove(&key);
        }

        // If there's still an element at the removal index after swapping, update its index
        if index < self.list.len() {
            self.update_index(index);
        }
        value
    }

    // Remove a key-value pair from the CenterMap by key and return its value
    pub fn remove(&mut self, key: K) -> Option<V> {
        // Try to find the KeyPriorityIndex in the "less" set, then the "greater" set
        let KeyPriorityIndex { index, .. } = match self.less.remove(&key) {
            Some((_, kpi)) => Some(kpi),
            None => match self.greater.remove(&key) {
                Some((_, kpi)) => Some(kpi),
                None => None,
            },
        }?; // Return None if not found in either set

        // Remove the KeyValue at the found index and return its value
        let KeyValue { value, .. } = self.list.swap_remove(index);

        // If there's still an element at the removal index after swapping, update its index
        if index < self.list.len() {
            self.update_index(index);
        }

        Some(value)
    }

    pub fn len(&self) -> usize {
        return self.list.len();
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some((_, kpi)) = self.less.get(key) {
            Some(&self.list[kpi.index].value)
        } else if let Some((_, kpi)) = self.greater.get(key) {
            Some(&self.list[kpi.index].value)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if let Some((_, kpi)) = self.less.get(key) {
            Some(&mut self.list[kpi.index].value)
        } else if let Some((_, kpi)) = self.greater.get(key) {
            Some(&mut self.list[kpi.index].value)
        } else {
            None
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn clear(&mut self) {
        self.list.clear();
        self.less.clear();
        self.greater.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.list.iter().map(|kv| (&kv.key, &kv.value))
    }

    pub fn update_center(&mut self, new_center: P) {
        if new_center == self.center {
            return;
        }

        self.center = new_center;

        // Move elements from the "less" set to the "greater" set if their priority is greater than or equal to the new center
        while let Some((_, kpi)) = self.less.peek_max() {
            if kpi.priority >= self.center {
                let (key, kpi) = self.less.pop_max().unwrap();
                self.greater.push(key, kpi);
            } else {
                break;
            }
        }

        // Move elements from the "greater" set to the "less" set if their priority is less than the new center
        while let Some((_, kpi)) = self.greater.peek_min() {
            if kpi.priority < self.center {
                let (key, kpi) = self.greater.pop_min().unwrap();
                self.less.push(key, kpi);
            } else {
                break;
            }
        }

        // Trim the "less" set if it exceeds the maximum side length
        while self.less.len() > self.max_less {
            let (_, lowest) = self.less.pop_min().unwrap();
            self.list.swap_remove(lowest.index);
            if lowest.index < self.list.len() {
                self.update_index(lowest.index);
            }
        }

        // Trim the "greater" set if it exceeds the maximum side length
        while self.greater.len() > self.max_greater {
            let (_, greatest) = self.greater.pop_max().unwrap();
            self.list.swap_remove(greatest.index);
            if greatest.index < self.list.len() {
                self.update_index(greatest.index);
            }
        }
    }
}

impl<K: Hash + Eq + Clone + Ord + Debug, P: Ord, V: CenterMapValue<P> + Debug> std::fmt::Debug
    for CenterMap<K, P, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.list.iter().map(|kv| (&kv.key, &kv.value)))
            .finish()
    }
}

impl<K: Hash + Eq + Clone + Ord, P: Ord, V: CenterMapValue<P>> std::ops::Index<usize>
    for CenterMap<K, P, V>
{
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index].value
    }
}

impl<K: Hash + Eq + Clone + Ord, P: Ord, V: CenterMapValue<P>> std::ops::IndexMut<usize>
    for CenterMap<K, P, V>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index].value
    }
}

impl<P: Ord + Clone> CenterMapValue<P> for P {
    fn priority(&self) -> P {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut map = CenterMap::new(0, 2, 2);
        map.insert(1, 10);
        map.insert(2, 20);
        map.insert(3, 30);
        map.insert(4, -10);
        map.insert(5, -20);

        assert_eq!(map.get(&1), Some(&10));
        assert_eq!(map.get(&2), Some(&20));
        assert_eq!(map.get(&3), None);
        assert_eq!(map.get(&4), Some(&-10));
        assert_eq!(map.get(&5), Some(&-20));

        assert_eq!(map.list.len(), 4);
        assert_eq!(map.less.len(), 2);
        assert_eq!(map.greater.len(), 2);
    }

    #[test]
    fn test_contains() {
        let mut map = CenterMap::new(0, 2, 2);
        map.insert(1, 10);
        map.insert(2, 20);

        assert!(map.contains(&1));
        assert!(map.contains(&2));
        assert!(!map.contains(&3));
    }

    #[test]
    fn test_get() {
        let mut map = CenterMap::new(0, 2, 2);
        map.insert(1, 10);
        map.insert(2, 20);

        assert_eq!(map.get(&1), Some(&10));
        assert_eq!(map.get(&2), Some(&20));
        assert_eq!(map.get(&3), None);
    }

    #[test]
    fn test_is_empty() {
        let mut map: CenterMap<i32, i32, i32> = CenterMap::new(0, 2, 2);
        assert!(map.is_empty());
        map.insert(1, 10);
        assert!(!map.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut map = CenterMap::new(0, 2, 2);
        map.insert(1, 10);
        map.insert(2, 20);
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.list.len(), 0);
        assert_eq!(map.less.len(), 0);
        assert_eq!(map.greater.len(), 0);
    }

    #[test]
    fn test_index() {
        let mut map = CenterMap::new(0, 2, 2);
        map.insert(1, 10);
        map.insert(2, 20);

        assert_eq!(map[0], 10);
        assert_eq!(map[1], 20);
    }
}
