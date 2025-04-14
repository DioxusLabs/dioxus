//! # Subsecond: Hot-patching for Rust
//!
//! Subsecond is a library that enables hot-patching for Rust applications. This allows you to change
//! the code of a running application without restarting it. This is useful for game engines, servers,
//! and other long-running applications where the typical edit-compile-run cycle is too slow.
//!
//! Subsecond also implements a technique we call "ThinLinking" which makes compiling Rust code
//! significantly faster in development mode, which can be used outside of hot-patching.
//!
//! # Usage
//!
//! Subsecond is designed to be as simple for both application developers and library authors.
//!
//! Simply call your existing functions with [`call`] and Subsecond will automatically detour
//! that call to the latest version of the function.
//!
//! ```rust
//! fn main() {
//!     for x in 0..5 {
//!         subsecond::call(|| {
//!             println!("Hello, world! {}", x);
//!         });
//!     }
//! }
//! ```
//!
//! To actually load patches into your application, a third-party tool that implements the Subsecond
//! compiler and protocol is required. Subsecond is built and maintained by the Dioxus team, so we
//! suggest using the dioxus CLI tool to use subsecond.
//!
//! To install the Dioxus CLI, we recommend using [`cargo binstall`](https://crates.io/crates/cargo-binstall):
//!
//! ```sh
//! cargo binstall dioxus-cli
//! ```
//!
//! The Dioxus CLI provides several tools for development. To run your application with Subsecond enabled,
//! use `dx serve` - this takes the same arguments as `cargo run` but will automatically hot-reload your
//! application when changes are detected.
//!
//! ```sh
//! dx serve
//! ```
//!
//! ## How it works
//!
//! Subsecond works by detouring function calls through a jump table. This jump table contains the latest
//! version of the program's function pointers, and when a function is called, Subsecond will look up
//! the function in the jump table and call that instead.
//!
//! Unlike libraries like [detour](https://crates.io/crates/detour), Subsecond *does not* modify your
//! process memory. Patching pointers is wildly unsafe and can lead to crashes and undefined behavior.
//!
//! Instead, an external tool compiles just the parts of your project that changed, links them together
//! using the addresses of the functions in your running program, and then sends the new jump table to
//! your application. Subsecond then applies the patch and continues running. Since Subsecond doesn't
//! modify memory, the program must have some runtime integration to handle the patching.
//!
//! If the framework you're using doesn't integrate with subsecond, you can rely on the fact that calls
//! to stale [`call`] instances will emit a safe panic that is automatically caught and retried
//! by the next [`call`] instance up the callstack.
//!
//! Subsecond is only enabled when debug_assertions are enabled so you can safely ship your application
//! with Subsecond enabled without worrying about the performance overhead.
//!
//! ## Globals and statics
//!
//! Subsecond *does* support hot-reloading of globals, statics, and thread locals. However, there are several limitations:
//!
//! - You may add new globals at runtime, but their destructors will never be called.
//! - Globals are tracked across patches, but will renames are considered to be *new* globals.
//! - Changes to static initializers will not be observed.
//!
//! Subsecond purposefully handles statics this way since many libraries like Dioxus and Tokio rely
//! on persistent global runtimes.
//!
//! ## Struct layout and alignment
//!
//! Subsecond currently does not support hot-reloading of structs. This is because the generated code
//! assumes a particular layout and alignment of the struct. If layout or alignment change and new
//! functions are called referencing an old version of the struct, the program will crash.
//!
//! To mitigate this, framework authors can integrate with Subsecond to either dispose of the old struct
//! or to re-allocate the struct in a way that is compatible with the new layout. This is called "re-instancing."
//!
//! Because Subsecond performs a safe panic if a stale function is called, you should never witness
//! a crash due to a struct layout change. However, changing a struct's layout will likely cause a
//! re-instantiation of the struct and potentially a loss of state.
//!
//! We'd like to lift this limitation in the future by providing utilities to re-instantiate structs,
//! but for now it's up to the framework authors to handle this. For example, Dioxus apps simply throw
//! out the old state and rebuild it from scratch.
//!
//! ## Nesting Calls
//!
//! Subsecond calls are designed to be nested. This provides clean integration points to know exactly
//! where a hooked function is called.
//!
//! The highest level call is `fn main()` though by default this is not hooked since initialization code
//! tends to be side-effectual and modify global state. Instead, we recommend wrapping the hot-patch
//! points manually with [`call`].
//!
//! ```rust
//! fn main() {
//!     // Changes to the the `for` loop will cause an unwind to this call.
//!     subsecond::call(|| {
//!         for x in 0..5 {
//!             // Changes to the `println!` will be isolated to this call.
//!             subsecond::call(|| {
//!                 println!("Hello, world! {}", x);
//!             });
//!         }
//!    });
//! }
//! ```
//!
//! The goal here is to provide granular control over where patches are applied to limit loss of state
//! when new code is loaded.
//!
//! ## Applying patches
//!
//! When running under the Dioxus CLI, the `dx serve` command will automatically apply patches when
//! changes are detected. Patches are delievered over the [Dioxus Devtools](https://crates.io/crates/dioxus-devtools)
//! websocket protocol and received by corresponding websocket.
//!
//! If you're using Subsecond in your own application that doesn't have a runtime integration, you can
//! build an integration using the [`apply_patch`] function. This function takes a `JumpTable` which
//! the subsecond-cli-support crate can generate.
//!
//! To add support for the Dioxus Devtools protocol to your app, you can use the [dioxus-devtools](https://crates.io/crates/dioxus-devtools)
//! crate which provides a `connect` method that will automatically apply patches to your application.
//!
//! Unfortunately, one design quirk of Subsecond is that running apps need to communicate the address
//! of `main` to the patcher. This is due to a security technique called [ASLR](https://en.wikipedia.org/wiki/Address_space_layout_randomization)
//! which randomizes the address of functions in memory. See the subsecond-harness and subsecond-cli
//! for more details on how to implement the protocol.
//!
//! ## ThinLink
//!
//! ThinLink is a program linker for Rust that is designed to be used with Subsecond. It implements
//! the powerful patching system that Subsecond uses to hot-reload Rust applications.
//!
//! ThinLink is simply a wrapper around your existing linker but with extra features:
//!
//! - Automatic dynamic linking to dependencies
//! - Generation of Subsecond jump tables
//! - Diffing of object files for function invalidation
//!
//! Because ThinLink performs very to little actual linking, it drastically speeds up traditional Rust
//! development. With a development-optimized profile, ThinLink can shrink an incremental build to less than 500ms.
//!
//! ThinLink is automatically integrated into the Dioxus CLI though it's currently not available as
//! a standalone tool.
//!
//! ## Limitations
//!
//! Subsecond is a powerful tool but it has several limitations. We talk about them above, but here's
//! a quick summary:
//!
//! - Struct hot reloading requires instancing or unwinding
//! - Statics are tracked but not destructed
//!
//! ## Platform support
//!
//! Subsecond works across all major platforms:
//!
//! - Android (arm64-v8a, armeabi-v7a)
//! - iOS (arm64, x86_64)
//! - Linux (x86_64, aarch64)
//! - macOS (x86_64, arm64)
//! - Windows (x86_64, aarch64)
//! - WebAssembly (wasm32)
//!
//! If you have a new platform you'd like to see supported, please open an issue on the Subsecond repository.
//! We are keen to add support for new platforms like wasm64, riscv64, and more.
//!
//! ## Adding the Subsecond badge to your project
//!
//! If you're a framework author and want your users to know that your library supports Subsecond, you
//! can add the Subsecond badge to your README! Users will know that your library is hot-reloadable and
//! can be used with Subsecond.
//!
//! [![Subsecond](https://img.shields.io/badge/Subsecond-Enabled-orange)](https://crates.io/crates/subsecond)
//!
//! ```markdown
//! [![Subsecond](https://img.shields.io/badge/Subsecond-Enabled-orange)](https://crates.io/crates/subsecond)
//! ```
//!
//! ## License
//!
//! Subsecond and ThinLink are licensed under the MIT license. See the LICENSE file for more information.
//!
//! ## Supporting this work
//!
//! Subsecond is a project by the Dioxus team. If you'd like to support our work, please consider
//! [sponsoring us on GitHub](https://github.com/sponsors/DioxusLabs) or eventually deploying your
//! apps with Dioxus Deploy (currently under construction).

