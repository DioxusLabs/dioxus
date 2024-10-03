#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub use dioxus_desktop::*;

pub fn launch() {
    // #[cfg(any(target_os = "android", target_os = "ios"))]
    // fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
    //     match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
    //         Ok(t) => t,
    //         Err(err) => {
    //             eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
    //             std::process::abort()
    //         }
    //     }
    // }
    let main_fn = || {};

    #[cfg(target_os = "android")]
    {
        tao::android_binding!(
            com_example,
            wrytest,
            WryActivity,
            wry::android_setup,
            main_fn,
            tao
        );
        wry::android_binding!(com_example, wrytest, wry);
    }
}
