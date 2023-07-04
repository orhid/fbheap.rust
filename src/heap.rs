use crate::node::{FbNode, NodeRef};
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    mem::swap,
};

trait FbQueueHelper<T, Priority>
where
    Priority: Ord,
{
    type Node: FbNode<T, Priority>;

    fn increment_node_count(&mut self) -> Result<(), &'static str>;
    fn decrement_node_count(&mut self) -> Result<(), &'static str>;
    fn max_node_rank(&self) -> Result<usize, &'static str>;

    fn get_first(&self) -> Option<&Self::Node>;
    fn set_first(&mut self, node: Self::Node);
    fn swap_first(&mut self, maybe_node: &mut Option<Self::Node>);
    fn remove_first(&mut self);
    fn find_first(&self) -> Option<Self::Node>;

    fn insert_root(&mut self, node: Self::Node);
    fn remove_root(&mut self, node: Self::Node) -> Result<(), &'static str>;
    fn drain_roots(&mut self) -> Vec<Self::Node>;

    fn insert_node(&mut self, t: T, priority: Priority) -> Self::Node;
    fn get_node(&self, t: &T) -> Option<Self::Node>;

    fn consolidate(&mut self) -> Result<(), &'static str> {
        let mut ranks: Vec<Option<Self::Node>> = (0..self.max_node_rank()?).map(|_| None).collect();

        for mut root in self.drain_roots() {
            let mut rank = root.rank();
            // indexing is safe, since structural guarantees
            while let Some(node) = &mut ranks[rank] {
                root.link(node);
                ranks[rank] = None;
                rank = root.rank();
            }
            ranks[rank] = Some(root);
        }

        for node in ranks.into_iter().flatten() {
            self.insert_root(node);
        }
        Ok(())
    }

    fn cut_node(&mut self, node: Self::Node) {
        if let Some(parent) = node.get_parent() {
            parent.mark();
            node.remove_parent();
            self.insert_root(node.clone());
            node.unmark();
            if parent.is_marked() {
                self.cut_node(parent);
            }
        }
    }
}

trait FbQueue<T, Priority>: FbQueueHelper<T, Priority>
where
    Priority: Ord,
{
    fn new() -> Self;
    // fn peek(&self) -> Option<(&T, &Priority)>;

    fn push(&mut self, t: T, priority: Priority) {
        let next = self.insert_node(t, priority);
        self.insert_root(next.clone());

        // there has to be a better way to write this conditional
        if self.get_first().is_none()
            || next
                < *self
                    .get_first()
                    // .clone() /* is clone necessary here ? */
                    .expect("just checked if none")
        {
            self.set_first(next);
        }
        self.increment_node_count();
    }

    fn pop(&mut self) -> Option<(T, Priority)> {
        let mut extractee = None;
        self.swap_first(&mut extractee);

        extractee.map(|first| {
            self.decrement_node_count();
            self.remove_root(first.clone());

            for child in first.drain_children() {
                child.remove_parent();
                self.insert_root(child);
            }

            self.consolidate();

            if let Some(new_first) = self.find_first() {
                self.set_first(new_first);
            }

            // this can be done better
            match first.pair() {
                Ok(pair) => pair,
                Err(message) => panic!("{}", message),
            }
        })
    }

    // i would like to have proper enum errors here
    fn decrease_priority(&mut self, value: &T, new_priority: Priority) -> Result<(), &'static str> {
        self.get_node(value).ok_or("index not found").map(|node| {
            node.set_priority(new_priority);
            if let Some(parent) = node.get_parent() && node < parent {
                self.cut_node(node.clone());
                if let Some(first) = self.get_first() && &node < first {
                    self.set_first(node);
                }
            }
        })
    }
}

/* # queues */

/* ## macros */