use std::{
    any::TypeId,
    backtrace,
    collections::HashMap,
    ffi::CStr,
    mem::transmute,
    ops::Deref,
    os::raw::c_void,
    panic::{panic_any, AssertUnwindSafe, UnwindSafe},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub use subsecond_macro::hot;
pub use subsecond_types::JumpTable;

// todo: if there's a reference held while we run our patch, this gets invalidated. should probably
// be a pointer to a jump table instead, behind a cell or something. I believe Atomic + relaxed is basically a no-op
static HOTRELOAD_HANDLERS: Mutex<Vec<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(Vec::new());
static mut APP_JUMP_TABLE: Option<JumpTable> = None;
static mut CHANGED: bool = false;
static mut SUBSECOND_ENABLED: bool = false;

/// Call a given function with hot-reloading enabled. If the function's code changes, `call` will use
/// the new version of the function. If code *above* the function changes, this will emit a panic
/// that forces an unwind to the next [`call`] instance.
///
/// # Example
///
///
/// # Without unwinding
///
///
/// # WebAssembly
///
/// WASM/rust does not support unwinding, so [`call`] will not track dependency graph changes.
/// If you are building a framework for use on WASM, you will need to use `Subsecond::HotFn` directly.
///
/// However, if you wrap your calling code in a future, you *can* simply drop the future which will
/// cause `drop` to execute and get something similar to unwinding. Not great if refcells are open.
pub fn call<O>(f: impl FnMut() -> O) -> O {
    let mut hotfn = HotFn::current(f);

    loop {
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| hotfn.call(())));

        // If the call succeeds just return the result, otherwise we try to handle the panic if its our own.
        let err = match res {
            Ok(res) => return res,
            Err(err) => err,
        };

        // If this is our panic then let's handle it, otherwise we just resume unwinding
        let Some(_hot_payload) = err.downcast_ref::<HotFnPanic>() else {
            std::panic::resume_unwind(err);
        };

        // If subsecond is in the look, issue a breakpoint so they can try and issue a hot-patch.
        unsafe {
            if SUBSECOND_ENABLED {
                #[cfg(any(unix, windows))]
                {
                    dbg_breakpoint::breakpoint_if_debugging();
                }
                continue;
            }
        }
    }
}

