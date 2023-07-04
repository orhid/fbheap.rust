use crate::error::Error;
use core::{cell::RefCell, cmp::Ordering};
use std::rc::Rc;

pub type NRef<T, Priority> = Rc<RefCell<NCore<T, Priority>>>;

pub trait NPrpt<T, Priority>: Clone + Ord {
    fn new_node(t: T, priority: Priority) -> Self;

    /** # Errors
    will error if the reference count on self exceeds one
    */
    fn pair(self) -> Result<(T, Priority), Error>;
    // fn pair_ref(&self) -> (&T, &Priority);

    /* # values */
    fn has_higher_priority(&self, priority: &Priority) -> bool;
    fn set_priority(&self, priority: Priority);
    fn has_value(&self, t: &T) -> bool;

    /* # mark */
    fn mark(&self);
    fn unmark(&self);
    fn is_marked(&self) -> bool;

    /* # parents */
    fn get_parent(&self) -> Option<Self>;
    fn set_parent(&self, parent: Self);
    fn remove_parent(&self);

    /* # children */
    fn rank(&self) -> usize;
    fn insert_child(&self, child: Self);

    /** # Errors
    will error if the child is not found
    */
    fn remove_child(&self, child: &Self) -> Result<(), Error>;
    fn get_children(&self) -> Vec<Self>;
    fn drain_children(&self) -> Vec<Self>;

    /* # ops */
    fn link(&mut self, other: &mut Self);
}

#[derive(PartialEq, Eq)]
pub struct NCore<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    /// held value
    t: T,
    /// priority of the held value
    priority: Priority,
    /// parent node in the tree structure
    parent: Option<NRef<T, Priority>>,
    /// children in the tree structure
    children: Vec<NRef<T, Priority>>,
    /// flag for whether this node has lost any children already
    marked: bool,
}

impl<T, Priority> NCore<T, Priority>
where
    T: Eq,
    Priority: Eq,
{
    /// create ampty node
    const fn new(t: T, priority: Priority) -> Self {
        Self {
            t,
            priority,
            parent: None,
            children: Vec::new(),
            marked: false,
        }
    }

    // this cannot actually be a constant function
    #[allow(clippy::missing_const_for_fn)]
    /// destructure the node into patrs relevant to the outside
    fn pair(self) -> (T, Priority) {
        (self.t, self.priority)
    }

    /*
    fn pair_ref(&self) -> (&T, &Priority) {
        (&self.t, &self.priority)
    }
    */
}

impl<T, Priority> PartialOrd for NCore<T, Priority>
where
    T: Eq,
    Priority: Eq + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl<T, Priority> Ord for NCore<T, Priority>
where
    T: Eq,
    Priority: Eq + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

/*
impl<T, Priority> Hash for NCore<T, Priority>
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

impl<T, Priority> NPrpt<T, Priority> for NRef<T, Priority>
where
    T: Eq,
    Priority: Eq + Ord,
{
    fn new_node(t: T, priority: Priority) -> Self {
        Self::new(RefCell::new(NCore::new(t, priority)))
    }

    fn rank(&self) -> usize {
        self.borrow().children.len()
    }

    fn pair(self) -> Result<(T, Priority), Error> {
        Ok(Self::into_inner(self)
            .ok_or(Error::ImpossibleRcRelease)?
            .into_inner()
            .pair())
    }

    /*
    fn pair_ref(&self) -> (&T, &Priority) {
        self.borrow().pair_ref()
    }
    */

    fn has_higher_priority(&self, priority: &Priority) -> bool {
        self.borrow().priority > *priority
    }

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

    fn remove_child(&self, child: &Self) -> Result<(), Error> {
        let index = self
            .borrow()
            .children
            .iter()
            .position(|x| x == child)
            .ok_or(Error::InvalidIndex)?;
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
        smaller.unmark();
    }
}
