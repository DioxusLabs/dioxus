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