/// A panic issued by the [`call`] function if the caller would be stale if called. This causes
/// an unwind to the next [`call`] instance that can properly handle the panic and retry the call.
///
/// This technique allows Subsecond to provide hot-reloading of codebases that don't have a runtime integration.
#[derive(Debug)]
pub struct HotFnPanic {
    _backtrace: backtrace::Backtrace,
}

/// A hot-reloadable function.
///
/// To call this function, use the [`HotFn::call`] method. This will automatically use the latest
/// version of the function from the JumpTable.
pub struct HotFn<A, M, T>
where
    T: HotFunction<A, M>,
{
    inner: T,
    _marker: std::marker::PhantomData<(A, M)>,
}

impl<A, M, F: HotFunction<A, M>> HotFn<A, M, F> {
    /// Create a new [`HotFn`] instance with the current function.
    ///
    /// Whenever you call [`HotFn::call`], it will use the current function from the JumpTable.
    pub const fn current(f: F) -> HotFn<A, M, F> {
        HotFn {
            inner: f,
            _marker: std::marker::PhantomData,
        }
    }

    /// Call the function with the given arguments.
    ///
    /// This will attempt to
    pub fn call(&mut self, args: A) -> F::Return {
        self.try_call(args).unwrap()
    }

    /// Attempt to call the function with the given arguments.
    ///
    /// If this function is stale and can't be updated in place (ie, changes occurred above this call),
    /// then this function will emit an [`HotFnPanic`] which can be unwrapped and handled by next [`call`]
    /// instance.
    pub fn try_call(&mut self, args: A) -> Result<F::Return, HotFnPanic> {
        // If we need to unwind, then let's throw a panic
        // This will occur when the pending patch is "over our head" and needs to be applied to a
        // "resume point". We can eventually look into migrating the datastructures over but for now
        // the resume point will force the struct to be re-built.
        // panic_any()

        unsafe {
            // Try to handle known function pointers. This is *really really* unsafe, but due to how
            // rust trait objects work, it's impossible to make an arbitrary usize-sized type implement Fn()
            // since that would require a vtable pointer, pushing out the bounds of the pointer size.
            if size_of::<F>() == size_of::<fn() -> ()>() {
                return Ok(self.inner.call_as_ptr(args));
            }

            // Handle trait objects. This will occur for sizes other than usize. Normal rust functions
            // become ZST's and thus their <T as SomeFn>::call becomes a function pointer to the function.
            //
            // For non-zst (trait object) types, then there might be an issue. The real call function
            // will likely end up in the vtable and will never be hot-reloaded since signature takes self.
            if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                let known_fn_ptr = <F as HotFunction<A, M>>::call_it as *const () as u64;
                if let Some(ptr) = jump_table.map.get(&known_fn_ptr).cloned() {
                    // The type sig of the cast should match the call_it function
                    // Technically function pointers need to be aligned, but that alignment is 1 so we're good
                    let call_it = transmute::<*const (), fn(&F, A) -> F::Return>(ptr as _);
                    return Ok(call_it(&self.inner, args));
                }
            }

            Ok(self.inner.call_it(args))
        }
    }
}

