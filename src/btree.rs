use std::ops::{Bound, RangeBounds};
use std::fmt::Debug;
//use std::cmp::Ordering;
use std::borrow::Borrow;

#[derive(Debug)]
enum Node<K: Ord + Clone + Debug + AsRef<str> , V: Clone + Debug, const M: usize> where [(); M - 1]:{
    Leaf {
        keys: [Option<K>; M - 1],
        values: [Option<V>; M - 1],
    },
    Internal {
        keys: [Option<K>; M - 1],
        children: [Option<Box<Node<K, V, M>>>; M],
    },
}

#[derive(Debug)]
pub struct Stats {
    pub size: usize,
    pub depth: usize
}

enum QueryKind {
    // None if no exact match with key
    Point,

    // Seek: index of smallest key greater than query key.
    // [2,4] seek 3 -> 1
    // [2,4] seek 1 -> 0
    // [2,4] seek 5 -> None
    Seek,

    // Insert: index where the key belongs, in sorted order.
    // This is the same as seek, except where Seek returns None, 
    // insert returns Some(index_after_last) if there is room
    Insert,

    // Same as Insert, just for clarity of code 
    Child
}

const fn constnone<K>() -> Option<K> {
    None
}

impl<K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> Node<K, V, M> where [(); M - 1]: {
    fn new_leaf() -> Box<Self> {
        Self::new_leaf_with([const { constnone() }; M - 1], [const { constnone() }; M - 1])
    }
    fn new_leaf_with(keys: [Option<K>; M - 1], vals: [Option<V>; M - 1]) -> Box<Self> {
        Box::new(Node::Leaf {
            keys,
            values: vals,
        })
    }
    fn new_internal() -> Box<Self> {
        Self::new_internal_with([const { constnone() }; M - 1], [const{ constnone()}; M])
    }
    fn new_internal_with(keys: [Option<K>; M - 1], children: [Option<Box<Node<K, V, M>>>; M]) -> Box<Self> {
        Box::new(Node::Internal {
            keys,
            children,
        })
    }
    fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf { .. })
    }
    fn num_keys(&self) -> usize {
        match self {
            Node::Leaf { keys, .. } | Node::Internal { keys, .. } => keys.iter().filter(|k| k.is_some()).count(),
        }
    }
    fn depth(&self) -> usize {
        match self {
            Node::Leaf { .. } => 1,
            Node::Internal { children, .. } => {
                children.iter()
                    .filter_map(|c| c.as_ref().map(|child| child.depth()))
                    .reduce(|max, one| if max > one + 1 { max } else { one + 1 })
                    .unwrap_or(1)
            }
        }
    }
    fn size(&self) -> usize {
        match self {
            Node::Leaf { .. } => self.num_keys(),
            Node::Internal { children ,.. } => {
                children.iter().filter_map(|c| c.as_ref().map(|child| child.size())).reduce(|sum, one| sum+one).unwrap()
            }
        }
    }

    fn is_full(&self) -> bool {
        // this assumes all the None elements are at the end. That's not true in a good solution. 
        match self {
            Node::Leaf{keys,..} | Node::Internal{keys,..} => 
              keys[M-2].is_some()
        }
//        self.num_keys() == M - 1
    }

    // returns the index of the smallest key that is greater than or equal to the search key
    #[inline(never)]
    fn find_key_index<B: ?Sized>(&self, key: &B, kind: QueryKind) -> Option<usize> where K: Borrow<B>, B: PartialOrd<B>, B: PartialEq<B>, B: Ord  {
        self.find_key_index_linear(key, kind, 0)
//        self.find_key_index_binary(key, false)
    }

    fn find_key_index_linear<B: ?Sized>(&self, key: &B, kind: QueryKind, from_index: usize) -> Option<usize> where K: Borrow<B>, B: PartialOrd<B>, B: PartialEq<B>, B: Ord  {
        let keys = match self {
            Node::Leaf { keys, .. } | Node::Internal { keys, .. } => keys,
        };
        let keycount = keys.len();
  
        for k in from_index..keycount {
            match &keys[k] {
                Some(array_key) if Borrow::<B>::borrow(array_key) > key  => {
                    return match kind {
                        QueryKind::Point => None,
                        QueryKind::Seek => Some(k),
                        QueryKind::Insert | QueryKind::Child => Some(k)
                    }
                },
                Some(array_key) if Borrow::<B>::borrow(array_key) == key => { 
                    return match kind {
                        QueryKind::Point | QueryKind::Seek | QueryKind::Insert => Some(k),
                        QueryKind::Child => Some(k+1)
                    }
                },
                None => {
                    return match kind {
                        QueryKind::Point | QueryKind::Seek => None,                        
                        QueryKind::Insert | QueryKind::Child => Some(k)
                    }
                },
                _ => {}
            }
        }

        match kind {
            QueryKind::Point => None,
            QueryKind::Seek | QueryKind::Insert => None,
            QueryKind::Child => Some(keycount)
        }
    }

    // fn find_key_index_binary<B: ?Sized>(&self, key: &B, debug: bool) -> Option<usize> where K: Borrow<B>, B: Ord {
    //     let keys = match self {
    //         Node::Leaf { keys, .. } | Node::Internal { keys, .. } => keys,
    //     };
    //     if self.num_keys() == 0 {
    //         return None;
    //     }
    //     let mut k = if self.num_keys() == 1 { 0 } else { self.num_keys() / 2 };
    //     let mut interval = (self.num_keys() + 1) / 2;
    //     let index = loop {
    //         if debug {
    //             println!("Index {} interval {}", k, interval);
    //         }
    //         if k >= keys.len() {
    //             break keys.len() - 1;
    //         }
    //         match &keys[k] {
    //             Some(array_key) => {
    //                 match Borrow::<B>::borrow(array_key).cmp(key) {
    //                     Ordering::Equal => break k,
    //                     Ordering::Greater => {
    //                         if interval > 1 {
    //                             k -= (interval + 1) / 2;
    //                         } else if interval == 1 && k > 0 {
    //                             k -= 1;
    //                         } else {
    //                             if k == 0 {
    //                                 return None;
    //                             } else {
    //                                 break k;
    //                             }
    //                         }
    //                     }
    //                     Ordering::Less => {
    //                         if interval > 1 {
    //                             k += (interval + 1) / 2;
    //                         } else if interval == 1 && k + 1 < keys.len() {
    //                             k += 1;
    //                         } else {
    //                             break k;
    //                         }
    //                     }
    //                 }
    //             }
    //             None => break k,
    //         }
    //         interval /= 2;
    //     };
    //     Some(index)
    // }

    // fn has_key<B: ?Sized>(&self, key: &B) -> bool where K: Borrow<B>, B: Ord   {
    //     self.find_key_index(key, QueryKind::Point).is_some()
    // }

    fn get_child_for_key<B: ?Sized>(&self, key: &B) -> &Box<Node<K, V, M>> where K: Borrow<B>, B: Ord  {
        let index = self.find_key_index(key, QueryKind::Child).expect("Child query should always return a number.");
        if let Node::Internal { children, .. } = self {
            children[index].as_ref().expect("Child must exist")
        } else {
            unreachable!();
        }
    }

    fn get_value<B: ?Sized>(&self, key: &B) -> Option<&V> where K: Borrow<B>, B: Ord {
        if !self.is_leaf() {
            return None;
        }
        if let Some(index) = self.find_key_index(key,QueryKind::Point) {
            match self {
                Node::Leaf { keys: _, values, .. } => {
                    values[index].as_ref()
                }
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    fn insert_key_value(&mut self, key: K, value: V) {
        if !self.is_leaf() {
            panic!("Cannot insert key-value into Internal node");
        }
        if self.is_full() {
            panic!("Node is full");
        }
        let index = self.find_key_index(&key,QueryKind::Insert).unwrap_or(0);
        self.insert_key_value_atindex(key,value,index);
    }

    fn insert_key_value_atindex(&mut self, key: K, value: V, mut index: usize)  {
        match self {
            Node::Leaf { keys, values, .. } => {
                if keys[index].is_some() && keys[index].as_ref().unwrap() < &key { index+=1;}

                let mut insert_pos = index;
                while insert_pos < keys.len() && keys[insert_pos].is_some() {
                    insert_pos += 1;
                }
                if insert_pos >= keys.len() {
                    panic!("No space to insert key-value");
                }
                for i in (index..insert_pos).rev() {
                    keys[i + 1] = keys[i].take();
                    values[i + 1] = values[i].take();
                }
                keys[index] = Some(key);
                values[index] = Some(value);
            }
            _ => unreachable!(),
        }
    }

    fn insert_key(&mut self, index: usize, key: K) -> Result<(), &'static str> {
        if self.is_full() {
            panic!("Node is full");
        }
        match self {
            Node::Leaf { keys, .. } | Node::Internal { keys, .. } => {
                let mut insert_pos = index;
                while insert_pos < keys.len() && keys[insert_pos].is_some() {
                    insert_pos += 1;
                }
                if insert_pos >= keys.len() {
                    panic!("No space to insert key");
                }
                for i in (index..insert_pos).rev() {
                    keys[i + 1] = keys[i].take();
                }
                keys[index] = Some(key);
                Ok(())
            }
        }
    }
    fn insert_child(&mut self, index: usize, child: Box<Node<K, V, M>>) -> Result<(), &'static str> {
        if self.is_leaf() {
            return Err("Cannot insert child into Leaf node");
        }
        if self.num_keys() + 1 >= M {
            panic!("Internal node cannot accept more children");
        }
        match self {
            Node::Internal { children, .. } => {
                let mut insert_pos = index;
                while insert_pos < children.len() && children[insert_pos].is_some() {
                    insert_pos += 1;
                }
                if insert_pos >= children.len() {
                    panic!("No space to insert child");
                }
                for i in (index..insert_pos).rev() {
                    children[i + 1] = children[i].take();
                }
                children[index] = Some(child);
                Ok(())
            }
            _ => unreachable!(),
        }
    }
    fn split(&mut self, median_index: usize) -> Result<(K, Box<Node<K, V, M>>), &'static str> {
        if !self.is_full() {
            return Err("Node is not full");
        }
        match self {
            Node::Leaf { keys, values, .. } => {
                let median_key = keys[median_index].clone().expect("Median key must exist");
                let mut new_keys = [const { constnone() }; M - 1];
                let mut new_vals = [const { constnone() }; M - 1];
                for i in 0..(keys.len() - median_index) {
                    new_keys[i] = keys[median_index + i].take();
                    new_vals[i] = values[median_index + i].take();
                }
                Ok((median_key, Node::new_leaf_with(new_keys, new_vals)))
            }
            Node::Internal { keys, children, .. } => {
                let median_key = keys[median_index].take().expect("Median key must exist");
                let mut new_keys = [const { constnone()}; M - 1];
                let mut new_children = [const { constnone()}; M];
                for i in 0..(keys.len() - median_index-1) {
                    new_keys[i] = keys[median_index + i+1].take();
                }
                for i in 0..(children.len() - median_index - 1) {
                    new_children[i] = children[median_index + 1 + i].take();
                }
                Ok((median_key, Node::new_internal_with(new_keys, new_children)))
            }
        }
    }
    fn get_child(&self, index: usize) -> Option<&Node<K, V, M>> {
        match self {
            Node::Internal { children, .. } => children[index].as_ref().map(|c| c.as_ref()),
            Node::Leaf { .. } => None,
        }
    }
    // fn get_child_mut(&mut self, index: usize) -> Option<&mut Node<K, V, M>> {
    //     match self {
    //         Node::Internal { children, .. } => children[index].as_mut().map(|c| c.as_mut()),
    //         Node::Leaf { .. } => None,
    //     }
    // }
    
    fn check(&self) {
        match self {
            Node::Leaf { keys, values, .. } => {
                let key_count = keys.iter().filter(|k| k.is_some()).count();
                let value_count = values.iter().filter(|v| v.is_some()).count();
                debug_assert_eq!(key_count, value_count);
            }
            Node::Internal { keys, children, .. } => {
                let key_count = keys.iter().filter(|k| k.is_some()).count();
                let child_count = children.iter().filter(|c| c.is_some()).count();
                debug_assert_eq!(key_count + 1, child_count);
                let some_keys: Vec<&K> = keys.iter().filter_map(|k| k.as_ref()).collect();
                for i in 0..some_keys.len() - 1 {
                    debug_assert!(some_keys[i] < some_keys[i + 1], "keys are {:?}", some_keys);
                }
                for i in 0..some_keys.len() {
                    match &**children[i + 1].as_ref().expect("Child must exist") {
                        Node::Leaf { keys: child_keys, .. } => {
                            let child_some_keys: Vec<&K> = child_keys.iter().filter_map(|k| k.as_ref()).collect();
                            debug_assert_eq!(some_keys[i], child_some_keys[0]);
                            for ck in child_some_keys {
                                debug_assert!(some_keys[i] <= ck, "Child has a smaller key than I {:?}", some_keys[i]);
                            }
                        }
                        Node::Internal { keys: child_keys, .. } => {
                            let child_some_keys: Vec<&K> = child_keys.iter().filter_map(|k| k.as_ref()).collect();
                            for ck in child_some_keys {
                                debug_assert!(some_keys[i] < ck);
                            }
                        }
                    }
                }
                for child in children.iter().filter_map(|c| c.as_ref()) {
                    child.check();
                }
            }
        }
    }
    fn insert_recursive(&mut self, key: K, value: V) -> Option<(K, Box<Node<K, V, M>>)> {
        let index = match self {
            Node::Leaf{..} => self.find_key_index(&key,QueryKind::Insert),
            Node::Internal{..} => self.find_key_index(&key,QueryKind::Child)
        };

        // match self {
        //     Node::Leaf{keys, ..} | Node::Internal{keys,..} => {
        //         println!("\nrecursive insert key {:?} at index {:?} in {:?}",&key, index, keys);
        //     }
        // }

        if let Node::Leaf { keys, values, .. } = self {                    
            if let Some(index)=index {
                if keys[index].is_some() && keys[index].as_ref().unwrap() == &key {
                    values[index] = Some(value);
                    return None;
                }
            }
        }

        if self.is_full() {
            let t = (M + 1) / 2;
            match self {
                Node::Leaf { .. } => {
                    let (median_key, mut new_sibling) = self.split(t).unwrap();
                    if key >= median_key {
                        new_sibling.insert_key_value(key, value);
                    } else {
                        self.insert_key_value_atindex(key,value,index.unwrap());
/*                        let Node::Leaf { keys, values, .. } = self else { panic!("Not a leaf?"); };
                        let mut insert_pos = t;
                        while insert_pos < keys.len() && keys[insert_pos].is_some() {
                            insert_pos += 1;
                        }
                        if insert_pos >= keys.len() {
                            panic!("No space to insert key-value");
                        }
                        for i in (index..insert_pos).rev() {
                            keys[i + 1] = keys[i].take();
                            values[i + 1] = values[i].take();
                        }
                        keys[index] = Some(key);
                        values[index] = Some(value);*/
                    }
                    Some((median_key, new_sibling))
                }
                Node::Internal { keys: _, children, .. } => {
//                    let some_keys: Vec<&K> = keys.iter().filter_map(|k| k.as_ref()).collect();
  //                  debug_assert!(some_keys.len() + 1 == children.iter().filter(|c| c.is_some()).count());
                    let index=index.unwrap();
                    if let Some((median_child_key, new_child)) = children[index].as_mut().expect("Child must exist").insert_recursive(key, value) {
                        let (median_key, mut new_sibling) = self.split(t - 1).unwrap();
                        if median_child_key >= median_key {
                            let index = new_sibling.find_key_index(&median_child_key,QueryKind::Insert).expect("Insert query should always return integer.");
                            new_sibling.insert_key(index, median_child_key).unwrap();
                            new_sibling.insert_child(index + 1, new_child).unwrap();
                        } else {
                            let index = self.find_key_index(&median_child_key,QueryKind::Insert).expect("Insert query should always return integer.");
                            let Node::Internal { keys, children, .. } = self else { panic!("Not internal?") };
                            debug_assert!(keys.iter().filter(|k| k.is_some()).count() + 1 == children.iter().filter(|c| c.is_some()).count());
                            let mut insert_pos = index;
                            while insert_pos < keys.len() && keys[insert_pos].is_some() {
                                insert_pos += 1;
                            }
                            if insert_pos >= keys.len() {
                                panic!("No space to insert key");
                            }
                            for i in (index..insert_pos).rev() {
                                keys[i + 1] = keys[i].take();
                            }
                            keys[index] = Some(median_child_key);
                            let mut child_insert_pos = index + 1;
                            while child_insert_pos < children.len() && children[child_insert_pos].is_some() {
                                child_insert_pos += 1;
                            }
                            if child_insert_pos >= children.len() {
                                panic!("No space to insert child");
                            }
                            for i in (index + 1..child_insert_pos).rev() {
                                children[i + 1] = children[i].take();
                            }
                            children[index + 1] = Some(new_child);
                        }
                        Some((median_key, new_sibling))
                    } else {
                        None
                    }
                }
            }
        } else {
            let index=index.unwrap();
            match self {
                Node::Leaf { .. } => {
                    self.insert_key_value_atindex(key, value, index);
                }
                Node::Internal { keys, children, .. } => {
                    debug_assert_eq!(keys.iter().filter(|k| k.is_some()).count() + 1, children.iter().filter(|c| c.is_some()).count());
                    if let Some((median_child_key, new_child)) = children[index].as_mut().unwrap().insert_recursive(key, value) {
                        let child_index = keys.iter().enumerate().find(|(_, k)| k.is_none() || k.as_ref().unwrap() > &median_child_key).map(|(i, _)| i).unwrap();
                        let mut insert_pos = child_index;
                        while insert_pos < keys.len() && keys[insert_pos].is_some() {
                            insert_pos += 1;
                        }
                        if insert_pos >= keys.len() {
                            panic!("No space to insert key {median_child_key:?} at {insert_pos} in {keys:?}");
                        }
                        for i in (child_index..insert_pos).rev() {
                            keys[i + 1] = keys[i].take();
                        }
                        keys[child_index] = Some(median_child_key);
                        let mut child_insert_pos = child_index + 1;
                        while child_insert_pos < children.len() && children[child_insert_pos].is_some() {
                            child_insert_pos += 1;
                        }
                        if child_insert_pos >= children.len() {
                            panic!("No space to insert child");
                        }
                        for i in (child_index + 1..child_insert_pos).rev() {
                            children[i + 1] = children[i].take();
                        }
                        children[child_index + 1] = Some(new_child);
                    }
                }
            }
            None
        }
    }
}

