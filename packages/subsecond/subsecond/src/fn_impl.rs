use super::APP_JUMP_TABLE;
use std::{collections::HashMap, ffi::CStr, path::PathBuf, sync::Arc};

pub trait HotFunction<Args, Marker> {
    type Return;
    type Real;

    // rust-call isnt' stable, so we wrap the underyling call with our own, giving it a stable vtable entry
    fn call_it(&self, args: Args) -> Self::Return;

    // call this as if it were a real function pointer. This is very unsafe
    unsafe fn call_as_ptr(&self, _args: Args) -> Self::Return;
}

/*
todo: generate these with a macro_rules
*/
impl<T, R> HotFunction<(), ()> for T
where
    T: Fn() -> R,
{
    type Return = R;
    type Real = fn() -> R;
    fn call_it(&self, _args: ()) -> Self::Return {
        self()
    }
    unsafe fn call_as_ptr(&self, _args: ()) -> Self::Return {
        unsafe {
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let real = std::mem::transmute_copy::<Self, Self::Real>(&self);
                let known_fn_ptr = real as *const ();
                if let Some(ptr) = jump_table.map.get(&(known_fn_ptr as u64)).cloned() {
                    let ptr = ptr as *const ();
                    let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                    return detoured();
                }
            }

            self.call_it(_args)
        }
    }
}

pub struct FnAMarker;
impl<T, A, R> HotFunction<A, FnAMarker> for T
where
    T: Fn(A) -> R,
{
    type Return = R;
    type Real = fn(A) -> R;
    fn call_it(&self, _args: A) -> Self::Return {
        self(_args)
    }
    unsafe fn call_as_ptr(&self, _args: A) -> Self::Return {
        unsafe {
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let real = std::mem::transmute_copy::<Self, Self::Real>(&self);
                let known_fn_ptr = real as *const ();
                if let Some(ptr) = jump_table.map.get(&(known_fn_ptr as u64)).cloned() {
                    let ptr = ptr as *const ();
                    let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                    return detoured(_args);
                }
            }

            self.call_it(_args)
        }
    }
}

pub struct FnABMarker;
impl<T, A, B, R> HotFunction<(A, B), FnABMarker> for T
where
    T: Fn(A, B) -> R,
{
    type Return = R;
    type Real = fn(A, B) -> R;
    fn call_it(&self, args: (A, B)) -> Self::Return {
        self(args.0, args.1)
    }
    unsafe fn call_as_ptr(&self, args: (A, B)) -> Self::Return {
        unsafe {
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let real = std::mem::transmute_copy::<Self, Self::Real>(&self);
                let known_fn_ptr = real as *const ();
                if let Some(ptr) = jump_table.map.get(&(known_fn_ptr as u64)).cloned() {
                    let ptr = ptr as *const ();
                    let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                    return detoured(args.0, args.1);
                }
            }

            self.call_it(args)
        }
    }
}

pub struct FnABCMarker;
impl<T, A, B, C, R> HotFunction<(A, B, C), FnABCMarker> for T
where
    T: Fn(A, B, C) -> R,
{
    type Return = R;
    type Real = fn(A, B, C) -> R;
    fn call_it(&self, args: (A, B, C)) -> Self::Return {
        self(args.0, args.1, args.2)
    }
    unsafe fn call_as_ptr(&self, args: (A, B, C)) -> Self::Return {
        unsafe {
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let real = std::mem::transmute_copy::<Self, Self::Real>(&self);
                let known_fn_ptr = real as *const ();
                if let Some(ptr) = jump_table.map.get(&(known_fn_ptr as u64)).cloned() {
                    let ptr = ptr as *const ();
                    let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                    return detoured(args.0, args.1, args.2);
                }
            }

            self.call_it(args)
        }
    }
}
