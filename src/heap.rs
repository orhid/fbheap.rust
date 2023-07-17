use crate::{
    error::Error,
    node::{NPrpt, NRef},
};
use core::mem::swap;
use std::collections::VecDeque;

/* # bare queue */

/**
fibonacci queue implemented for values that do not implement copy or hash

```
use fbheap::error::Error::Empty;
use fbheap::heap::BareQueue;

let mut queue = BareQueue::new();
queue.push("i was first", 3);
queue.push("i am important", 1);
queue.push("i was not important at first", 4);
assert_eq!(queue.pop(), Ok(("i am important", 1)));
queue.decrease_priority(&"i was not important at first", 2);
assert_eq!(queue.pop(), Ok(("i was not important at first", 2)));
assert_eq!(queue.pop(), Ok(("i was first", 3)));
assert!(queue.is_empty());
assert_eq!(queue.pop(), Err(Empty));
```
*/
pub struct BareQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    /// list of roots
    roots: Vec<NRef<T, Priority>>,
    /// reference to the node with the lowest priority, it such exists
    first: Option<NRef<T, Priority>>,
    /// number of nodes in the queue
    node_count: usize,
}

impl<T, Priority> Default for BareQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Priority> BareQueue<T, Priority>
where
    T: Eq,
    Priority: Ord,
{
    /* # helper functions */

    /* ## node count functions */

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
        match self.node_count {
            0 => Ok(0),
            node_count => {
                // this is never less than log_ϕ(x)+1
                // and for x below 100 000 is only ever bigger by one
                // and we never cast to floats
                usize::try_from(
                    node_count
                        .pow(3)
                        .ilog(4)
                        .checked_add(1)
                        .ok_or(Error::Numerical)?,
                )
                .map_err(|_| Error::Numerical)
            }
        }
    }

    /* ## first element functions */

    const fn get_first(&self) -> Option<&NRef<T, Priority>> {
        self.first.as_ref()
    }

    fn set_first(&mut self, node: NRef<T, Priority>) {
        self.first = Some(node)
    }

    // fn remove_first(&mut self) {
    //     self.first = None;
    // }

    fn swap_first(&mut self, maybe_node: &mut Option<NRef<T, Priority>>) {
        swap(&mut self.first, maybe_node);
    }

    fn find_first(&self) -> Option<NRef<T, Priority>> {
        self.roots.iter().min().cloned()
    }

    /* ## root functions */

    fn insert_root(&mut self, node: NRef<T, Priority>) {
        self.roots.push(node);
    }

    fn remove_root(&mut self, node: NRef<T, Priority>) -> Result<(), Error> {
        // TODO : this should be O(1), but is not, would be if we had a proper linked list
        self.roots.swap_remove(
            self.roots
                .iter()
                .position(|x| x == &node)
                .ok_or(Error::InvalidIndex)?,
        );
        Ok(())
    }

    fn drain_roots(&mut self) -> Vec<NRef<T, Priority>> {
        self.roots.drain(..).collect()
    }

    /* ## structural functions */

    fn consolidate(&mut self) -> Result<(), Error> {
        let mut ranks: Vec<Option<NRef<T, Priority>>> =
            (0..self.max_node_rank()?).map(|_| None).collect();

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
    /// possibly recursively to satisfy structural bounds of the queue
    fn cut_node(&mut self, node: NRef<T, Priority>) {
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

    fn get_node(&self, t: &T) -> Option<NRef<T, Priority>> {
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

    /* # heap functionality */

    /// construct empty queue
    #[must_use]
    pub const fn new() -> Self {
        Self {
            roots: Vec::new(),
            first: None,
            node_count: 0,
        }
    }

    /// returns true if the queue is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    // fn peek(&self) -> Option<(&T, &Priority)>;

    /**
    push a value onto the queue with given priority

    # Errors
    will error if the queue is already at capacity
    */
    pub fn push(&mut self, t: T, priority: Priority) -> Result<(), Error> {
        let next = NRef::<T, Priority>::new_node(t, priority);
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
    Empty => cannot return element from empty queue\n
    InvalidIndex => internal indexing error
    */
    pub fn pop(&mut self) -> Result<(T, Priority), Error> {
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
    InvalidIndex => index with given value was not found in the queue\n
    CannotIncreasePriority => the give prioprity is higher than the current one for the index of that value
    */
    pub fn decrease_priority(&mut self, value: &T, new_priority: Priority) -> Result<(), Error> {
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
