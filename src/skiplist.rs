use std::{
    cell::RefCell,
    f64::MAX,
    fmt::Debug,
    rc::{Rc, Weak},
    sync::Arc,
    u64::MIN,
};
//A SkipList is a probabilistic data structure that allows for fast search within an ordered sequence of elements. It consists of a hierarchy of linked lists, where:

//The bottom layer is a regular sorted linked list containing all elements.
//Each higher layer acts as an "express lane" for the lists below, skipping over some elements.
//The probability of an element appearing in each level decreases exponentially with the level.
//
//[level 3] --------------------------> [6]----------> [8]
//[level 2] -------> [4]----------------[6] ---------> [8]
//[level 0] -> [1] ->[4]-->[3] -> [5] --[6]--> [7]---> [8]

use rand::{Rng, random};

type Link<K, V> = Option<Rc<RefCell<SkipNode<K, V>>>>;
type WeakLink<K, V> = Option<Weak<RefCell<SkipNode<K, V>>>>; // backwarding non-owning

#[derive(Debug, Clone)]
pub struct SkipNode<K, V> {
    key: Option<K>,
    value: Option<V>,
    next: Vec<Link<K, V>>,
}

impl<K, V> SkipNode<K, V>
where
    K: Debug + Default,
    V: Ord + Debug + Default,
{
    pub fn new(key: K, value: V, levels: usize) -> Self {
        SkipNode {
            key: Some(key),
            value: Some(value),
            next: Self::empty_level(levels),
        }
    }

    fn empty_level(levels: usize) -> Vec<Link<K, V>> {
        let mut next = Vec::with_capacity(levels);
        for _ in 0..levels {
            next.push(None);
        }
        next
    }

    pub fn sentinel(levels: usize, key: K, value: V) -> Self {
        SkipNode {
            key: Some(key),
            value: Some(value),
            next: Self::empty_level(levels),
        }
    }
}

type NodePtr<K, V> = Rc<RefCell<SkipNode<K, V>>>;

#[derive(Debug)]
pub struct SkipList<K, V> {
    head: Rc<RefCell<SkipNode<K, V>>>,
    //tail: SkipNode<K, V>,
    max_level: usize,
    probability_by_level: Vec<f64>,
}