pub fn register_handler(handler: Arc<dyn Fn() + Send + Sync + 'static>) {
    unsafe {
        HOTRELOAD_HANDLERS.lock().unwrap().push(handler);
    }
}

/// Apply the patch using a given jump table.
///
/// # Safety
///
/// This function is unsafe because it detours existing functions in memory. This is *wildly* unsafe,
/// especially if the JumpTable is malformed. Only run this if you know what you're doing.
///
/// If the pointers are incorrect, function type signatures will be incorrect and the program will crash,
/// sometimes in a way that requires a restart of your entire computer. Be careful.
///
/// # Warning
///
/// This function will load the library and thus allocates. In cannot be used when the program is
/// stopped (ie in a signal handler).
pub unsafe fn apply_patch(mut table: JumpTable) {
    // As a form of integrity checking, we use the ASLR reference from the incoming jump table to assert that it is intended to patch us
    // #[cfg(any(unix, windows))]
    // if table.aslr_reference != aslr_reference() as u64 {
    //     println!(
    //         "ASLR reference mismatch: {} != {}",
    //         table.aslr_reference,
    //         aslr_reference()
    //     );
    //     return;
    // }

    // On non-wasm platforms we can just use libloading and the known aslr offsets to load the library
    #[cfg(any(unix, windows))]
    {
        // on android we try to cirumvent permissions issues by copying the library to a memmap and then libloading that
        #[cfg(target_os = "android")]
        let lib = Box::leak(Box::new(android_memmap_dlopen(&table.lib)));

        #[cfg(not(target_os = "android"))]
        let lib = Box::leak(Box::new({
            match libloading::Library::new(&table.lib) {
                Ok(lib) => {
                    println!("Loaded library success!: {:?}", &table.lib);
                    lib
                }
                err => {
                    eprintln!("Failed to load library: {:?}", err);
                    return;
                }
            }
        }));

        // Use the `aslr_offset` symbol as a sentinel for the current executable. This is basically a
        // cross-platform version of `__mh_execute_header` on macOS that sets a reference point for the
        // jump table.
        let old_offset = aslr_reference() - table.aslr_reference as usize;

        // Use the `__rust_alloc` symbol as a sentinel for the loaded library. Might want to move away
        // from this at some point, or make it configurable
        let new_offset = unsafe {
            // Leak the libary. dlopen is basically a no-op on many platforms and if we even try to drop it,
            // some code might be called (ie drop) that results in really bad crashes (restart your computer...)
            //
            // todo - we should define a symbol instead of __rust_alloc since it's going to be removed
            //      see https://github.com/rust-lang/rust/issues/139265
            lib.get::<*const ()>(b"__rust_alloc")
                .ok()
                .unwrap()
                .try_as_raw_ptr()
                .unwrap()
                .wrapping_byte_sub(table.new_base_address as usize) as usize
        };

        // Modify the jump table to be relative to the base address of the loaded library
        table.map = table
            .map
            .iter()
            .map(|(k, v)| {
                (
                    (*k as usize + old_offset) as u64,
                    (*v as usize + new_offset) as u64,
                )
            })
            .collect();

        commit_patch(table);
    };

    // On wasm, we need to download the module, compile it, and then run it.
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        use js_sys::{
            ArrayBuffer, Object, Reflect,
            WebAssembly::{self, Memory, Table},
        };
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsValue;
        use wasm_bindgen::UnwrapThrowExt;
        use wasm_bindgen_futures::JsFuture;

        let funcs: Table = wasm_bindgen::function_table().unchecked_into();
        let memory: Memory = wasm_bindgen::memory().unchecked_into();
        let exports: Object = wasm_bindgen::exports().unchecked_into();
        let buffer: ArrayBuffer = memory.buffer().unchecked_into();

        let path = table.lib.to_str().unwrap();
        if !path.ends_with(".wasm") {
            return;
        }

        // Start the fetch of the module
        let response = web_sys::window().unwrap_throw().fetch_with_str(&path);

        // Wait for the fetch to complete - we need the wasm module size in bytes to reserve in the memory
        let response: web_sys::Response = JsFuture::from(response).await.unwrap().unchecked_into();
        let dl_bytes: ArrayBuffer = JsFuture::from(response.array_buffer().unwrap())
            .await
            .unwrap()
            .unchecked_into();

        // Expand the memory and table size to accommodate the new data and functions
        //
        // Normally we wouldn't be able to trust that we are allocating *enough* memory
        // for BSS segments, but ld emits them in the binary when using import-memory.
        //
        // Make sure we align the memory base to the page size
        const PAGE_SIZE: u32 = 64 * 1024;
        let page_count = (buffer.byte_length() as f64 / PAGE_SIZE as f64).ceil() as u32;
        let memory_base = (page_count + 1) * PAGE_SIZE;

        // We need to grow the memory to accommodate the new module
        memory.grow((dl_bytes.byte_length() as f64 / PAGE_SIZE as f64).ceil() as u32);

        // We grow the ifunc table to accommodate the new functions
        // In theory we could just put all the ifuncs in the jump map and use that for our count,
        // but there's no guarantee from the jump table that it references "itself"
        // We might need a sentinel value for each ifunc in the jump map to indicate that it is
        let table_base = funcs.grow(table.ifunc_count as u32).unwrap();

        // Adjust the jump table to be relative to the new base address
        for v in table.map.values_mut() {
            *v += table_base as u64;
        }

        // Build up the import object
        //
        // let imports = {
        //     env: {
        //         memory: base.memory,
        //         __tls_base: base.__tls_base,
        //         __stack_pointer: base.__stack_pointer,
        //         __indirect_function_table: base.__indirect_function_table,
        //         __memory_base: memory_base,
        //         __table_base: table_base,
        //        ..base_exports
        //     },
        // };
        let env = Object::new();

        // Move memory, __tls_base, __stack_pointer, __indirect_function_table, and all exports over
        for key in Object::keys(&exports) {
            Reflect::set(&env, &key, &Reflect::get(&exports, &key).unwrap()).unwrap();
        }

        // Set the memory and table in the imports
        // Following this patern: Global.new({ value: "i32", mutable: false }, value)
        for (name, value) in [("__table_base", table_base), ("__memory_base", memory_base)] {
            let descriptor = Object::new();
            Reflect::set(&descriptor, &"value".into(), &"i32".into()).unwrap();
            Reflect::set(&descriptor, &"mutable".into(), &false.into()).unwrap();
            let value = WebAssembly::Global::new(&descriptor, &value.into()).unwrap();
            Reflect::set(&env, &name.into(), &value.into()).unwrap();
        }

        // Set the memory and table in the imports
        let imports = Object::new();
        Reflect::set(&imports, &"env".into(), &env).unwrap();

        // Download the module, returning { module, instance }
        let result_object = JsFuture::from(WebAssembly::instantiate_module(
            dl_bytes.unchecked_ref(),
            &imports,
        ))
        .await
        .unwrap();

        // We need to run the data relocations
        let res: Object = result_object.unchecked_into();
        let instance: Object = Reflect::get(&res, &"instance".into())
            .unwrap()
            .unchecked_into();
        let exports: Object = Reflect::get(&instance, &"exports".into())
            .unwrap()
            .unchecked_into();
        let func = Reflect::get(&exports, &"__wasm_apply_data_relocs".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>();
        func.call0(&JsValue::undefined()).unwrap();

        unsafe { commit_patch(table) };
    });
}

