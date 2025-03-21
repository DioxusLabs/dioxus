use std::{
    any::TypeId,
    collections::HashMap,
    ffi::CStr,
    os::raw::c_void,
    panic::{panic_any, AssertUnwindSafe, UnwindSafe},
    path::PathBuf,
    sync::Arc,
};

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
