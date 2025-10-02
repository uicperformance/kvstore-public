// tree.rs
// Simple ordered tree map implementation with persistence support.
// Requires in Cargo.toml:
// serde = { version = "1.0", features = ["derive"] }
// bincode = "1.3"

use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::borrow::Borrow;

use serde::{Serialize, Deserialize, de::DeserializeOwned};
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned")]
pub struct TreeMap<K: Ord + Clone, V: Clone> {
    root: Option<Box<Node<K, V>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned")]
struct Node<K: Ord + Clone, V: Clone> {
    key: K,
    value: V,
    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}
impl<K: Ord + Clone + Serialize + DeserializeOwned, V: Clone + Serialize + DeserializeOwned> TreeMap<K, V> {
    pub fn new() -> Self { TreeMap { root: None } }

    #[inline(never)]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        Self::insert_node(&mut self.root, key, value)
    }

    fn insert_node(node: &mut Option<Box<Node<K, V>>>, key: K, value: V) -> Option<V> {
        match node {
            Some(n) => match key.cmp(&n.key) {
                Ordering::Less    => Self::insert_node(&mut n.left, key, value),
                Ordering::Greater => Self::insert_node(&mut n.right, key, value),
                Ordering::Equal   => { let old = n.value.clone(); n.value = value; Some(old) }
            },
            None => {
                *node = Some(Box::new(Node { key: key, value: value, left: None, right: None }));
                None
            }
        }
    }

    #[inline(never)]
    pub fn get(&self, key: &K) -> Option<V> { Self::get_node(&self.root, key) }
    fn get_node(node: &Option<Box<Node<K, V>>>, key: &K) -> Option<V> {
        match node {
            Some(n) => match key.borrow().cmp(&n.key) {
                Ordering::Less    => Self::get_node(&n.left, key),
                Ordering::Greater => Self::get_node(&n.right, key),
                Ordering::Equal   => Some(n.value.clone()),
            },
            None => None,
        }
    }

    #[inline(never)]
    pub fn remove(&mut self, key: &K) -> Option<V> { Self::remove_node(&mut self.root, key) }
    fn remove_node(node: &mut Option<Box<Node<K, V>>>, key: &K) -> Option<V> {
        if let Some(n) = node {
            match key.cmp(&n.key) {
                Ordering::Less    => return Self::remove_node(&mut n.left, key),
                Ordering::Greater => return Self::remove_node(&mut n.right, key),
                Ordering::Equal   => {
                    let removed = n.value.clone();
                    match (n.left.take(), n.right.take()) {
                        (None, None)   => *node = None,
                        (Some(l), None) => *node = Some(l),
                        (None, Some(r)) => *node = Some(r),
                        (Some(l), Some(r)) => {
                            let (min_k, min_v) = {
                                let mut cur = r.as_ref(); 
                                while let Some(lc) = cur.left.as_ref() { 
                                    cur = lc; 
                                }
                                (cur.key.clone(), cur.value.clone())
                            };
                            n.key = min_k;
                            n.value = min_v;
                            n.left = Some(l);
                            n.right = Some(Self::remove_min(r));
                        }
                    }
                    return Some(removed);
                }
            }
        }
        None
    }
    fn remove_min(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        if node.left.is_none() {
            return node.right.take().unwrap();
        }
        node.left = Some(Self::remove_min(node.left.take().unwrap()));
        node
    }

    pub fn seek_ge(&self, key: &K) -> Option<(K, V)> {
        let mut res = None;
        let mut cur = &self.root;
        while let Some(n) = cur {
            if &n.key < key { cur = &n.right; }
            else { res = Some((n.key.clone(), n.value.clone())); cur = &n.left; }
        }
        res
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut f = File::create(path)?;
        let data = bincode::serialize(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        f.write_all(&data)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut buf = Vec::new(); f.read_to_end(&mut buf)?;
        let map = bincode::deserialize(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(map)
    }
}