unsafe fn commit_patch(table: JumpTable) {
    // Update runtime state
    APP_JUMP_TABLE = Some(table);
    CHANGED = true;
    HOTRELOAD_HANDLERS
        .lock()
        .unwrap()
        .clone()
        .iter()
        .for_each(|handler| {
            handler();
        });
}

#[inline(never)]
#[no_mangle]
pub extern "C" fn aslr_reference() -> usize {
    aslr_reference as *const () as usize
}

/// On Android, we can't dlopen libraries that aren't placed inside /data/data/<package_name>/lib/
///
/// If the device isn't rooted, then we can't push the library there.
/// This is a workaround to copy the library to a memfd and then dlopen it.
///
/// I haven't tested it on device yet, so if if it doesn't work, then we can simply revert to using
/// "adb root" and then pushing the library to the /data/data folder instead of the tmp folder.
#[cfg(target_os = "android")]
unsafe fn android_memmap_dlopen(file: &Path) -> libloading::Library {
    use std::ffi::{c_void, CStr, CString};
    use std::os::fd::{AsRawFd, BorrowedFd};
    use std::ptr;

    #[repr(C)]
    struct ExtInfo {
        flags: u64,
        reserved_addr: *const c_void,
        reserved_size: libc::size_t,
        relro_fd: libc::c_int,
        library_fd: libc::c_int,
        library_fd_offset: libc::off64_t,
        library_namespace: *const c_void,
    }

    extern "C" {
        fn android_dlopen_ext(
            filename: *const libc::c_char,
            flags: libc::c_int,
            ext_info: *const ExtInfo,
        ) -> *const c_void;
    }

    use memmap2::MmapAsRawDesc;
    use std::os::unix::prelude::{FromRawFd, IntoRawFd};

    let contents = std::fs::read(file).unwrap();
    let mut mfd = memfd::MemfdOptions::default()
        .create("subsecond-patch")
        .unwrap();
    mfd.as_file().set_len(contents.len() as u64).unwrap();

    let raw_fd = mfd.into_raw_fd();

    let mut map = memmap2::MmapMut::map_mut(raw_fd).unwrap();
    map.copy_from_slice(&contents);
    let map = map.make_exec().unwrap();

    let filename = c"/subsecond-patch";
    let info = ExtInfo {
        flags: 0x10, // ANDROID_DLEXT_USE_LIBRARY_FD
        reserved_addr: ptr::null(),
        reserved_size: 0,
        relro_fd: 0,
        library_fd: raw_fd,
        library_fd_offset: 0,
        library_namespace: ptr::null(),
    };

    let flags = libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_LOCAL;

    let handle = libloading::os::unix::with_dlerror(
        || {
            let ptr = android_dlopen_ext(filename.as_ptr() as _, flags, &info);
            if ptr.is_null() {
                return None;
            } else {
                return Some(ptr);
            }
        },
        |err| err.to_str().unwrap().to_string(),
    )
    .unwrap();

    let lib = unsafe { libloading::os::unix::Library::from_raw(handle as *mut c_void) };
    let lib: libloading::Library = lib.into();
    lib
}

