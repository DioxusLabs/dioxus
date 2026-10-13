/// Expose the `Java_dev_dioxus_main_Rust_*` functions to the JNI layer.
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
    use crate::Config;
    use dioxus_core::{Element, VirtualDom};
    use std::any::Any;

    // tao 0.37 puts the application in `ndk_context`, where tao 0.34 put the activity. Crates that
    // read it here take it for the activity (manganis' `with_activity`, for example, builds
    // activity-bound objects with it: dialogs, permission prompts, the back dispatcher), so the first
    // activity replaces the application there before wry's setup and before `main` runs. It's kept
    // for the life of the process, as tao 0.34 did.
    //
    // A second activity in the process is the first one re-created for a configuration change its
    // manifest doesn't list (a dark mode switch, a font scale change, ...), or a second instance
    // opened by an intent. tao 0.37 keeps the window, but wry makes no web view for the new activity
    // and the page stays blank. tao 0.34 aborted there (`ndk_context` was initialized twice), so this
    // ends the process too, cleanly: the user reopens the app in a new process.
    unsafe fn android_setup(
        package: &str,
        env: jni::JNIEnv<'_>,
        looper: &ndk::looper::ThreadLooper,
        activity: jni::objects::GlobalRef,
    ) {
        static FIRST_ACTIVITY: std::sync::OnceLock<jni::objects::GlobalRef> =
            std::sync::OnceLock::new();
        if FIRST_ACTIVITY.set(activity.clone()).is_ok() {
            if let (Ok(vm), Some(first)) = (env.get_java_vm(), FIRST_ACTIVITY.get()) {
                unsafe {
                    ndk_context::release_android_context();
                    ndk_context::initialize_android_context(
                        vm.get_java_vm_pointer().cast(),
                        first.as_obj().as_raw().cast(),
                    );
                }
            }
        } else {
            // `_exit`, not `exit`: `exit` on this (Java) thread runs the runtime's shutdown, which
            // aborts with the other threads still running.
            unsafe { libc::_exit(0) }
        }
        unsafe { wry::android_setup(package, env, looper, activity) }
    }

    // wry's Kotlin declares tao's lifecycle calls on its `Rust` object
    // (`Java_dev_dioxus_main_Rust_onCreate`, ...).
    tao::android_binding!(dev_dioxus, main, Rust, android_setup, root, tao);
    wry::android_binding!(dev_dioxus, main, wry);

    #[cfg(target_os = "android")]
    fn root() {
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
                // Load the env file from the session cache if we're in debug mode and on android
                //
                // This is a slightly hacky way of being able to use std::env::var code in android apps without
                // going through their custom java-based system.
                let env_file = dioxus_cli_config::android_session_cache_dir().join(".env");
                if let Ok(env_file) = std::fs::read_to_string(&env_file) {
                    for line in env_file.lines() {
                        if let Some((key, value)) = line.trim().split_once('=') {
                            std::env::set_var(key, value);
                        }
                    }
                }
            }

            let main_fn: extern "C" fn() = std::mem::transmute(main_fn_ptr);
            main_fn();
        });
    }
}
