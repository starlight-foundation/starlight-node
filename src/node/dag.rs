use petgraph::{
    graphmap::GraphMap,
    visit::{Dfs, Visitable, Walker},
    Directed, Incoming,
};
use std::{
    cmp::Ordering, collections::{HashMap, HashSet}, hash::Hash
};

use crate::{error, util::Error};

struct Node<V> {
    /// How many direct predecessors this node has
    height: u64,
    /// The value of the node
    value: V,
}

/// A directed acyclic graph.
pub struct Dag<K: Hash + Copy + Ord, V> {
    /// The graph
    graph: GraphMap<K, (), Directed>,
    /// All heads (nodes without children)
    heads: HashSet<K>,
    /// The head of the tallest chain of nodes
    longest_chain: Option<K>,
    /// Node values
    nodes: HashMap<K, Node<V>>
}

fn is_a_taller_than_b<K: Ord>(a: (u64, &K), b: (u64, &K)) -> bool {
    match a.0.cmp(&b.0) {
        Ordering::Greater => true,
        Ordering::Equal => a.1 > b.1,
        Ordering::Less => false,
    }
}

impl<K: Hash + Copy + Ord, V> Dag<K, V> {
    /// Creates a new Dag.
    pub fn new() -> Self {
        Self {
            graph: GraphMap::new(),
            heads: HashSet::new(),
            longest_chain: None,
            nodes: HashMap::new(),
        }
    }

    /// Get the head of the longest chain of nodes
    pub fn get_longest_chain(&self) -> Option<&V> {
        self.longest_chain.as_ref().map(|idx| &self.nodes[&idx].value)
    }

    /// Insert a node into the DAG, returning `true` if it did not already exist.
    /// If `prev` exists, it must reference a valid node in the DAG.
    /// Updates `self.longest_chain` if necessary.
    pub fn insert(&mut self, key: K, value: V, prev: Option<K>) -> Result<bool, Error> {
        if self.nodes.contains_key(&key) {
            return Ok(false);
        }

        self.graph.add_node(key.clone());
        let height = if let Some(prev_key) = prev {
            let prev_height = self.nodes.get(&prev_key).ok_or_else(|| error!("can't find prev node in DAG"))?.height;
            self.graph.add_edge(prev_key.clone(), key.clone(), ());
            self.heads.remove(&prev_key);
            prev_height + 1
        } else {
            0
        };


        self.nodes.insert(key.clone(), Node { height, value });
        self.heads.insert(key.clone());

        if self.longest_chain.is_none() || is_a_taller_than_b(
            (height, &key),
            self.longest_chain.as_ref().map(|key| {
                (self.nodes.get(key).unwrap().height, key)
            }).unwrap(),
        ) {
            self.longest_chain = Some(key.clone());
        }

        Ok(true)
    }

    /// Removes a node from the DAG.
    pub fn remove(&mut self, key: K) -> Result<(), Error> {
        if !self.graph.contains_node(key.clone()) {
            return Err(error!("can't find node in DAG"));
        }
        self.graph.remove_node(key.clone());
        self.nodes.remove(&key);
        self.heads.remove(&key);
        Ok(())
    }

    /// Iterate over the node denoted by `key`, and all its ancestors.
    /// Starts at `key` and works backwards.
    /// Returns `None` if the node denoted by `key` does not exist.
    pub fn iter_node_and_ancestors(&self, key: K) -> Option<impl Iterator<Item = &V>> {
        match self.graph.contains_node(key) {
            false => None,
            true => Some(std::iter::successors(Some(key), |&x| {
                self.graph.neighbors_directed(x, Incoming).next()
            }).filter_map(move |k| self.nodes.get(&k).map(|n| &n.value))),
        }
    }

    /// Iterate over the node denoted by `key`, and all its descendants.
    /// Starts at `key` and works forwards.
    /// Returns `None` if the node denoted by `key` does not exist.
    pub fn iter_node_and_descendants(&self, key: K) -> Option<impl Iterator<Item = &V>> {
        if !self.graph.contains_node(key.clone()) {
            return None;
        }
        let descendants = Dfs::new(&self.graph, key.clone())
            .iter(&self.graph)
            .filter_map(move |k| self.nodes.get(&k).map(|n| &n.value));
        Some(descendants)
    }

    /// Get the common ancestor of the two nodes denoted by `key1` and `key2`, or None if one does not exist.
    /// Returns `None` if either node denoted by `key1` or `key2` does not exist.
    pub fn get_common_ancestor(&self, mut k1: K, mut k2: K) -> Option<&V> {
        while k1 != k2 {
            if self.nodes.get(&k1)?.height > self.nodes.get(&k2)?.height {
                k1 = self.graph.neighbors_directed(k1, Incoming).next()?;
            } else {
                k2 = self.graph.neighbors_directed(k2, Incoming).next()?;
            }
        }

        Some(&self.nodes.get(&k1)?.value)
    }

