#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_desktop::*;
use dioxus_lib::prelude::*;
use std::sync::Mutex;

pub mod launch_bindings {
    use std::any::Any;

    use super::*;
    pub fn launch(
        root: fn() -> Element,
        _contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
        _platform_config: Vec<Box<dyn Any>>,
    ) {
        super::launch(root);
    }

    pub fn launch_virtual_dom(_virtual_dom: VirtualDom, _desktop_config: Config) -> ! {
        todo!()
    }
}

/// Launch via the binding API
pub fn launch(incoming: fn() -> Element) {
    #[cfg(target_os = "android")]
    {
        *APP_FN_PTR.lock().unwrap() = Some(incoming);
    }

    #[cfg(not(target_os = "android"))]
    {
        dioxus_desktop::launch::launch(incoming, vec![], Default::default());
    }
}

static APP_FN_PTR: Mutex<Option<fn() -> Element>> = Mutex::new(None);

pub fn root() {
    let app = APP_FN_PTR
        .lock()
        .expect("APP_FN_PTR lock failed")
        .expect("Android to have set the app trampoline");

    dioxus_desktop::launch::launch(app, vec![], Default::default());
}

#[cfg(target_os = "android")]
#[no_mangle]
#[inline(never)]
pub extern "C" fn start_app() {
    tao::android_binding!(
        com_example,
        androidfinal,
        WryActivity,
        wry::android_setup,
        root,
        tao
    );
    wry::android_binding!(com_example, androidfinal, wry);
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