/// Apply the patch using a given jump table.
///
/// Used on WASM platforms where we need async integration to fetch the patch.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async unsafe fn __subsecond_wasm_patch(table: wasm_bindgen::JsValue) {
    use js_sys::{Object, Reflect};
    use subsecond_types::AddressMap;
    use wasm_bindgen::prelude::*;

    let as_obj: js_sys::Object = table.unchecked_into();
    let ifunc_count = Reflect::get(&as_obj, &"ifunc_count".into())
        .unwrap()
        .as_f64()
        .unwrap() as u64;
    let lib_url = Reflect::get(&as_obj, &"lib".into())
        .unwrap()
        .as_string()
        .unwrap();
    let map_obj = Reflect::get(&as_obj, &"map".into())
        .unwrap()
        .unchecked_into::<js_sys::Object>();

    let mut map = AddressMap::default();
    for entry in Object::entries(&map_obj).iter() {
        let entry = entry.unchecked_into::<js_sys::Array>();
        let key = entry.get(0);
        let value = entry.get(1);
        let key = key.as_string();
        let value = value.as_f64();
        if let Some(value) = value {
            if let Some(key) = key {
                if let Ok(key) = key.parse::<u64>() {
                    map.insert(key, value as u64);
                }
            }
        }
    }

    let table: JumpTable = JumpTable {
        map,
        ifunc_count,
        lib: lib_url.into(),
        aslr_reference: 0,
        new_base_address: 0,
    };

    apply_patch(table);
}

