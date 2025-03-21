use std::{
    any::TypeId,
    collections::HashMap,
    ffi::CStr,
    os::raw::c_void,
    panic::{panic_any, AssertUnwindSafe, UnwindSafe},
    path::PathBuf,
    sync::Arc,
};

pub use subsecond_macro::hot;
pub use subsecond_types::JumpTable;

mod macho;
mod unix;
mod wasm;
mod windows;

pub mod prelude {
    pub use subsecond_macro::hot;
}

mod fn_impl;
use fn_impl::*;

// todo: if there's a reference held while we run our patch, this gets invalidated. should probably
// be a pointer to a jump table instead, behind a cell or something. I believe Atomic + relaxed is basically a no-op
static mut APP_JUMP_TABLE: Option<JumpTable> = None;
static mut HOTRELOAD_HANDLERS: Vec<Arc<dyn Fn()>> = vec![];
static mut CHANGED: bool = false;
static mut SUBSECOND_ENABLED: bool = false;

/// Call a given function with hot-reloading enabled. If the function's code changes, `call` will use
/// the new version of the function. If code *above* the function changes, this will emit a panic
/// that forces an unwind to the next `Subsecond::call` instance.
///
/// # Example
///
///
/// # Without unwinding
///
///
/// # WebAssembly
///
/// WASM/rust does not support unwinding, so `Subsecond::call` will not track dependency graph changes.
/// If you are building a framework for use on WASM, you will need to use `Subsecond::HotFn` directly.
///
/// However, if you wrap your calling code in a future, you *can* simply drop the future which will
/// cause `drop` to execute and get something similar to unwinding. Not great if refcells are open.
pub fn call<O>(f: impl FnMut() -> O) -> O {
    let mut hotfn = current(f);

    loop {
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| hotfn.call(())));

        // If the call succeeds just return the result, otherwise we try to handle the panic if its our own.
        let err = match res {
            Ok(res) => return res,
            Err(err) => err,
        };

        // If this is our panic then let's handle it, otherwise we just resume unwinding
        let Some(hot_payload) = err.downcast_ref::<HotFnPanic>() else {
            std::panic::resume_unwind(err);
        };

        // If we're not manually unwinding, then it's their panic
        // We issue a sigstop to the process so it can be debugged
        unsafe {
            if SUBSECOND_ENABLED {
                // todo: wait for the new patch to be applied
                continue;
            }
        }
    }
}

pub const fn current<A, M, F>(f: F) -> HotFn<A, M, F>
where
    F: HotFunction<A, M>,
{
    HotFn {
        inner: f,
        _marker: std::marker::PhantomData,
    }
}

pub struct HotFnPanic {}

pub struct HotFn<A, M, T: HotFunction<A, M>> {
    inner: T,
    _marker: std::marker::PhantomData<(A, M)>,
}

impl<A, M, T: HotFunction<A, M>> HotFn<A, M, T> {
    pub fn call(&mut self, args: A) -> T::Return {
        // If we need to unwind, then let's throw a panic
        // This will occur when the pending patch is "over our head" and needs to be applied to a
        // "resume point". We can eventually look into migrating the datastructures over but for now
        // the resume point will force the struct to be re-built.
        // panic_any()

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
                    let ptr = ptr as *const ();
                    let true_fn = std::mem::transmute::<*const (), fn(&T, A) -> T::Return>(ptr);
                    return true_fn(&self.inner, args);
                }
            }

            self.inner.call_it(args)
        }
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

/// Apply the patch using the jump table.
///
/// # Safety
///
/// This function is unsafe because it is detouring existing functions in memory. This is wildly unsafe,
/// especially if the JumpTable is malformed. Only run this if you know what you're doing.
pub unsafe fn run_patch(jump_table: JumpTable) {
    // On non-wasm platforms we can just use libloading and the known aslr offsets to load the library
    #[cfg(any(unix, windows))]
    let jump_table = relocate_native_jump_table(jump_table);

    // On wasm we need to do a lot more work - merging our ifunc table, etc
    #[cfg(target_arch = "wasm32")]
    let jump_table = relocate_wasm_jump_table(jump_table);

    // Update runtime state
    unsafe {
        APP_JUMP_TABLE = Some(jump_table);
        CHANGED = true;
        HOTRELOAD_HANDLERS.clone().iter().for_each(|handler| {
            handler();
        });
    }
}

#[cfg(any(unix, windows))]
fn relocate_native_jump_table(mut jump_table: JumpTable) -> JumpTable {
    let old_offset = alsr_offset(
        jump_table.old_base_address as usize,
        #[cfg(unix)]
        libloading::os::unix::Library::this(),
        #[cfg(windows)]
        libloading::os::windows::Library::this().unwrap(),
    )
    .unwrap();

    let new_offset = alsr_offset(
        jump_table.new_base_address as usize,
        #[cfg(unix)]
        unsafe { libloading::os::unix::Library::new(&jump_table.lib).unwrap() }.into(),
        #[cfg(windows)]
        unsafe { libloading::Library::new(&jump_table.lib).unwrap() }.into(),
    )
    .unwrap();

    // Modify the jump table to be relative to the base address of the loaded library
    jump_table.map = jump_table
        .map
        .iter()
        .map(|(k, v)| {
            (
                (*k + old_offset as u64) as u64,
                (*v + new_offset as u64) as u64,
            )
        })
        .collect();

    jump_table
}