    /// If the node denoted by `key` exists in the DAG, set the corresponding node as the "root" node.
    /// This removes all nodes that are not descendants of the node, or the node itself.
    pub fn set_root(&mut self, key: K) -> Result<(), Error> {
        if !self.graph.contains_node(key.clone()) {
            return Err(error!("can't find node in DAG"));
        }

        let descendants: HashSet<K> = Dfs::new(&self.graph, key.clone())
            .iter(&self.graph)
            .collect();
        let to_remove: Vec<K> = self.graph.nodes().filter(|x| !descendants.contains(x)).collect();
        for node in to_remove {
            self.graph.remove_node(node);
        }
        self.nodes.retain(|k, _| descendants.contains(k));
        self.heads.retain(|k| descendants.contains(k));

        if self.longest_chain.as_ref().map(|k| descendants.contains(k)) == Some(false) {
            self.longest_chain = self.heads
                .iter()
                .max_by(|key1, key2| {
                    match is_a_taller_than_b(
                        (self.nodes.get(key1).unwrap().height, key1),
                        (self.nodes.get(key2).unwrap().height, key2),
                    ) {
                        true => Ordering::Greater,
                        false => Ordering::Less
                    }
                })
                .map(|k| k.clone());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let dag: Dag<char, i32> = Dag::new();
        assert_eq!(dag.nodes.len(), 0);
        assert_eq!(dag.heads.len(), 0);
        assert_eq!(dag.get_longest_chain(), None);
    }

    #[test]
    fn test_insert() {
        let mut dag = Dag::new();

        // Insert a node without a parent
        assert_eq!(dag.insert('A', 1, None), Ok(true));
        assert_eq!(dag.nodes.len(), 1);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.get_longest_chain(), Some(&1));

        // Insert a node with a parent
        assert_eq!(
            dag.insert('B', 2, Some('A')),
            Ok(true)
        );
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.get_longest_chain(), Some(&2));

        // Insert a node with a non-existent parent
        assert!(dag
            .insert('C', 3, Some('D'))
            .is_err());

        // Insert a node that already exists
        assert_eq!(dag.insert('A', 4, None), Ok(false));
    }

    #[test]
    fn test_get_longest_chain() {
        let mut dag = Dag::new();

        assert!(dag.insert('A', 1, None).is_ok());
        assert!(dag
            .insert('B', 2, Some('A'))
            .is_ok());
        assert!(dag
            .insert('C', 3, Some('B'))
            .is_ok());
        assert!(dag
            .insert('D', 4, Some('A'))
            .is_ok());

        assert_eq!(dag.get_longest_chain(), Some(&3));
    }

    #[test]
    fn test_iter_node_and_ancestors() {
        let mut dag = Dag::new();

        assert!(dag.insert('A', 1, None).is_ok());
        assert!(dag
            .insert('B', 2, Some('A'))
            .is_ok());
        assert!(dag
            .insert('C', 3, Some('B'))
            .is_ok());
        assert!(dag
            .insert('D', 4, Some('A'))
            .is_ok());

        let ancestors: Vec<&i32> = dag
            .iter_node_and_ancestors('C')
            .unwrap()
            .collect();
        assert_eq!(ancestors, vec![&3, &2, &1]);

        assert!(dag.iter_node_and_ancestors('E').is_none());
    }

    #[test]
    fn test_iter_node_and_descendants() {
        let mut dag = Dag::new();

        assert!(dag.insert('A', 1, None).is_ok());
        assert!(dag
            .insert('B', 2, Some('A'))
            .is_ok());
        assert!(dag
            .insert('C', 3, Some('B'))
            .is_ok());
        assert!(dag
            .insert('D', 4, Some('A'))
            .is_ok());

        let descendants: Vec<&i32> = dag
            .iter_node_and_descendants('A')
            .unwrap()
            .collect();
        assert_eq!(descendants, vec![&1, &4, &2, &3]);

        assert!(dag.iter_node_and_descendants('E').is_none());
    }

    #[test]
    fn test_get_common_ancestor() {
        let mut dag = Dag::new();

        assert!(dag.insert('A', 1, None).is_ok());
        assert!(dag
            .insert('B', 2, Some('A'))
            .is_ok());
        assert!(dag
            .insert('C', 3, Some('B'))
            .is_ok());
        assert!(dag
            .insert('D', 4, Some('A'))
            .is_ok());

        assert_eq!(
            dag.get_common_ancestor('B', 'D'),
            Some(&1)
        );
        assert_eq!(
            dag.get_common_ancestor('C', 'D'),
            Some(&1)
        );
        assert_eq!(
            dag.get_common_ancestor('A', 'C'),
            Some(&1)
        );
        assert_eq!(
            dag.get_common_ancestor('A', 'A'),
            Some(&1)
        );

        assert!(dag
            .get_common_ancestor('A', 'E')
            .is_none());
        assert!(dag
            .get_common_ancestor('E', 'A')
            .is_none());
    }

    #[test]
    fn test_set_root() {
        let mut dag = Dag::new();

        assert!(dag.insert('A', 1, None).is_ok());
        assert!(dag
            .insert('B', 2, Some('A'))
            .is_ok());
        assert!(dag
            .insert('C', 3, Some('B'))
            .is_ok());
        assert!(dag
            .insert('D', 4, Some('A'))
            .is_ok());

        assert!(dag.set_root('B').is_ok());
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.get_longest_chain(), Some(&3));

        assert!(dag.set_root('E').is_err());
    }
}
