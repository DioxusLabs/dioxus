use std::{collections::HashMap, ffi::CStr, os::raw::c_void, path::PathBuf, sync::Arc};

pub use subsecond_macro::hot;
pub use subsecond_types::JumpTable;

mod fn_impl;
use fn_impl::*;

// todo: if there's a reference held while we run our patch, this gets invalidated. should probably
// be a pointer to a jump table instead, behind a cell or something. I believe Atomic + relaxed is basically a no-op
static mut APP_JUMP_TABLE: Option<JumpTable> = None;
static mut HOTRELOAD_HANDLERS: Vec<Arc<dyn Fn()>> = vec![];

pub const fn current<A, M, F>(f: F) -> HotFn<A, M, F>
where
    F: HotFunction<A, M>,
{
    HotFn {
        inner: f,
        _marker: std::marker::PhantomData,
    }
}

pub struct HotFn<A, M, T: HotFunction<A, M>> {
    inner: T,
    _marker: std::marker::PhantomData<(A, M)>,
}

impl<A, M, T: HotFunction<A, M>> HotFn<A, M, T> {
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
                let known_fn_ptr = <T as HotFunction<A, M>>::call_it as *const ();
                if let Some(ptr) = jump_table.map.get(&(known_fn_ptr as u64)).cloned() {
                    let _f = std::mem::transmute::<_, fn(&T, A) -> T::Return>(ptr as *const ());
                    return _f(&self.inner, args);
                }
            }

            self.inner.call_it(args)
        }
    }
}

#[no_mangle]
pub extern "C" fn hotfn_load_binary_patch(path: *const i8, jump_table_path: *const i8) {
    let patch = PathBuf::from(unsafe { CStr::from_ptr(path).to_str().unwrap() });
    let jump_table = PathBuf::from(unsafe { CStr::from_ptr(jump_table_path).to_str().unwrap() });
    let jump_table: JumpTable = bincode::deserialize(&std::fs::read(jump_table).unwrap()).unwrap();
    run_patch(patch, jump_table)
}

/// Apply the patch using the jump table
pub fn run_patch(patch: PathBuf, mut jump_table: JumpTable) {
    let lib = unsafe { libloading::os::unix::Library::new(patch).unwrap() };
    let lib = Box::leak(Box::new(lib));

    let old_dl_offset =
        aslr_offset_of_library(&libloading::os::unix::Library::this().into(), &jump_table)
            .expect("Could not find ASLR offset");

    let new_dl_offset = unsafe { lib.get::<*const ()>(b"main") }
        .unwrap()
        .as_raw_ptr()
        .wrapping_sub(jump_table.new_main_address as usize);

    // Modify the jump table to be relative to the base address of the loaded library
    jump_table.map = jump_table
        .map
        .iter()
        .map(|(k, v)| {
            (
                (*k as usize + old_dl_offset as usize) as u64,
                *v + new_dl_offset as u64,
            )
        })
        .collect();

    unsafe { APP_JUMP_TABLE = Some(jump_table) }

    // And then call the original main function
    for handler in unsafe { HOTRELOAD_HANDLERS.iter() } {
        handler();
    }
}

pub fn register_handler(handler: Arc<dyn Fn() + Send + Sync + 'static>) {
    unsafe {
        HOTRELOAD_HANDLERS.push(handler);
    }
}

/// Get the offset of the library in the address space of the current process
///
/// Attempts to use a known symbol on the platform, and if not, falls back to using `main`
fn aslr_offset_of_library(lib: &libloading::Library, table: &JumpTable) -> Option<*mut c_void> {
    #[cfg(target_os = "macos")]
    return {
        unsafe {
            lib.get::<*const ()>(b"_mh_execute_header")
                .unwrap()
                .try_as_raw_ptr()
                .map(|ptr| ptr.wrapping_sub(0x100000000))
        }
    };

    None
}
