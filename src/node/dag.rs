use petgraph::{csr::DefaultIx, graph::{Graph, NodeIndex}, visit::{Walker, Dfs}, Directed, Incoming};
use std::{collections::{HashSet, HashMap}, hash::Hash};

use crate::{error, util::Error};

struct Node<V> {
    /// The value of the node
    value: V,
    /// How many direct predecessors this node has
    height: u64,
}

/// A directed acyclic graph.
pub struct Dag<K: Hash + Eq + Clone, V> {
    /// The graph
    graph: Graph<Node<V>, (), Directed, DefaultIx>,
    /// Mapping of node keys to indices
    nodes: HashMap<K, NodeIndex<DefaultIx>>,
    /// All heads (nodes without children)
    heads: HashSet<NodeIndex<DefaultIx>>,
    /// The head of the tallest chain of nodes
    longest_chain: Option<NodeIndex<DefaultIx>>,
}

impl<K: Hash + Eq + Clone, V> Dag<K, V> {
    /// Creates a new Dag.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            nodes: HashMap::new(),
            heads: HashSet::new(),
            longest_chain: None,
        }
    }

    /// Get the node corresponding to the longest chain
    pub fn get_longest_chain(&self) -> Option<&V> {
        self.longest_chain
            .map(|idx| &self.graph[idx].value)
    }

    /// Insert a node into the DAG if it does not already exist.
    /// If `prev` exists, it must reference a valid node in the DAG.
    /// Updates `self.longest_chain` if necessary.
    pub fn try_insert(&mut self, key: K, value: V, prev: Option<K>) -> Result<(), Error> {
        if self.nodes.contains_key(&key) {
            return Err(error!("Node already exists in the DAG"));
        }

        let node_idx = self.graph.add_node(Node {
            value,
            height: 0,
        });

        if let Some(prev_key) = prev {
            let &prev_idx = self.nodes.get(&prev_key).ok_or_else(
                || error!("can't find prev node in DAG")
            )?;
            self.graph.add_edge(prev_idx, node_idx, ());
            self.graph[node_idx].height = self.graph[prev_idx].height + 1;
            self.heads.remove(&prev_idx);
        }

        self.nodes.insert(key.clone(), node_idx);
        self.heads.insert(node_idx);

        if self.longest_chain.is_none()
            || self.graph[node_idx].height > self.graph[self.longest_chain.unwrap()].height
        {
            self.longest_chain = Some(node_idx);
        }

        Ok(())
    }

    /// Iterate over the node denoted by `key`, and all its ancestors.
    /// Starts at `key` and works backwards.
    /// Returns `None` if the node denoted by `key` does not exist.
    pub fn try_iter_node_and_ancestors(&self, key: K) -> Option<impl Iterator<Item = &V>> {
        let &node_idx = self.nodes.get(&key)?;
        Some(std::iter::successors(Some(node_idx), |&x| {
            self.graph.neighbors_directed(x, Incoming).next()
        })
        .map(|idx| &self.graph[idx].value))
    }

    /// Iterate over the node denoted by `key`, and all its descendants.
    /// Starts at `key` and works forwards.
    /// Returns `None` if the node denoted by `key` does not exist.
    pub fn try_iter_node_and_descendants(&self, key: K) -> Option<impl Iterator<Item = &V>> {
        let node_idx = self.nodes.get(&key)?;
        Some(Dfs::new(&self.graph, *node_idx)
            .iter(&self.graph)
            .map(move |idx| &self.graph[idx].value))
    }

    /// Get the common ancestor of the two nodes denoted by `key1` and `key2`, or None if one does not exist.
    /// Returns `None` if either node denoted by `key1` or `key2` does not exist.
    pub fn get_common_ancestor(&self, key1: K, key2: K) -> Option<&V> {
        let mut idx1 = *self.nodes.get(&key1)?;
        let mut idx2 = *self.nodes.get(&key2)?;

        while idx1 != idx2 {
            if self.graph[idx1].height > self.graph[idx2].height {
                idx1 = self.graph.neighbors_directed(idx1, Incoming).next()?;
            } else {
                idx2 = self.graph.neighbors_directed(idx2, Incoming).next()?;
            }
        }

        Some(&self.graph[idx1].value)
    }

    /// If the node denoted by `key` exists in the DAG, set the corresponding node as the "root" node.
    /// This removes all nodes that are not descendants of the node, or the node itself.
    pub fn try_set_root(&mut self, key: K) -> bool {
        let node_idx = match self.nodes.get(&key) {
            Some(idx) => *idx,
            None => return false,
        };

        let mut descendants: HashSet<NodeIndex<DefaultIx>> = Dfs::new(&self.graph, node_idx)
            .iter(&self.graph)
            .collect();
        descendants.insert(node_idx);

        self.graph.retain_nodes(|_, idx| descendants.contains(&idx));
        self.nodes.retain(|_, idx| descendants.contains(idx));
        self.heads.retain(|idx| descendants.contains(idx));

        self.longest_chain = self.longest_chain.and_then(|idx| {
            if self.heads.contains(&idx) {
                Some(idx)
            } else {
                self.heads.iter().max_by_key(|idx| self.graph[**idx].height).cloned()
            }
        });

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let dag: Dag<String, i32> = Dag::new();
        assert_eq!(dag.nodes.len(), 0);
        assert_eq!(dag.heads.len(), 0);
        assert_eq!(dag.longest_chain, None);
    }

    #[test]
    fn test_try_insert() {
        let mut dag = Dag::new();

        // Insert a node without a parent
        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert_eq!(dag.nodes.len(), 1);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.longest_chain, Some(dag.nodes["A"]));

        // Insert a node with a parent
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.longest_chain, Some(dag.nodes["B"]));

        // Insert a node with a non-existent parent
        assert!(dag.try_insert("C".to_string(), 3, Some("D".to_string())).is_err());

        // Insert a node that already exists
        assert!(dag.try_insert("A".to_string(), 4, None).is_err());
    }

    #[test]
    fn test_get_longest_chain() {
        let mut dag = Dag::new();

        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert!(dag.try_insert("C".to_string(), 3, Some("B".to_string())).is_ok());
        assert!(dag.try_insert("D".to_string(), 4, Some("A".to_string())).is_ok());

        assert_eq!(dag.get_longest_chain(), Some(&3));
    }

    #[test]
    fn test_try_iter_node_and_ancestors() {
        let mut dag = Dag::new();

        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert!(dag.try_insert("C".to_string(), 3, Some("B".to_string())).is_ok());
        assert!(dag.try_insert("D".to_string(), 4, Some("A".to_string())).is_ok());

        let ancestors: Vec<&i32> = dag.try_iter_node_and_ancestors("C".to_string()).unwrap().collect();
        assert_eq!(ancestors, vec![&3, &2, &1]);

        assert!(dag.try_iter_node_and_ancestors("E".to_string()).is_none());
    }

    #[test]
    fn test_try_iter_node_and_descendants() {
        let mut dag = Dag::new();

        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert!(dag.try_insert("C".to_string(), 3, Some("B".to_string())).is_ok());
        assert!(dag.try_insert("D".to_string(), 4, Some("A".to_string())).is_ok());

        let descendants: Vec<&i32> = dag.try_iter_node_and_descendants("A".to_string()).unwrap().collect();
        assert_eq!(descendants, vec![&1, &2, &3, &4]);

        assert!(dag.try_iter_node_and_descendants("E".to_string()).is_none());
    }

    #[test]
    fn test_get_common_ancestor() {
        let mut dag = Dag::new();

        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert!(dag.try_insert("C".to_string(), 3, Some("B".to_string())).is_ok());
        assert!(dag.try_insert("D".to_string(), 4, Some("A".to_string())).is_ok());

        assert_eq!(dag.get_common_ancestor("B".to_string(), "D".to_string()), Some(&1));
        assert_eq!(dag.get_common_ancestor("C".to_string(), "D".to_string()), Some(&1));
        assert_eq!(dag.get_common_ancestor("A".to_string(), "C".to_string()), Some(&1));
        assert_eq!(dag.get_common_ancestor("A".to_string(), "A".to_string()), Some(&1));

        assert!(dag.get_common_ancestor("A".to_string(), "E".to_string()).is_none());
        assert!(dag.get_common_ancestor("E".to_string(), "A".to_string()).is_none());
    }

    #[test]
    fn test_try_set_root() {
        let mut dag = Dag::new();

        assert!(dag.try_insert("A".to_string(), 1, None).is_ok());
        assert!(dag.try_insert("B".to_string(), 2, Some("A".to_string())).is_ok());
        assert!(dag.try_insert("C".to_string(), 3, Some("B".to_string())).is_ok());
        assert!(dag.try_insert("D".to_string(), 4, Some("A".to_string())).is_ok());

        assert!(dag.try_set_root("B".to_string()));
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.heads.len(), 1);
        assert_eq!(dag.longest_chain, Some(dag.nodes["C"]));

        assert!(!dag.try_set_root("E".to_string()));
    }
}