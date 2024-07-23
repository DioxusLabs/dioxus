use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;

use crate::GenerationalLocation;

#[derive(Debug, Clone, PartialEq)]
/// An error that can occur when trying to borrow a value.
pub enum BorrowError {
    /// The value was dropped.
    Dropped(ValueDroppedError),
    /// The value was already borrowed mutably.
    AlreadyBorrowedMut(AlreadyBorrowedMutError),
}

impl Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::Dropped(error) => Display::fmt(error, f),
            BorrowError::AlreadyBorrowedMut(error) => Display::fmt(error, f),
        }
    }
}

impl Error for BorrowError {}

#[derive(Debug, Clone, PartialEq)]
/// An error that can occur when trying to borrow a value mutably.
pub enum BorrowMutError {
    /// The value was dropped.
    Dropped(ValueDroppedError),
    /// The value was already borrowed.
    AlreadyBorrowed(AlreadyBorrowedError),
    /// The value was already borrowed mutably.
    AlreadyBorrowedMut(AlreadyBorrowedMutError),
}

impl Display for BorrowMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowMutError::Dropped(error) => Display::fmt(error, f),
            BorrowMutError::AlreadyBorrowedMut(error) => Display::fmt(error, f),
            BorrowMutError::AlreadyBorrowed(error) => Display::fmt(error, f),
        }
    }
}

impl Error for BorrowMutError {}

/// An error that can occur when trying to use a value that has been dropped.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ValueDroppedError {
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    pub(crate) created_at: &'static std::panic::Location<'static>,
}

impl ValueDroppedError {
    /// Create a new `ValueDroppedError`.
    #[allow(unused)]
    pub fn new(created_at: &'static std::panic::Location<'static>) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            created_at,
        }
    }

    /// Create a new `ValueDroppedError` for a [`GenerationalLocation`].
    #[allow(unused)]
    pub(crate) fn new_for_location(location: GenerationalLocation) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            created_at: location.created_at,
        }
    }
}

impl Display for ValueDroppedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow because the value was dropped.")?;
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        f.write_fmt(format_args!("created_at: {}", self.created_at))?;
        Ok(())
    }
}

impl std::error::Error for ValueDroppedError {}

/// An error that can occur when trying to borrow a value that has already been borrowed mutably.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AlreadyBorrowedMutError {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrowed_mut_at: &'static std::panic::Location<'static>,
}

impl AlreadyBorrowedMutError {
    /// Create a new `AlreadyBorrowedMutError`.
    #[allow(unused)]
    pub fn new(borrowed_mut_at: &'static std::panic::Location<'static>) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_mut_at,
        }
    }
}

impl Display for AlreadyBorrowedMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow because the value was already borrowed mutably.")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        f.write_fmt(format_args!("borrowed_mut_at: {}", self.borrowed_mut_at))?;
        Ok(())
    }
}

impl std::error::Error for AlreadyBorrowedMutError {}

/// An error that can occur when trying to borrow a value mutably that has already been borrowed immutably.
#[derive(Debug, Clone, PartialEq)]
pub struct AlreadyBorrowedError {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrowed_at: Vec<&'static std::panic::Location<'static>>,
}

impl AlreadyBorrowedError {
    /// Create a new `AlreadyBorrowedError`.
    #[allow(unused)]
    pub fn new(borrowed_at: Vec<&'static std::panic::Location<'static>>) -> Self {
        Self {
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_at,
        }
    }
}

impl Display for AlreadyBorrowedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow mutably because the value was already borrowed immutably.")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        f.write_str("borrowed_at:")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        for location in self.borrowed_at.iter() {
            f.write_fmt(format_args!("\t{}", location))?;
        }
        Ok(())
    }
}

impl std::error::Error for AlreadyBorrowedError {}
