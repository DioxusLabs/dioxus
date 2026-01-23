#![allow(dead_code)]
use std::{fmt::Debug, hash::Hash, mem::MaybeUninit};

use crate::ConstReadBuffer;

const DEFAULT_MAX_SIZE: usize = 2usize.pow(10);

/// [`ConstVec`] is a version of [`Vec`] that is usable in const contexts. It has
/// a fixed maximum size, but it can grow and shrink within that size limit
/// as needed.
///
/// # Example
/// ```rust
/// # use const_serialize::ConstVec;
/// const EMPTY: ConstVec<u8> = ConstVec::new();
/// // Methods that mutate the vector will return a new vector
/// const ONE: ConstVec<u8> = EMPTY.push(1);
/// const TWO: ConstVec<u8> = ONE.push(2);
/// const THREE: ConstVec<u8> = TWO.push(3);
/// const FOUR: ConstVec<u8> = THREE.push(4);
/// // If a value is also returned, that will be placed in a tuple in the return value
/// // along with the new vector
/// const POPPED: (ConstVec<u8>, Option<u8>) = FOUR.pop();
/// assert_eq!(POPPED.0, THREE);
/// assert_eq!(POPPED.1.unwrap(), 4);
/// ```
pub struct ConstVec<T, const MAX_SIZE: usize = DEFAULT_MAX_SIZE> {
    memory: [MaybeUninit<T>; MAX_SIZE],
    len: u32,
}

impl<T: Clone, const MAX_SIZE: usize> Clone for ConstVec<T, MAX_SIZE> {
    fn clone(&self) -> Self {
        let mut cloned = Self::new_with_max_size();
        for i in 0..self.len as usize {
            cloned = cloned.push(self.get(i).unwrap().clone());
        }
        cloned
    }
}

impl<T: Copy, const MAX_SIZE: usize> Copy for ConstVec<T, MAX_SIZE> {}

impl<T: PartialEq, const MAX_SIZE: usize> PartialEq for ConstVec<T, MAX_SIZE> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<T: Hash, const MAX_SIZE: usize> Hash for ConstVec<T, MAX_SIZE> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T, const MAX_SIZE: usize> Default for ConstVec<T, MAX_SIZE> {
    fn default() -> Self {
        Self::new_with_max_size()
    }
}

impl<T: Debug, const MAX_SIZE: usize> Debug for ConstVec<T, MAX_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstVec")
            .field("len", &self.len)
            .field("memory", &self.as_ref())
            .finish()
    }
}

impl<T> ConstVec<T> {
    /// Create a new empty [`ConstVec`]
    pub const fn new() -> Self {
        Self::new_with_max_size()
    }
}

impl<T, const MAX_SIZE: usize> ConstVec<T, MAX_SIZE> {
    /// Create a new empty [`ConstVec`] with a custom maximum size
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8, 10> = ConstVec::new_with_max_size();
    /// ```
    pub const fn new_with_max_size() -> Self {
        Self {
            memory: [const { MaybeUninit::uninit() }; MAX_SIZE],
            len: 0,
        }
    }

    /// Push a value onto the end of the [`ConstVec`]
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// assert_eq!(ONE.as_ref(), &[1]);
    /// ```
    pub const fn push(mut self, value: T) -> Self {
        self.memory[self.len as usize] = MaybeUninit::new(value);
        self.len += 1;
        self
    }

    /// Extend the [`ConstVec`] with the contents of a slice
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.extend(&[1, 2, 3]);
    /// assert_eq!(ONE.as_ref(), &[1, 2, 3]);
    /// ```
    pub const fn extend(mut self, other: &[T]) -> Self
    where
        T: Copy,
    {
        let mut i = 0;
        while i < other.len() {
            self = self.push(other[i]);
            i += 1;
        }
        self
    }

    /// Get a reference to the value at the given index
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// assert_eq!(ONE.get(0), Some(&1));
    /// ```
    pub const fn get(&self, index: usize) -> Option<&T> {
        if index < self.len as usize {
            Some(unsafe { &*self.memory[index].as_ptr() })
        } else {
            None
        }
    }

    /// Get the length of the [`ConstVec`]
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// assert_eq!(ONE.len(), 1);
    /// ```
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if the [`ConstVec`] is empty
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// assert!(EMPTY.is_empty());
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// assert!(!ONE.is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a reference to the underlying slice
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// assert_eq!(ONE.as_ref(), &[1]);
    /// ```
    pub const fn as_ref(&self) -> &[T] {
        unsafe {
            &*(self.memory.split_at(self.len as usize).0 as *const [MaybeUninit<T>] as *const [T])
        }
    }

    /// Swap the values at the given indices
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.push(2);
    /// const THREE: ConstVec<u8> = TWO.swap(0, 1);
    /// assert_eq!(THREE.as_ref(), &[2, 1]);
    /// ```
    pub const fn swap(mut self, first: usize, second: usize) -> Self
    where
        T: Copy,
    {
        assert!(first < self.len as usize);
        assert!(second < self.len as usize);
        let temp = self.memory[first];
        self.memory[first] = self.memory[second];
        self.memory[second] = temp;
        self
    }

    /// Pop a value off the end of the [`ConstVec`]
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.push(2);
    /// const THREE: ConstVec<u8> = TWO.push(3);
    /// const POPPED: (ConstVec<u8>, Option<u8>) = THREE.pop();
    /// assert_eq!(POPPED.0, TWO);
    /// assert_eq!(POPPED.1.unwrap(), 3);
    /// ```
    pub const fn pop(mut self) -> (Self, Option<T>)
    where
        T: Copy,
    {
        let value = if self.len > 0 {
            self.len -= 1;
            let last = self.len as usize;
            let last_value = unsafe { self.memory[last].assume_init() };
            Some(last_value)
        } else {
            None
        };
        (self, value)
    }

