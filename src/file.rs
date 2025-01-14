extern crate winapi;

use std::ffi::CString;
use std::ptr;
use winapi::um::handleapi::CloseHandle;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::libloaderapi::GetProcAddress;
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory};
use winapi::um::processthreadsapi::{CreateRemoteThread, OpenProcess};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS};

pub(crate) fn attach_to_process(pid: u32, dll_path: &str) -> Result<(), String> {
    let process_handle = unsafe {
        OpenProcess(PROCESS_ALL_ACCESS, 0, pid)
    };
    if process_handle.is_null() {
        return Err("Failed to open target process".to_string());
    }

    let alloc_mem = unsafe {
        VirtualAllocEx(process_handle, ptr::null_mut(), dll_path.len() + 1, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
    };
    if alloc_mem.is_null() {
        return Err("Failed to allocate memory in target process".to_string());
    }

    let c_dll_path = CString::new(dll_path).unwrap();
    let result = unsafe {
        WriteProcessMemory(process_handle, alloc_mem, c_dll_path.as_ptr() as *const _, dll_path.len() + 1, ptr::null_mut())
    };
    if result == 0 {
        return Err("Failed to write DLL path into target process".to_string());
    }

    let kernel32 = CString::new("kernel32.dll").unwrap();
    let load_lib = CString::new("LoadLibraryA").unwrap();

    let load_lib_addr = unsafe {
        GetProcAddress(GetModuleHandleA(kernel32.as_ptr()), load_lib.as_ptr())
    };
    if load_lib_addr.is_null() {
        return Err("Failed to get address of LoadLibraryA".to_string());
    }

    let thread_handle = unsafe {
        CreateRemoteThread(process_handle, ptr::null_mut(), 0, Some(std::mem::transmute(load_lib_addr)), alloc_mem, 0, ptr::null_mut())
    };
    if thread_handle.is_null() {
        return Err("Failed to create remote thread".to_string());
    }

    unsafe {
        CloseHandle(thread_handle);
        CloseHandle(process_handle);
    }

    Ok(())
}