/// A trait that enables types to be hot-patched.
///
/// This trait is only implemented for FnMut types which naturally includes function pointers and
/// closures that can be re-ran. FnOnce closures are currently not supported since the hot-patching
/// system we use implies that the function can be called multiple times.
pub trait HotFunction<Args, Marker> {
    /// The return type of the function.
    type Return;

    /// The real function type. This is meant to be a function pointer.
    /// When we call `call_as_ptr`, we will transmute the function to this type and call it.
    type Real;

    /// Call the HotFunction with the given arguments.
    ///
    ///
    /// # Why
    ///
    /// "rust-call" isn't stable, so we wrap the underyling call with our own, giving it a stable vtable entry.
    /// This is more important than it seems since this function becomes "real" and can be hot-patched.
    fn call_it(&mut self, args: Args) -> Self::Return;

    /// Call the HotFunction as if it were a function pointer.
    ///
    /// # Safety
    ///
    /// This is only safe if the underyling type is a function (function pointer or virtual/fat pointer).
    /// Using this will use the JumpTable to find the patched function and call it.
    unsafe fn call_as_ptr(&mut self, _args: Args) -> Self::Return;
}

macro_rules! impl_hot_function {
    (
        $(
            ($marker:ident, $($arg:ident),*)
        ),*
    ) => {
        $(
            /// A marker type for the function.
            /// This is hidden with the intention to seal this trait.
            #[doc(hidden)]
            pub struct $marker;

            impl<T, $($arg,)* R> HotFunction<($($arg,)*), $marker> for T
            where
                T: FnMut($($arg),*) -> R,
            {
                type Return = R;
                type Real = fn($($arg),*) -> R;

                fn call_it(&mut self, args: ($($arg,)*)) -> Self::Return {
                    #[allow(non_snake_case)]
                    let ( $($arg,)* ) = args;
                    self($($arg),*)
                }

                unsafe fn call_as_ptr(&mut self, args: ($($arg,)*)) -> Self::Return {
                    unsafe {
                        if let Some(jump_table) = APP_JUMP_TABLE.as_ref() {
                            let real = std::mem::transmute_copy::<Self, Self::Real>(&self) as *const ();

                            // Android implements MTE / pointer tagging and we need to preserve the tag.
                            // If we leave the tag, then indexing our jump table will fail and patching won't work (or crash!)
                            // This is only implemented on 64-bit platforms since pointer tagging is not available on 32-bit platforms
                            // In dev, Dioxus disables MTE to work around this issue, but we still handle it anyways.
                            #[cfg(target_pointer_width = "64")] let nibble  = real as u64 & 0xFF00_0000_0000_0000;
                            #[cfg(target_pointer_width = "64")] let real    = real as u64 & 0x00FFF_FFF_FFFF_FFFF;

                            #[cfg(target_pointer_width = "64")] let real  = real as u64;

                            // No nibble on 32-bit platforms, but we still need to assume u64 since the host always writes 64-bit pointers
                            #[cfg(target_pointer_width = "32")] let real = real as u64;

                            if let Some(ptr) = jump_table.map.get(&real).cloned() {
                                // Re-apply the nibble - though this might not be required (we aren't calling malloc for a new pointer)
                                // #[cfg(target_pointer_width = "64")] let ptr: u64 = ptr | nibble;

                                #[cfg(target_pointer_width = "64")] let ptr: u64 = ptr;
                                #[cfg(target_pointer_width = "32")] let ptr: u32 = ptr as u32;

                                // Macro-rules requires unpacking the tuple before we call it
                                #[allow(non_snake_case)]
                                let ( $($arg,)* ) = args;
                                return std::mem::transmute::<_, Self::Real>(ptr)($($arg),*);
                            }
                        }

                        self.call_it(args)
                    }
                }
            }
        )*
    };
}

impl_hot_function!(
    (Fn0Marker,),
    (Fn1Marker, A),
    (Fn2Marker, A, B),
    (Fn3Marker, A, B, C),
    (Fn4Marker, A, B, C, D),
    (Fn5Marker, A, B, C, D, E),
    (Fn6Marker, A, B, C, D, E, F),
    (Fn7Marker, A, B, C, D, E, F, G),
    (Fn8Marker, A, B, C, D, E, F, G, H),
    (Fn9Marker, A, B, C, D, E, F, G, H, I)
);