fn load_android_lib(path: &PathBuf) -> *mut c_void {
    use std::ffi::{c_char, c_int, c_void, CString};
    use std::mem;
    use std::ptr;

    #[repr(C)]
    struct AndroidNamespaceT {
        _private: [u8; 0],
    }

    #[repr(C)]
    struct AndroidDlextinfo {
        flags: u64,
        reserved_addr: *mut c_void,
        reserved_size: usize,
        library_namespace: *mut AndroidNamespaceT,
    }

    const ANDROID_DLEXT_USE_NAMESPACE: u64 = 0x80;
    const RTLD_NOW: c_int = 2;
    const RTLD_DEFAULT: *mut c_void = 0 as *mut c_void;

    type AndroidDlopenExtFn =
        unsafe extern "C" fn(*const c_char, c_int, *const AndroidDlextinfo) -> *mut c_void;
    type AndroidGetExportedNamespaceFn =
        unsafe extern "C" fn(*const c_char) -> *mut AndroidNamespaceT;

    extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlerror() -> *const c_char;
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    }

    pub fn load_in_app_namespace(library_path: &str) -> Result<*mut c_void, String> {
        unsafe {
            // Get function pointers dynamically
            let android_dlopen_ext_name = CString::new("android_dlopen_ext").unwrap();
            let android_get_exported_namespace_name =
                CString::new("android_get_exported_namespace").unwrap();

            let android_dlopen_ext_ptr = dlsym(RTLD_DEFAULT, android_dlopen_ext_name.as_ptr());
            if android_dlopen_ext_ptr.is_null() {
                return Err("Could not find android_dlopen_ext function".to_string());
            }

            let android_get_exported_namespace_ptr =
                dlsym(RTLD_DEFAULT, android_get_exported_namespace_name.as_ptr());
            if android_get_exported_namespace_ptr.is_null() {
                return Err("Could not find android_get_exported_namespace function".to_string());
            }

            let android_dlopen_ext: AndroidDlopenExtFn = mem::transmute(android_dlopen_ext_ptr);
            let android_get_exported_namespace: AndroidGetExportedNamespaceFn =
                mem::transmute(android_get_exported_namespace_ptr);

            // Rest of the code as before
            let c_lib_path = match CString::new(library_path) {
                Ok(s) => s,
                Err(_) => return Err("Invalid library path".to_string()),
            };

            let app_namespace_name = CString::new("app").unwrap();
            let app_namespace = android_get_exported_namespace(app_namespace_name.as_ptr());

            if app_namespace.is_null() {
                return Err("Could not find app namespace".to_string());
            }

            let mut dlextinfo: AndroidDlextinfo = mem::zeroed();
            dlextinfo.flags = ANDROID_DLEXT_USE_NAMESPACE;
            dlextinfo.library_namespace = app_namespace;

            let handle = android_dlopen_ext(c_lib_path.as_ptr(), RTLD_NOW, &dlextinfo);

            if handle.is_null() {
                let error = dlerror();
                if error.is_null() {
                    Err("Unknown error loading library".to_string())
                } else {
                    let error_str = std::ffi::CStr::from_ptr(error)
                        .to_string_lossy()
                        .to_string();
                    Err(format!("Error loading library: {}", error_str))
                }
            } else {
                Ok(handle)
            }
        }
    }

    let lib = load_in_app_namespace(path.display().to_string().as_str()).unwrap();

    lib
}

/// Get the offset of the current executable in the address space of the current process.
///
/// Forgets the library to prevent its drop from being calleds
fn alsr_offset(
    base_address: usize,
    #[cfg(unix)] lib: libloading::os::unix::Library,
    #[cfg(windows)] lib: libloading::os::windows::Library,
) -> Option<*mut c_void> {
    #[allow(unused_assignments)]
    let mut offset = None;

    // the only "known global symbol" for everything we compile is __rust_alloc
    // however some languages won't have this. we could consider linking in a known symbol but this works for now
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    unsafe {
        offset = lib
            .get::<*const ()>(b"__rust_alloc")
            .ok()
            .map(|ptr| ptr.as_raw_ptr());
    };

    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
    unsafe {
        // used to be __executable_start by that doesn't work for shared libraries
        offset = lib
            .get::<*const ()>(b"__rust_alloc")
            .ok()
            .map(|ptr| ptr.as_raw_ptr());
    };

    // Leak the library to prevent its drop from being called and unloading the library
    let _handle = lib.into_raw() as *mut c_void;

    // windows needs the raw handle directly to lookup the base address
    #[cfg(windows)]
    unsafe {
        offset = windows::get_module_base_address(_handle);
    }

    offset.map(|offset| offset.wrapping_byte_sub(base_address))
}

#[cfg(target_arch = "wasm32")]
fn relocate_wasm_jump_table(jump_table: JumpTable) -> JumpTable {
    todo!()
}