impl<K, V> SkipList<K, V>
where
    K: Debug + Default,
    V: Ord + Debug + Default,
{
    pub fn new(max_level: usize, head_value: V, tail_value: V) -> Self {
        let mut probability_by_level = Vec::with_capacity(max_level);
        for i in 0..max_level {
            probability_by_level.push(1.0 / (2.0_f64).powi(i as i32));
        }
        let head = SkipNode::<K, V>::sentinel(max_level, K::default(), head_value);
        let head = Rc::new(RefCell::new(head));

        SkipList {
            head,
            max_level,
            probability_by_level,
        }
    }
    /// Identity check: do two Rc's point to the same node?
    fn same_node(a: &NodePtr<K, V>, b: &NodePtr<K, V>) -> bool {
        Rc::ptr_eq(a, b)
    }
    /// Read: get a clone of node.forward[level]
    fn get_forward(node: &NodePtr<K, V>, level: usize) -> Link<K, V> {
        node.borrow().next[level].as_ref().map(Rc::clone)
    }

    /// Write: set node.forward[level] = new_node
    fn set_forward(node: &NodePtr<K, V>, level: usize, new_node: Link<K, V>) {
        node.borrow_mut().next[level] = new_node;
    }

    /// Compare: is node's  value < given valeu?
    fn node_value_less(node: &NodePtr<K, V>, value: &V) -> bool {
        node.borrow().value.as_ref().unwrap() < value
    }

    /// Compare: is node's value == given value?
    fn node_value_equals(node: &NodePtr<K, V>, value: &V) -> bool {
        node.borrow().value.as_ref().unwrap() == value
    }

    pub fn search(&self, value: &V) -> Option<Rc<RefCell<SkipNode<K, V>>>>
    where
        K: Ord,
    {
        let mut current = Rc::clone(&self.head);
        for lvl in (0..self.max_level).rev() {
            loop {
                // borrow the current, clone the next pointer
                let next = {
                    let ref_curr = current.borrow();
                    ref_curr.next[lvl].as_ref().map(Rc::clone)
                }; //drop the borrow
                match next {
                    Some(next_node) => {
                        let should_move_forward = {
                            let ref_next = next_node.borrow();
                            ref_next.value.as_ref().unwrap() < value
                        };
                        if should_move_forward {
                            current = next_node;
                        } else {
                            break; // move down
                        }
                    }
                    None => break, // reach the end of the level, move down
                }
            }
        }
        let candidate = {
            let ref_curr = current.borrow();
            ref_curr.next[0].as_ref().map(Rc::clone)
        };
        candidate.filter(|n| {
            let n_ref = n.borrow();
            n_ref.value.as_ref().unwrap() == value
        })
    }

    pub fn pick_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut lvl = 0;
        // check bound first
        while lvl < self.max_level && rng.r#gen::<f64>() < self.probability_by_level[lvl] {
            lvl += 1;
        }
        lvl
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        //update[0..max_level] <- head

        let mut update: Vec<NodePtr<K, V>> = vec![Rc::clone(&self.head); self.max_level];
        //current <- head
        let mut current = Rc::clone(&self.head);

        //for level from current_level down to 0:
        for level in (0..self.max_level).rev() {
            //    while current.forward[level] != NIL
            //          and current.forward[level].key < key:
            while let Some(next) = Self::get_forward(&current, level) {
                if Self::node_value_less(&next, &value) {
                    //        current <- current.forward[level]
                    current = next;
                } else {
                    break; // move down
                }
            }
            //    update[level] <- current
            update[level] = Rc::clone(&current);
        }

        //current <- current.forward[0]
        //if current != NIL and current.key == key:
        //    current.value <- value
        //    return
        if let Some(existing) = Self::get_forward(&current, 0) {
            if Self::node_value_equals(&existing, &value) {
                let old = existing.borrow_mut().value.replace(value);
                return old;
            }
        }

        //new_level <- RANDOM_LEVEL(max_level, p)
        let new_level = self.pick_level();
        let new_node = Rc::new(RefCell::new(SkipNode::new(key, value, new_level)));
        // for level from 0 to new_level:
        for level in 0..new_level {
            //   node.forward[level] <- update[level].forward[level]
            let pred_next = Self::get_forward(&update[level], level);
            Self::set_forward(&new_node, level, pred_next);

            //   update[level].forward[level] <- node
            Self::set_forward(&update[level], level, Some(Rc::clone(&new_node)));
        }

        None
    }

    pub fn get_key_by_value(&self, value: &V) -> Option<K>
    where
        K: Ord + Clone,
    {
        self.search(value)
            .and_then(|node| node.borrow().key.as_ref().cloned())
    }

    fn insert_force_level(&mut self, key: K, value: V, level_forced: usize) -> Option<V> {
        //update[0..max_level] <- head

        let mut update: Vec<NodePtr<K, V>> = vec![Rc::clone(&self.head); self.max_level];
        //current <- head
        let mut current = Rc::clone(&self.head);

        //for level from current_level down to 0:
        for level in (0..self.max_level).rev() {
            //    while current.forward[level] != NIL
            //          and current.forward[level].key < key:
            while let Some(next) = Self::get_forward(&current, level) {
                if Self::node_value_less(&next, &value) {
                    //        current <- current.forward[level]
                    current = next;
                } else {
                    break; // move down
                }
            }
            //    update[level] <- current
            update[level] = Rc::clone(&current);
        }

        //current <- current.forward[0]
        //if current != NIL and current.key == key:
        //    current.value <- value
        //    return
        if let Some(existing) = Self::get_forward(&current, 0) {
            if Self::node_value_equals(&existing, &value) {
                let old = existing.borrow_mut().value.replace(value);
                return old;
            }
        }

        //new_level <- RANDOM_LEVEL(max_level, p) the only diff form insert
        let new_level = level_forced;

        let new_node = Rc::new(RefCell::new(SkipNode::new(key, value, new_level)));
        // for level from 0 to new_level:
        for level in 0..new_level {
            //   node.forward[level] <- update[level].forward[level]
            let pred_next = Self::get_forward(&update[level], level);
            Self::set_forward(&new_node, level, pred_next);

            //   update[level].forward[level] <- node
            Self::set_forward(&update[level], level, Some(Rc::clone(&new_node)));
        }

        None
    }

    fn delete(&self, value: V) -> Option<V> {
        // update[0..max_level] <- NIL
        let mut update: Vec<NodePtr<K, V>> = vec![Rc::clone(&self.head); self.max_level];

        let mut current = Rc::clone(&self.head);
        for level in (0..self.max_level).rev() {
            while let Some(next) = Self::get_forward(&current, level) {
                if Self::node_value_less(&next, &value) {
                    current = next;
                } else {
                    break; // move down
                }
            }
            update[level] = Rc::clone(&current);
        }
        //    target <- current.forward[0]
        //
        //    if target == NIL or target.key != key:
        //        return NOT_FOUND
        //

        let target = match Self::get_forward(&current, 0) {
            Some(node) if Self::node_value_equals(&node, &value) => node,
            _ => return None, // not found
        };

        // alternative to this is goes with target high only
        for level in 0..self.max_level {
            match Self::get_forward(&update[level], level) {
                Some(fwd) if Self::same_node(&fwd, &target) => {
                    //bypass target predecessor skips over it
                    let target_next = Self::get_forward(&target, level);
                    Self::set_forward(&update[level], level, target_next);
                }
                _ => break, // no more levels to update
            }
        }

        target.borrow_mut().value.take() // return the removed value
    }

    fn print_level(&self, level: usize) {
        let mut current = Rc::clone(&self.head);
        print!("Level {}: ", level);
        loop {
            let next = Self::get_forward(&current, level);
            match next {
                Some(next_node) => {
                    print!("{:#?} -> ", next_node.borrow().value.as_ref().unwrap());
                    current = next_node;
                }
                None => {
                    println!("None");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::Rc,
        u64::{MAX, MIN},
    };

    use crate::skiplist::{SkipList, SkipNode};

    #[test]
    fn test_skiplist_new() {
        let s = SkipList::<String, u64>::new(3, MIN, MAX);
        //println!("{:#?}", s);
        assert_eq!(s.max_level, 3);
    }

    #[test]
    fn test_search_new() {
        let mut s = SkipList::<String, u64>::new(3, MIN, MAX);

        // Create each node ONCE as an Rc — same allocation shared across all levels
        // The `levels` arg controls how many levels this node participates in
        let node1 = Rc::new(RefCell::new(SkipNode::new("1".to_string(), 1, 1))); // level 0 only
        let node3 = Rc::new(RefCell::new(SkipNode::new("3".to_string(), 3, 1))); // level 0 only
        let node4 = Rc::new(RefCell::new(SkipNode::new("4".to_string(), 4, 2))); // levels 0-1
        let node5 = Rc::new(RefCell::new(SkipNode::new("5".to_string(), 5, 1))); // level 0 only
        let node6 = Rc::new(RefCell::new(SkipNode::new("6".to_string(), 6, 3))); // levels 0-2
        let node7 = Rc::new(RefCell::new(SkipNode::new("7".to_string(), 7, 1))); // level 0 only
        let node8 = Rc::new(RefCell::new(SkipNode::new("8".to_string(), 8, 3))); // levels 0-2

        // Wire up level 2: head → 6 → 8
        s.head.borrow_mut().next[2] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[2] = Some(Rc::clone(&node8));

        // Wire up level 1: head → 4 → 6 → 8
        s.head.borrow_mut().next[1] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[1] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[1] = Some(Rc::clone(&node8));

        // Wire up level 0: head → 1 → 3 → 4 → 5 → 6 → 7 → 8
        s.head.borrow_mut().next[0] = Some(Rc::clone(&node1));
        node1.borrow_mut().next[0] = Some(Rc::clone(&node3));
        node3.borrow_mut().next[0] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[0] = Some(Rc::clone(&node5));
        node5.borrow_mut().next[0] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[0] = Some(Rc::clone(&node7));
        node7.borrow_mut().next[0] = Some(Rc::clone(&node8));

        // Search should work now
        let found = s.search(&6);
        println!("found node: {:#?}", found);
        assert!(found.is_some());
        assert_eq!(*found.unwrap().borrow().value.as_ref().unwrap(), 6);

        let not_found = s.search(&9);
        assert!(not_found.is_none());

        let found = s.search(&7);
        println!("found node: {:#?}", found);
    }

    #[test]
    fn test_insert_new() {
        let mut s = SkipList::<String, u64>::new(3, MIN, MAX);

        let node4 = Rc::new(RefCell::new(SkipNode::new("4".to_string(), 4, 2))); // levels 0-1
        let node6 = Rc::new(RefCell::new(SkipNode::new("6".to_string(), 6, 3))); // levels 0-2
        let node7 = Rc::new(RefCell::new(SkipNode::new("7".to_string(), 7, 1))); // level 0 only
        let node8 = Rc::new(RefCell::new(SkipNode::new("8".to_string(), 8, 3))); // levels 0-2
        let node10 = Rc::new(RefCell::new(SkipNode::new("10".to_string(), 10, 3))); // levels 0-2
        // Wire up level 0: head → 4 → 6 → 7 → 8 -> 10
        s.head.borrow_mut().next[0] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[0] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[0] = Some(Rc::clone(&node7));
        node7.borrow_mut().next[0] = Some(Rc::clone(&node8));
        node8.borrow_mut().next[0] = Some(Rc::clone(&node10));

        //level 1: head → 4 → 6 → 8
        s.head.borrow_mut().next[1] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[1] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[1] = Some(Rc::clone(&node8));

        //level 2: head → 6 → 8

        s.head.borrow_mut().next[2] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[2] = Some(Rc::clone(&node8));

        s.insert("9".to_string(), 9);

        s.print_level(0);
        s.print_level(1);
        s.print_level(2);

        let found = s.search(&9);
        println!("found node: {:#?}", found);
        assert!(found.is_some());
        assert_eq!(*found.unwrap().borrow().value.as_ref().unwrap(), 9);
    }

    #[test]
    fn test_insert_level_new() {
        let mut s = SkipList::<String, u64>::new(3, MIN, MAX);

        let node4 = Rc::new(RefCell::new(SkipNode::new("4".to_string(), 4, 2))); // levels 0-1
        let node6 = Rc::new(RefCell::new(SkipNode::new("6".to_string(), 6, 3))); // levels 0-2
        let node7 = Rc::new(RefCell::new(SkipNode::new("7".to_string(), 7, 1))); // level 0 only
        let node8 = Rc::new(RefCell::new(SkipNode::new("8".to_string(), 8, 3))); // levels 0-2
        let node10 = Rc::new(RefCell::new(SkipNode::new("10".to_string(), 10, 3))); // levels 0-2
        // Wire up level 0: head → 4 → 6 → 7 → 8 -> 10
        s.head.borrow_mut().next[0] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[0] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[0] = Some(Rc::clone(&node7));
        node7.borrow_mut().next[0] = Some(Rc::clone(&node8));
        node8.borrow_mut().next[0] = Some(Rc::clone(&node10));

        //level 1: head → 4 → 6 → 8
        s.head.borrow_mut().next[1] = Some(Rc::clone(&node4));
        node4.borrow_mut().next[1] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[1] = Some(Rc::clone(&node8));

        //level 2: head → 6 → 8

        s.head.borrow_mut().next[2] = Some(Rc::clone(&node6));
        node6.borrow_mut().next[2] = Some(Rc::clone(&node8));

        s.insert_force_level("9".to_string(), 9, 2);

        s.print_level(0);
        s.print_level(1);
        s.print_level(2);

        let found = s.search(&9);
        println!("found node: {:#?}", found);
        assert!(found.is_some());
        assert_eq!(*found.unwrap().borrow().value.as_ref().unwrap(), 9);
    }

    #[test]
    fn test_delete() {
        let mut s = SkipList::<String, u64>::new(3, MIN, MAX);
        // nodes
        let mut nodes_to_insert = Vec::new();
        for i in 1..=10 {
            let key = i.to_string();
            let value = i as u64;
            let level = s.pick_level();
            nodes_to_insert.push((key, value, level));
        }

        for (key, value, level) in nodes_to_insert {
            s.insert_force_level(key, value, level);
        }

        s.print_level(0);
        s.print_level(1);
        s.print_level(2);

        let d = s.delete(8);
        assert!(d.is_some());
        assert_eq!(d.unwrap(), 8);

        s.print_level(0);
        s.print_level(1);
        s.print_level(2);
    }
}
