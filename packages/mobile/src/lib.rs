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

static APP_OBJECTS: Mutex<Option<BoundLaunchObjects>> = Mutex::new(None);

struct BoundLaunchObjects {
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
}

unsafe impl Send for BoundLaunchObjects {}
unsafe impl Sync for BoundLaunchObjects {}

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

        let main_fn: extern "C" fn() = std::mem::transmute(main_fn_ptr);
        main_fn();
    };

    jni::sys::JNI_VERSION_1_6
}