    /// Remove the value at the given index
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.push(2);
    /// const THREE: ConstVec<u8> = TWO.push(3);
    /// const REMOVED: (ConstVec<u8>, Option<u8>) = THREE.remove(1);
    /// assert_eq!(REMOVED.0.as_ref(), &[1, 3]);
    /// assert_eq!(REMOVED.1.unwrap(), 2);
    /// ```
    pub const fn remove(mut self, index: usize) -> (Self, Option<T>)
    where
        T: Copy,
    {
        let value = if index < self.len as usize {
            let value = unsafe { self.memory[index].assume_init() };
            let mut swap_index = index;
            while swap_index + 1 < self.len as usize {
                self.memory[swap_index] = self.memory[swap_index + 1];
                swap_index += 1;
            }
            self.len -= 1;
            Some(value)
        } else {
            None
        };

        (self, value)
    }

    /// Set the value at the given index
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.set(0, 2);
    /// assert_eq!(TWO.as_ref(), &[2]);
    /// ```
    pub const fn set(mut self, index: usize, value: T) -> Self {
        if index >= self.len as usize {
            panic!("Out of bounds")
        }
        self.memory[index] = MaybeUninit::new(value);
        self
    }

    pub(crate) const fn into_parts(self) -> ([MaybeUninit<T>; MAX_SIZE], usize) {
        (self.memory, self.len as usize)
    }

    /// Split the [`ConstVec`] into two at the given index
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::ConstVec;
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.push(2);
    /// const THREE: ConstVec<u8> = TWO.push(3);
    /// const SPLIT: (ConstVec<u8>, ConstVec<u8>) = THREE.split_at(1);
    /// assert_eq!(SPLIT.0.as_ref(), &[1]);
    /// assert_eq!(SPLIT.1.as_ref(), &[2, 3]);
    /// ```
    pub const fn split_at(&self, index: usize) -> (Self, Self)
    where
        T: Copy,
    {
        assert!(index <= self.len as usize);
        let slice = self.as_ref();
        let (left, right) = slice.split_at(index);
        let mut left_vec = Self::new_with_max_size();
        let mut i = 0;
        while i < left.len() {
            left_vec = left_vec.push(left[i]);
            i += 1;
        }
        let mut right_vec = Self::new_with_max_size();
        i = 0;
        while i < right.len() {
            right_vec = right_vec.push(right[i]);
            i += 1;
        }
        (left_vec, right_vec)
    }
}

impl<const MAX_SIZE: usize> ConstVec<u8, MAX_SIZE> {
    /// Convert the [`ConstVec`] into a [`ConstReadBuffer`]
    ///
    /// # Example
    /// ```rust
    /// # use const_serialize::{ConstVec, ConstReadBuffer};
    /// const EMPTY: ConstVec<u8> = ConstVec::new();
    /// const ONE: ConstVec<u8> = EMPTY.push(1);
    /// const TWO: ConstVec<u8> = ONE.push(2);
    /// const READ: ConstReadBuffer = TWO.read();
    /// ```
    pub const fn read(&self) -> ConstReadBuffer<'_> {
        ConstReadBuffer::new(self.as_ref())
    }
}

#[test]
fn test_const_vec() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec
    };
    assert_eq!(VEC.as_ref(), &[1234, 5678]);
    let vec = VEC;
    let (vec, value) = vec.pop();
    assert_eq!(value, Some(5678));
    let (vec, value) = vec.pop();
    assert_eq!(value, Some(1234));
    let (vec, value) = vec.pop();
    assert_eq!(value, None);
    assert_eq!(vec.as_ref(), &[]);
}

#[test]
fn test_const_vec_len() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec
    };
    assert_eq!(VEC.len(), 2);
}

#[test]
fn test_const_vec_get() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec
    };
    assert_eq!(VEC.get(0), Some(&1234));
    assert_eq!(VEC.get(1), Some(&5678));
    assert_eq!(VEC.get(2), None);
}

#[test]
fn test_const_vec_swap() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec
    };
    let mut vec = VEC;
    assert_eq!(vec.as_ref(), &[1234, 5678]);
    vec = vec.swap(0, 1);
    assert_eq!(vec.as_ref(), &[5678, 1234]);
    vec = vec.swap(0, 1);
    assert_eq!(vec.as_ref(), &[1234, 5678]);
}

#[test]
fn test_const_vec_remove() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec
    };
    let vec = VEC;
    println!("{:?}", vec);
    assert_eq!(vec.as_ref(), &[1234, 5678]);
    let (vec, value) = vec.remove(0);
    assert_eq!(value, Some(1234));
    assert_eq!(vec.as_ref(), &[5678]);
    let (vec, value) = vec.remove(0);
    assert_eq!(value, Some(5678));
    assert_eq!(vec.as_ref(), &[]);
}

#[test]
fn test_const_vec_extend() {
    const VEC: ConstVec<u32> = {
        let mut vec = ConstVec::new();
        vec = vec.push(1234);
        vec = vec.push(5678);
        vec = vec.extend(&[91011, 1213]);
        vec
    };
    let vec = VEC;
    println!("{:?}", vec);
    assert_eq!(vec.as_ref(), &[1234, 5678, 91011, 1213]);
}
