//! A library for hot reloading functions.

use std::{
    ffi::{CStr, CString},
    sync::{Arc, Mutex},
};

use flashmap::{ReadHandle, WriteHandle};
use once_cell::sync::OnceCell;

#[test]
fn works() {
    fn hot_fn_demo() {
        println!("Hello, world!");
    }

    fn patched() {
        println!("Hello, patched!");
    }

    let mut rt = Runtime::initialize().unwrap();

    // Before patching...
    let f: fn() = Runtime::current(patched as fn());
    f();

    // Patching...
    unsafe {
        Runtime::patch([
            //
            ((patched as fn()).addr(), (hot_fn_demo as fn())),
        ])
    };

    // After patching...
    let f = Runtime::current(patched as fn());
    f();

    // Dump the current runtime
    Runtime::dump();
    println!("ptr of hot_fn_demo: {:#?}", hot_fn_demo as *const ());
    println!("ptr of patched: {:#?}", patched as *const ());
}

pub struct Runtime {}

static WRITER: OnceCell<Arc<Mutex<WriteHandle<usize, usize>>>> = OnceCell::new();
static READER: OnceCell<ReadHandle<usize, usize>> = OnceCell::new();

impl Runtime {
    pub fn initialize() -> Option<Self> {
        // If we're already initialized, return None
        if READER.get().is_some() {
            return None;
        }

        let (write, read) = flashmap::new::<usize, usize>();
        _ = READER.set(read);
        _ = WRITER.set(Arc::new(Mutex::new(write)));
        Some(Self {})
    }

    pub fn current<F: FnPtr>(f: F) -> F {
        let entry = READER.get().unwrap().guard().get(&f.addr()).cloned();

        // lazily fill
        if entry.is_none() {
            WRITER
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .guard()
                .insert(f.addr(), f.addr());
        }

        entry.map(|f| unsafe { F::from_addr(f) }).unwrap_or(f)
    }

    pub fn dump() {
        let reader = READER.get().unwrap().guard();
        for (k, v) in reader.iter() {
            println!("{:x?} -> {:x?}", k, v);
        }
    }

    pub unsafe fn patch<F: FnPtr>(items: impl IntoIterator<Item = (usize, F)>) {
        let mut handle = WRITER.get().unwrap().lock().unwrap();

        let mut guard = handle.guard();
        for (key, value) in items {
            guard.insert(key, value.addr());
        }
    }

    pub fn patch_from_binary(lib: &libloading::Library) {
        let mut handle = WRITER.get().unwrap().lock().unwrap();
        let mut guard = handle.guard();

        let ptrs: Vec<_> = guard.iter().map(|(k, v)| (*k, *v)).collect();
        for (k, v) in ptrs {
            let mut addr_info = libc::Dl_info {
                dli_fname: std::ptr::null_mut(),
                dli_fbase: std::ptr::null_mut(),
                dli_sname: std::ptr::null_mut(),
                dli_saddr: std::ptr::null_mut(),
            };
            let addr = k as *const ();
            unsafe {
                libc::dladdr(addr as *const _, &mut addr_info);
                let sname = CStr::from_ptr(addr_info.dli_sname as _);
                println!("Patching {:#?} to {:#?}", sname, v);
                let Ok(sym) = lib.get::<fn() -> ()>(sname.to_bytes()) else {
                    println!("Failed to find {:#?}", sname);
                    continue;
                };
                let ptr = sym.try_as_raw_ptr().unwrap() as *mut ();
                guard.insert(k, ptr as usize);
            }

            // println!("addr: {:#?}", addr);
            // println!("dli_fname: {:#?}", CStr::from_ptr(addr_info.dli_fname as _));
            // println!("dli_fbase: {:#?}", CStr::from_ptr(addr_info.dli_fbase as _));
            // println!("dli_sname: {:#?}", CStr::from_ptr(addr_info.dli_sname as _));
            // println!("dli_saddr: {:#?}", addr_info.dli_saddr);
        }
    }
}

trait FnPtr: Sized {
    fn addr(&self) -> usize;
    unsafe fn from_addr(addr: usize) -> Self;
}
impl<R> FnPtr for fn() -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<'a, R, A: ?Sized + 'a> FnPtr for fn(A) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<'a, R, A: ?Sized + 'a, B: ?Sized + 'a> FnPtr for fn(A, B) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<R, A: ?Sized, B: ?Sized, C: ?Sized> FnPtr for fn(A, B, C) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<R, A: ?Sized, B: ?Sized, C: ?Sized, D: ?Sized> FnPtr for fn(A, B, C, D) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<R, A: ?Sized, B: ?Sized, C: ?Sized, D: ?Sized, E: ?Sized> FnPtr for fn(A, B, C, D, E) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<R, A, B, C, D, E, F> FnPtr for fn(A, B, C, D, E, F) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
impl<R, A, B, C, D, E, F, G> FnPtr for fn(A, B, C, D, E, F, G) -> R {
    fn addr(&self) -> usize {
        *self as *const () as usize
    }
    unsafe fn from_addr(addr: usize) -> Self {
        unsafe { std::mem::transmute(addr) }
    }
}
