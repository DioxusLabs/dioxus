//! Spawn a process at a non aslr location
//!
//! We shouldn't need this in practice but it is generally quite useful outside of hotpatching.
//! This ensures the process is spawned at the location it was compiled at. The process can then
//! use addresses of its binary to call functions in its own address space.
//!
//! Typically the platforms *don't* want you to do this, but it makes our life easier.

use std::{ffi::CString, path::Path};

use anyhow::bail;

/// Spawn a process at memory address 0, disabling ASLR
pub fn spawn_aslr_posix(exe: &Path, args: Vec<String>) -> anyhow::Result<i32> {
    let mut pid = 0;

    // Convert exe to CString
    let program_c = std::ffi::CString::new(exe.as_os_str().to_str().unwrap()).unwrap();

    // Convert args to CStrings
    let mut args_vec: Vec<CString> = Vec::with_capacity(args.len() + 1);
    args_vec.push(program_c.clone());
    for arg in args {
        args_vec.push(CString::new(arg)?);
    }

    // Create null-terminated array of pointers to args
    let mut args_ptr: Vec<*const libc::c_char> = args_vec.iter().map(|arg| arg.as_ptr()).collect();
    args_ptr.push(std::ptr::null());

    // Load the environ pointer (our current environment)
    extern "C" {
        #[allow(unused)]
        static environ: *const *const libc::c_char;
    }

    unsafe {
        let mut attr: libc::posix_spawnattr_t = std::mem::zeroed();
        let ret = libc::posix_spawnattr_init(&mut attr);
        if ret != 0 {
            bail!("posix_spawnattr_init failed");
        }

        // Set the flag to disable ASLR - maybe also enable libc::POSIX_SPAWN_SETEXEC?
        const POSIX_SPAWN_DISABLE_ASLR: libc::c_int = 0x0100;
        let ret = libc::posix_spawnattr_setflags(&mut attr, (POSIX_SPAWN_DISABLE_ASLR) as _);
        if ret != 0 {
            libc::posix_spawnattr_destroy(&mut attr);
            bail!("posix_spawnattr_setflags failed");
        }

        // Set the file actions to use the default actions
        let mut fileactions: libc::posix_spawn_file_actions_t = std::ptr::null_mut();
        let ret = libc::posix_spawn_file_actions_init(&mut fileactions);
        if ret != 0 {
            libc::posix_spawnattr_destroy(&mut attr);
            bail!("posix_spawn_file_actions_init failed");
        }

        libc::posix_spawn(
            &mut pid,
            program_c.as_ptr(),
            &fileactions,
            &attr,
            args_ptr.as_ptr() as *const *mut libc::c_char,
            environ as *const _,
        );
    };

    Ok(pid)
}
