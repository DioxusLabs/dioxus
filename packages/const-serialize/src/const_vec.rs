#![allow(dead_code)]
use std::{fmt::Debug, mem::MaybeUninit};

const MAX_SIZE: usize = 2usize.pow(10);

pub struct ConstVec<T> {
    memory: [MaybeUninit<T>; MAX_SIZE],
    len: usize,
}

impl<T> Default for ConstVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Debug for ConstVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstVec")
            .field("len", &self.len)
            .field("memory", &self.as_ref())
            .finish()
    }
}

impl<T> ConstVec<T> {
    pub const fn new() -> Self {
        Self {
            memory: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    pub const fn push(mut self, value: T) -> Self {
        self.memory[self.len] = MaybeUninit::new(value);
        self.len += 1;
        self
    }

    pub const fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(unsafe { &*self.memory[index].as_ptr() })
        } else {
            None
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn as_ref(&self) -> &[T] {
        unsafe { &*(self.memory.split_at(self.len).0 as *const [MaybeUninit<T>] as *const [T]) }
    }

    pub const fn swap(mut self, first: usize, second: usize) -> Self
    where
        T: Copy,
    {
        assert!(first < self.len);
        assert!(second < self.len);
        let temp = self.memory[first];
        self.memory[first] = self.memory[second];
        self.memory[second] = temp;
        self
    }

    pub const fn pop(mut self) -> (Self, Option<T>)
    where
        T: Copy,
    {
        let value = if self.len > 0 {
            self.len -= 1;
            let last = self.len;
            let last_value = unsafe { self.memory[last].assume_init() };
            Some(last_value)
        } else {
            None
        };
        (self, value)
    }

    pub const fn remove(mut self, index: usize) -> (Self, T)
    where
        T: Copy,
    {
        assert!(index < self.len);
        let value = unsafe { self.memory[index].assume_init() };
        let mut swap_index = index;
        while swap_index + 1 < self.len {
            self.memory[swap_index] = self.memory[swap_index + 1];
            swap_index += 1;
        }
        self.len -= 1;
        (self, value)
    }

    pub const fn set(mut self, index: usize, value: T) -> Self {
        if index >= self.len {
            panic!("Out of bounds")
        }
        self.memory[index] = MaybeUninit::new(value);
        self
    }

    pub const fn into_parts(self) -> ([MaybeUninit<T>; MAX_SIZE], usize) {
        (self.memory, self.len)
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
    assert_eq!(value, 1234);
    assert_eq!(vec.as_ref(), &[5678]);
    let (vec, value) = vec.remove(0);
    assert_eq!(value, 5678);
    assert_eq!(vec.as_ref(), &[]);
}
