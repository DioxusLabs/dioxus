#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::mem;

/// Gets the base address of a loaded library using GetModuleInformation
pub(crate) fn get_module_base_address(module_handle: *mut c_void) -> Option<*mut c_void> {
    // Manual Windows type definitions
    type BOOL = i32;
    type DWORD = u32;
    type HANDLE = *mut c_void;
    type HMODULE = *mut c_void;
    type LPCVOID = *const c_void;
    type LPVOID = *mut c_void;

    // MODULEINFO structure
    #[repr(C)]
    #[allow(non_snake_case)]
    struct MODULEINFO {
        lpBaseOfDll: LPVOID,
        SizeOfImage: DWORD,
        EntryPoint: LPVOID,
    }

    // Manual function imports
    extern "system" {
        fn GetCurrentProcess() -> HANDLE;
        fn GetModuleInformation(
            hProcess: HANDLE,
            hModule: HMODULE,
            lpmodinfo: *mut MODULEINFO,
            cb: DWORD,
        ) -> BOOL;
        fn GetLastError() -> DWORD;
    }

    unsafe {
        // Prepare to get module information
        let mut module_info: MODULEINFO = mem::zeroed();

        // Call GetModuleInformation to get details about this module
        let result = GetModuleInformation(
            GetCurrentProcess(),
            module_handle,
            &mut module_info,
            mem::size_of::<MODULEINFO>() as DWORD,
        );

        match result {
            0 => Some(module_info.lpBaseOfDll),
            _ => None,
        }
    }
}

// // We're going to use our reference function that we manually linked in
// let reference = unsafe { leak.get::<*mut u64>(b"dynamic_aslr_reference").unwrap() };
// let reference_addr = *reference as *mut u64;
// let reference_value = unsafe { *reference_addr } as usize;

// println!("reference: {reference_addr:#x?}");
// println!("reference value: {reference_value:#x?}");

// // because the program is opened above address 0, the addr will always be higher than the value
// let out = reference_addr as usize - reference_value;
// println!("out: {out:#x?}");
// out

// #[allow(unused_assignments)]
// let mut offset = None;

// // the only "known global symbol" for everything we compile is __rust_alloc
// // however some languages won't have this. we could consider linking in a known symbol but this works for now
// #[cfg(any(target_os = "macos", target_os = "ios"))]
// unsafe {
//     offset = lib
//         .get::<*const ()>(b"__rust_alloc")
//         .ok()
//         .map(|ptr| ptr.as_raw_ptr());
// };

// #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
// unsafe {
//     offset = lib
//         .get::<*const ()>(b"__rust_alloc")
//         .ok()
//         .map(|ptr| ptr.as_raw_ptr());
// };

// // Leak the library to prevent its drop from being called and unloading the library
// let _handle = lib.into_raw() as *mut c_void;

// // windows needs the raw handle directly to lookup the base address
// #[cfg(windows)]
// unsafe {
//     offset = windows::get_module_base_address(_handle);
// }

// 03-21 02:20:20.332 25787 25811 I RustStdoutStderr: offset: Some(0x71ff1d87f8)
// 03-21 02:20:20.332 25787 25811 I RustStdoutStderr: base_address: 354296

// println!("offset: {offset:?}");
// println!("base_address: {base_address:?}");
// // offset.map(|offset| offset as usize - base_address)
// let offset = offset.unwrap();
// offset as usize - base_address

// // the only "known global symbol" for everything we compile is __rust_alloc
// // however some languages won't have this. we could consider linking in a known symbol but this works for now
// // #[cfg(any(target_os = "macos", target_os = "ios"))]
// unsafe {
//     offset = lib
//         .get::<*const ()>(b"__rust_alloc")
//         .ok()
//         .map(|ptr| ptr.as_raw_ptr());
// };

// println!("-aslr calc offset: {offset:?}");
// println!("-aslr calc base_address: {base_address:?}");

// // attempt to determine the aslr slide by using the on-disk rust-alloc symbol
// // offset.map(|offset| offset.wrapping_byte_sub(base_address as usize))
// offset.map(|offset| offset.wrapping_byte_sub(base_address))

// #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
// unsafe {
//     // used to be __executable_start by that doesn't work for shared libraries
//     offset = lib
//         .get::<*const ()>(b"__rust_alloc")
//         .ok()
//         .map(|ptr| ptr.as_raw_ptr());
// };

// Leak the library to prevent its drop from being called and unloading the library
// let _handle = lib.into_raw() as *mut c_void;

// // windows needs the raw handle directly to lookup the base address
// #[cfg(windows)]
// unsafe {
//     offset = windows::get_module_base_address(_handle);
// }

// let offset = offset.unwrap() as usize;
// // strip the tag
// //
// let offset = offset & 0x00FFFFFFFFFFFFFF;
// // let offset = offset & 0x00FFFFFFFFFFFFFF;

// // println!("offset: {offset:?}");
// // println!("base_address: {base_address:?}");
// // println!("base_address: {base_address:x?}");

// let res = offset - base_address as usize;
// // let res = offset.map(|offset| offset.wrapping_byte_sub(base_address as usize));
// println!("res: {res:?}");
// Some(res as _)
