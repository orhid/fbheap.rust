use std::{
    cell::RefCell,
    cmp::Ordering,
    // hash::{Hash, Hasher},
    rc::Rc,
};

pub type NodeRef<T, Priority> = Rc<RefCell<NodeCore<T, Priority>>>;

pub trait FbNode<T, Priority>: Clone + Ord {
    fn new_node(t: T, priority: Priority) -> Self;
    fn rank(&self) -> usize;
    fn pair(self) -> Result<(T, Priority), &'static str>;
    // fn pair_ref(&self) -> (&T, &Priority);

    fn set_priority(&self, priority: Priority);
    fn has_value(&self, t: &T) -> bool;

    fn mark(&self);
    fn unmark(&self);
    fn is_marked(&self) -> bool;

    fn get_parent(&self) -> Option<Self>;
    fn set_parent(&self, parent: Self);
    fn remove_parent(&self);

    fn insert_child(&self, child: Self);
    fn remove_child(&self, child: &Self) -> Result<(), &'static str>;
    fn get_children(&self) -> Vec<Self>;
    fn drain_children(&self) -> Vec<Self>;

    fn link(&mut self, other: &mut Self);
}

#[derive(PartialEq, Eq)]
pub struct NodeCore<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    t: T,
    priority: Priority,
    parent: Option<NodeRef<T, Priority>>,
    children: Vec<NodeRef<T, Priority>>,
    marked: bool,
}

impl<T, Priority> NodeCore<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    const fn new(t: T, priority: Priority) -> Self {
        Self {
            t,
            priority,
            parent: None,
            children: Vec::new(),
            marked: false,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    // this cannot actually be a constant function
    fn pair(self) -> (T, Priority) {
        (self.t, self.priority)
    }

    /*
    fn pair_ref(&self) -> (&T, &Priority) {
        (&self.t, &self.priority)
    }
    */
}

impl<T, Priority> PartialOrd for NodeCore<T, Priority>
where
    T: Eq,
    Priority: Eq + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl<T, Priority> Ord for NodeCore<T, Priority>
where
    T: Eq,
    Priority: Eq + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

/*
impl<T, Priority> Hash for NodeCore<T, Priority>
where
    T: Eq + Hash,
    Priority: Eq + Ord + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.t.hash(state);
        self.priority.hash(state);
    }
}
*/

impl<T, Priority> FbNode<T, Priority> for NodeRef<T, Priority>
where
    T: Eq,
    Priority: Eq + Ord,
{
    fn new_node(t: T, priority: Priority) -> Self {
        Self::new(RefCell::new(NodeCore::new(t, priority)))
    }

    fn rank(&self) -> usize {
        self.borrow().children.len()
    }

    fn pair(self) -> Result<(T, Priority), &'static str> {
        Ok(Self::into_inner(self)
            .ok_or("could not release rc")?
            .into_inner()
            .pair())
    }

    /*
    fn pair_ref(&self) -> (&T, &Priority) {
        self.borrow().pair_ref()
    }
    */

    fn set_priority(&self, priority: Priority) {
        self.borrow_mut().priority = priority;
    }

    fn has_value(&self, t: &T) -> bool {
        self.borrow().t == *t
    }

    fn mark(&self) {
        self.borrow_mut().marked = true;
    }

    fn unmark(&self) {
        self.borrow_mut().marked = false;
    }

    fn is_marked(&self) -> bool {
        self.borrow().marked
    }

    fn get_parent(&self) -> Option<Self> {
        self.borrow().parent.clone()
    }

    fn set_parent(&self, parent: Self) {
        self.borrow_mut().parent = Some(parent);
    }

    fn remove_parent(&self) {
        self.borrow_mut().parent = None;
    }

    fn insert_child(&self, child: Self) {
        self.borrow_mut().children.push(child);
    }

    fn remove_child(&self, child: &Self) -> Result<(), &'static str> {
        // -> Result?
        let index = self
            .borrow()
            .children
            .iter()
            .position(|x| x == child)
            .ok_or("not a child")?;
        self.borrow_mut().children.swap_remove(index);
        Ok(())
    }

    fn get_children(&self) -> Vec<Self> {
        self.borrow_mut().children.clone()
    }

    fn drain_children(&self) -> Vec<Self> {
        self.borrow_mut().children.drain(..).collect()
    }

    fn link(&mut self, other: &mut Self) {
        let (smaller, bigger) = match self.cmp(&other) {
            Ordering::Greater => (other, self),
            _ => (self, other),
        };

        bigger.set_parent(smaller.clone());
        smaller.insert_child(bigger.clone());
        // probably unmarking
        todo!();
    }
}
