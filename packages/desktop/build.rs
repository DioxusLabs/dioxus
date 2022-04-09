#[cfg(any(target_os = "windows", target_env = "gnu"))]
use std::env;
#[cfg(any(target_os = "windows", target_env = "gnu"))]
use std::path::PathBuf;

#[cfg(any(target_os = "windows", target_env = "gnu"))]
fn link_windows_gnu() {
    let mut exe_pth = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    let mut webview2_loadr_dll_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    webview2_loadr_dll_path.push("dll");
    let wv_arch = if target.contains("x86_64") {
        "x64"
    } else if target.contains("i686") {
        "x86"
    } else {
        "arm64"
    };
    webview2_loadr_dll_path.push(wv_arch);
    let webview2_loadr_dll = webview2_loadr_dll_path.as_path().to_str().unwrap();
    println!("cargo:rustc-link-search={}", webview2_loadr_dll);
	println!("cargo:rustc-link-lib=WebView2Loader");
    if !target.contains("aarch64") {
        let webview2_loadr_dll_name = "WebView2Loader.dll";
        webview2_loadr_dll_path.push(webview2_loadr_dll_name);
        exe_pth.push("../../..");
        let mut for_examples_exe_pth = exe_pth.clone();
        for_examples_exe_pth.push("examples");
        exe_pth.push(webview2_loadr_dll_name);
        std::fs::copy(&webview2_loadr_dll_path, exe_pth.as_path())
            .expect("Can't copy from DLL dir /target/..");

        // Copy .dll to examples folder too, in order to run examples when cross compiling from linux.
        for_examples_exe_pth.push(webview2_loadr_dll_name);
        std::fs::copy(&webview2_loadr_dll_path, for_examples_exe_pth.as_path())
            .expect("Can't copy from DLL dir to /target/../examples");
    } else {
        panic!("{:?} not supported yet", target)
    }
}

fn main() {
    #[cfg(any(target_os = "windows", target_env = "gnu"))]
    link_windows_gnu();
}
