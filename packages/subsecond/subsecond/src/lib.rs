use std::{
    any::TypeId,
    collections::HashMap,
    ffi::CStr,
    os::raw::c_void,
    panic::{AssertUnwindSafe, UnwindSafe},
    path::PathBuf,
    sync::Arc,
};

pub use subsecond_macro::hot;
pub use subsecond_types::JumpTable;

mod fn_impl;
use fn_impl::*;

// todo: if there's a reference held while we run our patch, this gets invalidated. should probably
// be a pointer to a jump table instead, behind a cell or something. I believe Atomic + relaxed is basically a no-op
static mut APP_JUMP_TABLE: Option<JumpTable> = None;
static mut HOTRELOAD_HANDLERS: Vec<Arc<dyn Fn()>> = vec![];
static mut CHANGED: bool = false;
static mut UNWINDING: bool = false;
static mut SUBSECOND_ENABLED: bool = false;

/// Run the given function in a subsecond root context
///
/// Whenever the code changes, the function will be re-executed
///
/// Generally you might block inside the function, so downstream subsecond code might throw a panic.
///
/// This will catch those panics and re-execute the function.
///
/// Calling this function registers this location is a "resume" point if subsecond is enabled.
///
/// Unfortunately, panic unwinding is not implemented for WASM, so resume points won't work for WASM
/// targets. Fortunately, subsecond is still quite usable without resume points.
pub fn resume<T>(mut f: impl FnMut() -> T) -> T {
    loop {
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| f()));

        match res {
            Ok(res) => return res,
            Err(err) => {
                // If this is *our* panic, then we need to re-run the function
                // Othwerwise, we just return the result
                unsafe {
                    if UNWINDING {
                        UNWINDING = false;
                        continue;
                    }
                }

                // If we're not manually unwinding, then it's their panic
                // We issue a sigstop to the process so it can be debugged
                unsafe {
                    if SUBSECOND_ENABLED {
                        // todo: wait for the new patch to be applied
                        continue;
                    }
                }

                std::panic::resume_unwind(err);
            }
        }
    }
}

pub const fn current<A, M, F>(f: F) -> HotFn<A, M, F>
where
    F: HotFunction<A, M> + 'static,
{
    HotFn {
        inner: f,
        _marker: std::marker::PhantomData,
    }
}

pub fn call<O: 'static>(f: impl FnMut() -> O + 'static) -> O {
    let mut f = current(f);
    f.call(())
}

pub struct HotFn<A, M, T: HotFunction<A, M>> {
    inner: T,
    _marker: std::marker::PhantomData<(A, M)>,
}

impl<A, M, T: HotFunction<A, M> + 'static> HotFn<A, M, T> {
    pub fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    pub fn call(&mut self, args: A) -> T::Return {
        // If we need to unwind, then let's throw a panic
        // This will occur when the pending patch is "over our head" and needs to be applied to a
        // "resume point". We can eventually look into migrating the datastructures over but for now
        // the resume point will force the struct to be re-built.

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
                    // let _f = std::mem::transmute::<_, fn(&T, A) -> T::Return>(ptr as *const ());
                    // return _f(&self.inner, args);
                    let ptr = ptr as *const ();
                    let _f = std::mem::transmute::<*const (), fn(&T, A) -> T::Return>(ptr);
                    return _f(&self.inner, args);
                }
            }

            self.inner.call_it(args)
        }
    }
}

/// Apply the patch using the jump table
pub fn run_patch(mut jump_table: JumpTable) {
    let lib = unsafe { libloading::os::unix::Library::new(&jump_table.lib).unwrap() };
    let lib = Box::leak(Box::new(lib));

    let old_dl_offset = unsafe {
        libloading::os::unix::Library::this()
            .get::<*const ()>(b"_mh_execute_header")
            .unwrap()
            .as_raw_ptr()
            .wrapping_sub(0x100000000)
    };

    let new_dl_offset = unsafe {
        lib.get::<*const ()>(b"_mh_execute_header")
            .unwrap()
            .as_raw_ptr()
            .wrapping_sub(0x100000000)
    };
    // let new_dl_offset = unsafe { lib.get::<*const ()>(b"main") }
    //     .unwrap()
    //     .as_raw_ptr()
    //     .wrapping_sub(jump_table.new_main_address as usize);

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

    unsafe { APP_JUMP_TABLE = Some(jump_table) };

    unsafe { CHANGED = true };

    // And then call the original main function
    for handler in unsafe { HOTRELOAD_HANDLERS.clone() } {
        handler();
    }
}

pub fn register_handler(handler: Arc<dyn Fn() + Send + Sync + 'static>) {
    unsafe {
        HOTRELOAD_HANDLERS.push(handler);
    }
}

pub fn changed() -> bool {
    let changed = unsafe { CHANGED };
    unsafe { CHANGED = false };
    changed
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
