#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_desktop::*;
use dioxus_lib::prelude::*;
use std::any::Any;
use std::sync::Mutex;

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
    #[cfg(target_os = "android")]
    {
        *APP_OBJECTS.lock().unwrap() = Some(BoundLaunchObjects {
            root,
            contexts,
            platform_config,
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        dioxus_desktop::launch::launch(root, contexts, platform_config);
    }
}

/// We need to store the root function and contexts in a static so that when the tao bindings call
/// "start_app", that the original function arguments are still around.
///
/// If you look closely, you'll notice that we impl Send for this struct. This would normally be
/// unsound. However, we know that the thread that created these objects ("main()" - see JNI_OnLoad)
/// is finished once `start_app` is called. This is similar to how an Rc<T> is technically safe
/// to move between threads if you can prove that no other thread is using the Rc<T> at the same time.
/// Crates like https://crates.io/crates/sendable exist that build on this idea but with runtimk,
/// validation that the current thread is the one that created the object.
///
/// Since `main()` completes, the only reader of this data will be `start_app`, so it's okay to
/// impl this as Send/Sync.
///
/// Todo(jon): the visibility of functions in this module is too public. Make sure to hide them before
/// releasing 0.7.
struct BoundLaunchObjects {
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
}

unsafe impl Send for BoundLaunchObjects {}
unsafe impl Sync for BoundLaunchObjects {}

static APP_OBJECTS: Mutex<Option<BoundLaunchObjects>> = Mutex::new(None);

#[doc(hidden)]
pub fn root() {
    let app = APP_OBJECTS
        .lock()
        .expect("APP_FN_PTR lock failed")
        .take()
        .expect("Android to have set the app trampoline");

    let BoundLaunchObjects {
        root,
        contexts,
        platform_config,
    } = app;

    dioxus_desktop::launch::launch(root, contexts, platform_config);
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

/// Call our `main` function to initialize the rust runtime and set the launch binding trampoline
#[cfg(target_os = "android")]
#[no_mangle]
#[inline(never)]
pub extern "C" fn JNI_OnLoad(
    _vm: *mut libc::c_void,
    _reserved: *mut libc::c_void,
) -> jni::sys::jint {
    // we're going to find the `main` symbol using dlsym directly and call it
    unsafe {
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
    };

    jni::sys::JNI_VERSION_1_6
}

/// Load the env file from the session cache if we're in debug mode and on android
///
/// This is a slightly hacky way of being able to use std::env::var code in android apps without
/// going through their custom java-based system.
#[cfg(target_os = "android")]
fn load_env_file_from_session_cache() {
    let env_file = dioxus_cli_config::android_session_cache_dir().join(".env");
    if let Some(env_file) = std::fs::read_to_string(&env_file).ok() {
        for line in env_file.lines() {
            if let Some((key, value)) = line.trim().split_once('=') {
                std::env::set_var(key, value);
            }
        }
    }
}
