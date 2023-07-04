use crate::{
    error::Error,
    node::{NPrpt, NRef},
};
use core::{hash::Hash, mem::swap};
use std::collections::{HashMap, VecDeque};

/// functions necessary to make the queue work, but dependant on the implementation
trait FbQueueHelper<T, Priority>
where
    Priority: Ord,
{
    /// individual node of the queue, contining the value and priority
    type Node: NPrpt<T, Priority>;

    /* # nodes */

    /**
    increments the node count by one

    # Errors
    will error if the additional node cannot be accounted for in the count
    due to the size of the variable holding it
    */
    fn increment_node_count(&mut self) -> Result<(), Error>;

    /**
    decrements the node count by one

    # Errors
    will error if queue is already empty
    */
    fn decrement_node_count(&mut self) -> Result<(), Error>;

    /**
    calculates the maximum amount of children a node can have
    this works due to the structural theoretical soundness of the queue

    # Errors
    will error if the max rank breaks bounds of the variables used to store it
    */
    fn max_node_rank(&self) -> Result<usize, Error>;

    /* # first */

    /// returns a reference to the node holding the first element in queue
    fn get_first(&self) -> Option<&Self::Node>;

    /// sets the node as the first element in queue
    fn set_first(&mut self, node: Self::Node);

    /// swaps the first node in the queue for the given node and returns the previous one
    fn swap_first(&mut self, maybe_node: &mut Option<Self::Node>);

    /// sets the first node to None
    fn remove_first(&mut self);

    /// searches for the node with lowest priority
    fn find_first(&self) -> Option<Self::Node>;

    /* # roots */

    /// insert a node into the list of roots
    fn insert_root(&mut self, node: Self::Node);

    /**
    remove a node from the list of roots

    # Errors
    will error if the given node is not found in the roots list
    */
    fn remove_root(&mut self, node: Self::Node) -> Result<(), Error>;

    /// remove all roots and return them in a list
    fn drain_roots(&mut self) -> Vec<Self::Node>;

    /* # nodes */

    /// insert a node into the structure of the queue
    fn insert_node(&mut self, t: T, priority: Priority) -> Self::Node;

    /// finds the node with the given value
    fn get_node(&self, t: &T) -> Option<Self::Node>;

    /* # ops */

    /**
    collect smaller trees into bigger ones
    while satisfying structural properties

    # Errors
    may error if the node count exceeds the bounds of the variables used to store it
    */
    fn consolidate(&mut self) -> Result<(), Error> {
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

    /// separate node from its parent and add it to the list of roots
    /// possibly recursively to satisfu structural bounds of the queue
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

/// fibonacci queue (almost, the get first is not strictly O(1), sorry)
pub trait FbQueue<T, Priority>: FbQueueHelper<T, Priority>
where
    Priority: Ord,
{
    /// construct empty queue
    fn new() -> Self;

    /// returns true if the queue is empty
    fn is_empty(&self) -> bool;
    // fn peek(&self) -> Option<(&T, &Priority)>;

    /**
    push a value onto the queue with given priority

    # Errors
    will error if the queue is already at capacity
    */
    fn push(&mut self, t: T, priority: Priority) -> Result<(), Error> {
        let next = self.insert_node(t, priority);
        self.insert_root(next.clone());

        // there has to be a better way to write this conditional
        if let Some(first) = self.get_first() && first < &next {
        } else {
            self.set_first(next);
        }
        self.increment_node_count()?;
        Ok(())
    }

    /**
    return the element with the lowest priority

    # Errors
    Empty => cannot return element from empty queue
    InvalidIndex => internal indexing error
    */
    fn pop(&mut self) -> Result<(T, Priority), Error> {
        let mut extractee = None;
        self.swap_first(&mut extractee);

        let Some(first) = extractee else {
            return Err(Error::Empty);
        };

        self.decrement_node_count()?;
        self.remove_root(first.clone())?;

        for child in first.drain_children() {
            child.remove_parent();
            self.insert_root(child);
        }

        self.consolidate()?;

        if let Some(new_first) = self.find_first() {
            self.set_first(new_first);
        }

        first.pair()
    }

    /**
    decreases the priority of the item with given value

    # Errors
    InvalidIndex => index with given value was not found in the queue
    CannotIncreasePriority => the give prioprity is higher than the current one for the index of that value
    */
    fn decrease_priority(&mut self, value: &T, new_priority: Priority) -> Result<(), Error> {
        if let Some(node) = self.get_node(value) {
            if node.has_higher_priority(&new_priority) {
                node.set_priority(new_priority);
                if let Some(parent) = node.get_parent() && node < parent {
                    self.cut_node(node.clone());
                    if let Some(first) = self.get_first() && &node < first {
                    self.set_first(node);
                    }
                }
                Ok(())
            } else {
                Err(Error::CannotIncreasePriority)
            }
        } else {
            Err(Error::InvalidIndex)
        }
    }
}

/* # queues */

/* ## macros */

/// implements node functions for the ``FbQueueHelper`` Trait
macro_rules! make_node_count_fns {
    () => {
        fn increment_node_count(&mut self) -> Result<(), Error> {
            self.node_count = self
                .node_count
                .checked_add(1)
                .ok_or(Error::ReachedCapacity)?;
            Ok(())
        }

        fn decrement_node_count(&mut self) -> Result<(), Error> {
            self.node_count = self.node_count.checked_sub(1).ok_or(Error::Empty)?;
            Ok(())
        }

        fn max_node_rank(&self) -> Result<usize, Error> {
            // this is never less than log_ϕ(x)+1
            // and for x below 100 000 is only ever bigger by one
            // and we never cast to floats
            usize::try_from(
                self.node_count
                    .pow(3)
                    .ilog(4)
                    .checked_add(1)
                    .ok_or(Error::Numerical)?,
            )
            .map_err(|_| Error::Numerical)
        }
    };
}

/// implements 'first node' functions for the ``FbQueueHelper`` Trait
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

/// implements root functions for the ``FbQueueHelper`` Trait
macro_rules! make_root_fns {
    () => {
        fn insert_root(&mut self, node: Self::Node) {
            self.roots.push(node);
        }

        fn remove_root(&mut self, node: Self::Node) -> Result<(), Error> {
            // this should be O(1), but is not, would be if we had a proper linked list
            self.roots.swap_remove(
                self.roots
                    .iter()
                    .position(|x| x == &node)
                    .ok_or(Error::InvalidIndex)?,
            );
            Ok(())
        }

        fn drain_roots(&mut self) -> Vec<Self::Node> {
            self.roots.drain(..).collect()
        }
    };
}

/* # simple queue */

/// fibonacci queue implemented for values that do not implement copy or hash
pub struct SimpleQueue<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    /// list of roots
    roots: Vec<NRef<T, Priority>>,
    /// reference to the node with the lowest priority, it such exists
    first: Option<NRef<T, Priority>>,
    /// number of nodes in the queue
    node_count: usize,
}

impl<T, Priority> FbQueueHelper<T, Priority> for SimpleQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    type Node = NRef<T, Priority>;

    make_node_count_fns!();
    make_first_fns!();
    make_root_fns!();

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

    fn is_empty(&self) -> bool {
        self.node_count == 0
    }
}

/* ## hash queue */

/// fibonacci queue implemented for values which are Copy and Hash
/// however, if the Copy is slow, so will be the queue
/// since it copies the value every time it is input into the queue
pub struct HashQueue<T, Priority>
where
    T: Eq + Clone + Hash,
    Priority: Eq,
{
    /// map of nodes for O(1) lookup
    nodes: HashMap<T, NRef<T, Priority>>,
    /// list of roots
    roots: Vec<NRef<T, Priority>>,
    /// reference to the node with the lowest priority, it such exists
    first: Option<NRef<T, Priority>>,
    /// number of nodes in the queue
    node_count: usize,
}

impl<T, Priority> FbQueueHelper<T, Priority> for HashQueue<T, Priority>
where
    T: Eq + Clone + Hash,
    Priority: Ord,
{
    type Node = NRef<T, Priority>;

    make_node_count_fns!();
    make_first_fns!();
    make_root_fns!();

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

impl<T, Priority> FbQueue<T, Priority> for HashQueue<T, Priority>
where
    T: Eq + Clone + Hash,
    Priority: Ord,
{
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            first: None,
            node_count: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.node_count == 0
    }
}
