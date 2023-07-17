#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    ImpossibleRcRelease,
    InvalidIndex,
    ReachedCapacity,
    Numerical,
    Empty,
    CannotIncreasePriority,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::ImpossibleRcRelease => {
                write!(f, "cannot release rc due to outstanding reference")
            }
            Self::InvalidIndex => {
                write!(f, "requested value not found in queue")
            }
            Self::ReachedCapacity => {
                write!(f, "cannot account for additional nodes")
            }
            Self::Empty => {
                write!(f, "cannot perform operation on empty queue")
            }
            Self::Numerical => {
                write!(f, "failed numerical operation")
            }
            Self::CannotIncreasePriority => {
                write!(f, "cannot change priority to a higher value")
            }
        }
    }
}

impl std::error::Error for Error {}