#[derive(Debug)]
pub struct BTree<K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> where [(); M - 1]: {
    root: Option<Box<Node<K, V, M>>>,
}

impl<K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> BTree<K, V, M> where [(); M - 1]: {
    pub fn new() -> Self {
        if M < 3 {
            panic!("B-tree order M must be at least 3");
        }
        BTree { root: None }
    }

    pub fn stats(&self) -> Stats {
        Stats {
            size: self.root.as_ref().unwrap().size(),
            depth:  self.root.as_ref().unwrap().depth()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
    pub fn check(&self) {
        if let Some(root) = self.root.as_ref() {
            root.check();
        }
    }
    pub fn depth(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.depth())
    }

    pub fn get<B: ?Sized>(&self, key: &B) -> Option<&V> where K: Borrow<B>, B: Ord {
        let mut current = self.root.as_ref()?;
        loop {
            match &**current {
                Node::Internal { .. } => {
                    current = current.get_child_for_key(key);
                }
                Node::Leaf { .. } => {
                    return current.get_value(key);
                }
            }
        }
    }
    pub fn insert(&mut self, key: K, value: V) {
        if self.root.is_none() {
            let mut node = Node::new_leaf();
            node.insert_key_value(key, value);
            self.root = Some(node);
            return;
        }
        if self.root.as_mut().unwrap().is_full() {
            let t = (M + 1) / 2;
            let (median_key, new_sibling) = self.root.as_mut().unwrap().split(t).unwrap();
            let mut new_root = Node::new_internal();
            new_root.insert_child(0, self.root.take().unwrap()).unwrap();
            new_root.insert_key(0, median_key).unwrap();
            new_root.insert_child(1, new_sibling).unwrap();
            self.root = Some(new_root);
            self.insert(key, value);
        } else {
            if let Some((median_key, new_child)) = self.root.as_mut().unwrap().insert_recursive(key, value) {
                let node = self.root.as_mut().unwrap();
                let index = node.find_key_index(&median_key,QueryKind::Insert).expect("Insert query should always return integer");
                node.insert_key(index, median_key).unwrap();
                node.insert_child(index, new_child).unwrap();
            }
        }
    }
    pub fn range(&self, range: impl RangeBounds<K>) -> RangeIter<'_, K, V, M> {
        RangeIter {
            tree: self,
            start: match range.start_bound() {
                Bound::Included(k) => Bound::Included(k.to_owned()),
                Bound::Excluded(k) => Bound::Excluded(k.to_owned()),
                Bound::Unbounded => Bound::Unbounded,
            },
            end: match range.end_bound() {
                Bound::Included(k) => Bound::Included(k.to_owned()),
                Bound::Excluded(k) => Bound::Excluded(k.to_owned()),
                Bound::Unbounded => Bound::Unbounded,
            },
            stack: vec![],
            current: None,
            index: 0,
        }
    }
}

