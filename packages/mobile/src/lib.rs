#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_desktop::*;
// intentionally shadow the launch module as it does not work on mobile.
pub mod launch {}

use dioxus_lib::prelude::*;
use std::any::Any;

pub mod launch_bindings {

    use super::*;
    pub fn launch(
        root: fn() -> Element,
        contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
        platform_config: Vec<Box<dyn Any>>,
    ) {
        super::launch_cfg(root, contexts, platform_config);
    }

    pub fn launch_virtual_dom(_virtual_dom: VirtualDom, _desktop_config: Config) -> ! {
        todo!()
    }
}

/// Launch via the binding API
pub fn launch(root: fn() -> Element) {
    launch_cfg(root, vec![], vec![]);
}

pub fn launch_cfg(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) {
    dioxus_desktop::launch::launch(root, contexts, platform_config);
}

#[doc(hidden)]
pub fn root() {
    fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(t) => t,
            Err(err) => {
                eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
                std::process::abort()
            }
        }
    }

    stop_unwind(|| unsafe {
        let mut main_fn_ptr = libc::dlsym(libc::RTLD_DEFAULT, b"main\0".as_ptr() as _);

        if main_fn_ptr.is_null() {
            main_fn_ptr = libc::dlsym(libc::RTLD_DEFAULT, b"_main\0".as_ptr() as _);
        }

        if main_fn_ptr.is_null() {
            panic!("Failed to find main symbol");
        }

        // Set the env vars that rust code might expect, passed off to us by the android app
        // Doing this before main emulates the behavior of a regular executable
        if cfg!(target_os = "android") && cfg!(debug_assertions) {
            load_env_file_from_session_cache();
        }

        let main_fn: extern "C" fn() = std::mem::transmute(main_fn_ptr);
        main_fn();
    });
}

/// Expose the `Java_dev_dioxus_main_WryActivity_create` function to the JNI layer.
/// We hardcode these to have a single trampoline for host Java code to call into.
///
/// This saves us from having to plumb the top-level package name all the way down into
/// this file. This is better for modularity (ie just call dioxus' main to run the app) as
/// well as cache thrashing since this crate doesn't rely on external env vars.
///
/// The CLI is expecting to find `dev.dioxus.main` in the final library. If you find a need to
/// change this, you'll need to change the CLI as well.
#[cfg(target_os = "android")]
#[no_mangle]
#[inline(never)]
pub extern "C" fn start_app() {
    tao::android_binding!(dev_dioxus, main, WryActivity, wry::android_setup, root, tao);
    wry::android_binding!(dev_dioxus, main, wry);
}

/// Load the env file from the session cache if we're in debug mode and on android
///
/// This is a slightly hacky way of being able to use std::env::var code in android apps without
/// going through their custom java-based system.
fn load_env_file_from_session_cache() {
    let env_file = dioxus_cli_config::android_session_cache_dir().join(".env");
    if let Ok(env_file) = std::fs::read_to_string(&env_file) {
        for line in env_file.lines() {
            if let Some((key, value)) = line.trim().split_once('=') {
                std::env::set_var(key, value);
            }
        }
    }
}