macro_rules! make_node_count_fns {
    () => {
        fn increment_node_count(&mut self) -> Result<(), &'static str> {
            self.node_count = self.node_count.checked_add(1).ok_or("at capacity")?;
            Ok(())
        }

        fn decrement_node_count(&mut self) -> Result<(), &'static str> {
            self.node_count = self.node_count.checked_sub(1).ok_or("already empty")?;
            Ok(())
        }

        fn max_node_rank(&self) -> Result<usize, &'static str> {
            // this is never less than log_ϕ(x)+1
            // and for x below 100 000 is only ever bigger by one
            // and we never cast to floats
            usize::try_from(
                self.node_count
                    .pow(3)
                    .ilog(4)
                    .checked_add(1)
                    .ok_or("overflow")?,
            )
            .map_err(|_| "conversion failure")
        }
    };
}

macro_rules! make_first_fns {
    () => {
        fn get_first(&self) -> Option<&Self::Node> {
            self.first.as_ref()
        }

        fn set_first(&mut self, node: Self::Node) {
            self.first = Some(node)
        }

        fn remove_first(&mut self) {
            self.first = None;
        }

        fn swap_first(&mut self, maybe_node: &mut Option<Self::Node>) {
            swap(&mut self.first, maybe_node);
        }

        fn find_first(&self) -> Option<Self::Node> {
            self.roots.iter().min().cloned()
        }
    };
}

/* # simple queue */

struct SimpleQueue<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    roots: Vec<NodeRef<T, Priority>>,
    first: Option<NodeRef<T, Priority>>,
    node_count: usize,
}

impl<T, Priority> FbQueueHelper<T, Priority> for SimpleQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    type Node = NodeRef<T, Priority>;

    make_node_count_fns!();
    make_first_fns!();

    /* # roots */

    fn insert_root(&mut self, node: Self::Node) {
        self.roots.push(node);
    }

    fn remove_root(&mut self, node: Self::Node) -> Result<(), &'static str> {
        // this should be O(1), but is not, would be if we had a proper linked list
        self.roots.swap_remove(
            self.roots
                .iter()
                .position(|x| x == &node)
                .ok_or("not a root")?,
        );
        Ok(())
    }

    fn drain_roots(&mut self) -> Vec<Self::Node> {
        self.roots.drain(..).collect()
    }

    /* # nodes */

    fn insert_node(&mut self, t: T, priority: Priority) -> Self::Node {
        Self::Node::new_node(t, priority)
    }

    fn get_node(&self, t: &T) -> Option<Self::Node> {
        // bfs on nodes
        let mut q = self.roots.iter().cloned().collect::<VecDeque<_>>();
        while let Some(node) = q.pop_front() {
            if node.has_value(t) {
                return Some(node);
            }
            for child in node.get_children() {
                q.push_back(child);
            }
        }
        None
    }
}

impl<T, Priority> FbQueue<T, Priority> for SimpleQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    fn new() -> Self {
        Self {
            roots: Vec::new(),
            first: None,
            node_count: 0,
        }
    }
}

/* ## hash queue */

struct HashQueue<T, Priority>
where
    T: Eq + Clone + Hash,
    Priority: Eq + Hash,
{
    nodes: HashMap<T, NodeRef<T, Priority>>,
    roots: Vec<NodeRef<T, Priority>>,
    first: Option<NodeRef<T, Priority>>,
    node_count: usize,
}

impl<T, Priority> FbQueueHelper<T, Priority> for HashQueue<T, Priority>
where
    T: Eq + Clone + Hash,
    Priority: Ord + Hash,
{
    type Node = NodeRef<T, Priority>;

    make_node_count_fns!();
    make_first_fns!();

    /* # roots */

    fn insert_root(&mut self, node: Self::Node) {
        self.roots.push(node);
    }

    fn remove_root(&mut self, node: Self::Node) -> Result<(), &'static str> {
        // this should be O(1), but is not, would be if we had a proper linked list

        self.roots.swap_remove(
            self.roots
                .iter()
                .position(|x| x == &node)
                .ok_or("not a root")?,
        );
        Ok(())
    }

    fn drain_roots(&mut self) -> Vec<Self::Node> {
        self.roots.drain(..).collect()
    }

    /* # nodes */

    fn insert_node(&mut self, t: T, priority: Priority) -> Self::Node {
        let node = Self::Node::new_node(t.clone(), priority);
        self.nodes.insert(t, node.clone());
        node
    }

    fn get_node(&self, t: &T) -> Option<Self::Node> {
        self.nodes.get(t).cloned()
    }
}