pub struct RangeIter<'a, K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> where [(); M - 1]: {
    tree: &'a BTree<K, V, M>,
    start: Bound<K>,
    end: Bound<K>,
    stack: Vec<(&'a Node<K, V, M>, usize)>,
    current: Option<&'a Node<K, V, M>>,
    index: usize,
}

impl<'a, K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> Iterator for RangeIter<'a, K, V, M> where [(); M - 1]: {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_none() {
            if let Some(node) = self.tree.root.as_ref() {
                self.current = Some(node);
                self.index = 0;
                self.stack.push((node, 0));
            } else {
                return None;
            }
        }
        while let Some(node) = self.current {
            if node.is_leaf() {
                let keys = match node {
                    Node::Leaf { keys, .. } => keys,
                    _ => unreachable!(),
                };
                let values = match node {
                    Node::Leaf { values, .. } => values,
                    _ => unreachable!(),
                };
                while self.index < keys.len() {
                    if let (Some(key), Some(value)) = (&keys[self.index], &values[self.index]) {
                        if self.in_range(key) {
                            let result = Some((key, value));
                            self.index += 1;
                            return result;
                        }
                    }
                    self.index += 1;
                }
                self.current = None;
                self.stack.pop();
                if let Some((parent, parent_index)) = self.stack.last_mut() {
                    self.current = Some(*parent);
                    self.index = *parent_index + 1;
                    *parent_index += 1;
                } else {
                    return None;
                }
            } else {
                if self.index < node.num_keys() + 1 {
                    if let Some(child) = node.get_child(self.index) {
                        self.stack.push((node, self.index));
                        self.current = Some(child);
                        self.index = 0;
                    } else {
                        self.index += 1;
                    }
                } else {
                    self.current = None;
                    self.stack.pop();
                    if let Some((parent, parent_index)) = self.stack.last_mut() {
                        self.current = Some(*parent);
                        self.index = *parent_index + 1;
                        *parent_index += 1;
                    }
                }
            }
        }
        None
    }
}

impl<'a, K: Ord + Clone + Debug + AsRef<str>, V: Clone + Debug, const M: usize> RangeIter<'a, K, V, M> where [(); M - 1]: {
    fn in_range(&self, key: &K) -> bool {
        match &self.start {
            Bound::Included(s) if key < s => return false,
            Bound::Excluded(s) if key <= s => return false,
            _ => {}
        }
        match &self.end {
            Bound::Included(e) if key > e => return false,
            Bound::Excluded(e) if key >= e => return false,
            _ => {}
        }
        true
    }
}