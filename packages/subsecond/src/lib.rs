use std::{collections::HashMap, ffi::CStr, path::PathBuf, sync::Arc};

// todo: if there's a reference held while we run our patch, this gets invalidated. should probably
// be a pointer to a jump table instead, behind a cell or something. I believe Atomic + relaxed is basically a no-op
static mut APP_JUMP_TABLE: Option<JumpTable> = None;
static mut HOTRELOAD_HANDLERS: Vec<Arc<dyn Fn()>> = vec![];

pub const fn current<A, M, F>(f: F) -> HotFn<A, M, F>
where
    F: SomeFn<A, M>,
{
    HotFn {
        inner: f,
        _marker: std::marker::PhantomData,
    }
}

pub struct HotFn<A, M, T: SomeFn<A, M>> {
    inner: T,
    _marker: std::marker::PhantomData<(A, M)>,
}

impl<A, M, T: SomeFn<A, M>> HotFn<A, M, T> {
    pub fn call(&self, args: A) -> T::Return {
        unsafe {
            // Try to handle known function pointers. This is *really really* unsafe, but due to how
            // rust trait objects work, it's impossible to make an arbitrary usize-sized type implement Fn()
            // since that would require a vtable pointer, pushing out the bounds of the pointer size.
            if size_of::<T>() == size_of::<fn() -> ()>() {
                return self.inner.call_as_ptr(args);
            }

            // Handle trait objects. This will occur for sizes other than usize. Normal rust functions
            // become ZST's and thus their <T as SomeFn>::call becomes a function pointer to the function.
            //
            // For non-zst (trait object) types, then there might be an issue. The real call function
            // will likely end up in the vtable and will never be hot-reloaded since signature takes self.
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let known_fn_ptr = <T as SomeFn<A, M>>::call_it as *const ();
                let ptr = jump_table.map.get(&(known_fn_ptr as u64)).unwrap().clone() as *const ();

                // https://stackoverflow.com/questions/46134477/how-can-i-call-a-raw-address-from-rust
                let _f = std::mem::transmute::<*const (), fn(&T, A) -> T::Return>(ptr);
                _f(&self.inner, args)
            } else {
                self.inner.call_it(args)
            }
        }
    }
}

pub trait SomeFn<Args, Marker> {
    type Return;
    type Real;

    // rust-call isnt' stable, so we wrap the underyling call with our own, giving it a stable vtable entry
    fn call_it(&self, args: Args) -> Self::Return;

    // call this as if it were a real function pointer. This is very unsafe
    unsafe fn call_as_ptr(&self, _args: Args) -> Self::Return;
}

impl<T, R> SomeFn<(), ()> for T
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
                let ptr = jump_table.map.get(&(known_fn_ptr as u64)).unwrap().clone() as *const ();
                let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                detoured()
            } else {
                self.call_it(_args)
            }
        }
    }
}

pub struct FnAMarker;
impl<T, A, R> SomeFn<A, FnAMarker> for T
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
                let ptr = jump_table.map.get(&(known_fn_ptr as u64)).unwrap().clone() as *const ();
                let detoured = std::mem::transmute::<*const (), Self::Real>(ptr);
                detoured(_args)
            } else {
                self.call_it(_args)
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn hotfn_load_binary_patch(path: *const i8, jump_table_path: *const i8) {
    let patch = PathBuf::from(unsafe { CStr::from_ptr(path).to_str().unwrap() });
    let jump_table = PathBuf::from(unsafe { CStr::from_ptr(jump_table_path).to_str().unwrap() });
    run_patch(patch, jump_table)
}

#[derive(serde::Deserialize, Debug)]
pub struct JumpTable {
    pub map: HashMap<u64, u64>,
    pub main_address: u64,
}

/// Run the patch
pub fn run_patch(patch: PathBuf, jump_table: PathBuf) {
    let lib = unsafe { libloading::os::unix::Library::new(patch).unwrap() };
    let lib = Box::leak(Box::new(lib));

    // Load the jump table by deserializing it from the file
    let mut jump_table: JumpTable =
        bincode::deserialize(&std::fs::read(jump_table).unwrap()).unwrap();

    // Correct the jump table since dlopen will load the binary at a different address than the original
    let dl_offset = unsafe { lib.get::<*const ()>(b"main") }
        .unwrap()
        .as_raw_ptr()
        .wrapping_sub(jump_table.main_address as usize);

    // Modify the jump table to be relative to the base address of the loaded library
    for new in jump_table.map.values_mut() {
        *new += dl_offset as u64;
    }

    unsafe { APP_JUMP_TABLE = Some(jump_table) }

    // And then call the original main function
    for handler in unsafe { HOTRELOAD_HANDLERS.iter() } {
        handler();
    }
}
