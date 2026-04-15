use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    rc::Rc,
};

use dioxus::signals::{
    AnyStorage, CopyValue, Readable, ReadableExt, ReadableRef, SyncStorage, UnsyncStorage,
    WritableExt, WritableRef, WriteLock,
};
use generational_box::GenerationalRef;

pub trait Get<Value> {
    fn get(&self) -> Value;
}

pub struct Signal<A> {
    access: A,
}

impl<A: Clone> Clone for Signal<A> {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
        }
    }
}

impl<T: 'static> Signal<RwRoot<T>> {
    pub fn new(value: T) -> Self {
        Self {
            access: RwRoot {
                cell: CopyValue::new(value),
            },
        }
    }
}

impl<A> Signal<A> {
    pub fn read<T>(&self) -> ReadableRef<'_, CopyValue<T>>
    where
        A: Get<ReadableRef<'static, CopyValue<T>>>,
        T: 'static,
    {
        self.access.get()
    }

    pub fn write<T>(&self) -> WritableRef<'_, CopyValue<T>>
    where
        A: Get<WriteLock<'static, T, UnsyncStorage>>,
        T: 'static,
    {
        WriteLock::downcast_lifetime(self.access.get())
    }
}

pub struct RwRoot<T> {
    cell: CopyValue<T>,
}

impl<T> Clone for RwRoot<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T> Get<GenerationalRef<Ref<'static, T>>> for RwRoot<T> {
    fn get(&self) -> GenerationalRef<Ref<'static, T>> {
        self.cell.read_unchecked()
    }
}

impl<T> Get<WriteLock<'static, T, UnsyncStorage>> for RwRoot<T> {
    fn get(&self) -> WritableRef<'static, CopyValue<T>> {
        self.cell.write_unchecked()
    }
}

fn main() {
    let value = Signal::new(0);
    let read = value.read();
    let write = value.write();
}
