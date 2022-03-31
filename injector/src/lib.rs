use std::ffi::CString;
use std::fs;
use std::mem;
use std::ptr;
use std::str;

use winapi::shared::minwindef::FALSE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::libloaderapi::GetProcAddress;
use winapi::um::memoryapi::VirtualAllocEx;
use winapi::um::memoryapi::VirtualFreeEx;
use winapi::um::memoryapi::WriteProcessMemory;
use winapi::um::processthreadsapi::CreateRemoteThread;
use winapi::um::processthreadsapi::GetExitCodeThread;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;
use winapi::um::winnt::MEM_RELEASE;
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS};

use widestring::WideCString;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const KERNEL_MODULE_NAME: &str = "Kernel32";
const LOADLIB_PROC_NAME: &str = "LoadLibraryA";

pub fn inject_dll(file_name: &str, pid: u32) -> Result<()> {
  // get full path name
  // ...and convert it into a cstring
  let path = fs::canonicalize(file_name)?;
  let path_cstr = WideCString::from_os_str(path.as_os_str())?;

  // u16 str on win
  // char_len * 2 + nul terminator
  let byte_len = path_cstr.len() * 2 + 1;

  // get handle to process
  let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, FALSE, pid) };

  // allocate memory for dll path string
  let dll_addr = unsafe {
    VirtualAllocEx(
      handle,
      ptr::null_mut(),
      byte_len,
      MEM_COMMIT | MEM_RESERVE,
      PAGE_READWRITE,
    )
  };

  unsafe {
    WriteProcessMemory(
      handle,
      dll_addr,
      path_cstr.as_ptr() as *mut _,
      byte_len,
      ptr::null_mut(),
    )
  };

  let loadlib_addr = unsafe {
    let kernel_cstr = CString::new(KERNEL_MODULE_NAME)?;
    let kernel = GetModuleHandleA(kernel_cstr.as_ptr());

    let loadlib_cstr = CString::new(LOADLIB_PROC_NAME)?;
    GetProcAddress(kernel, loadlib_cstr.as_ptr())
  };

  let thread = unsafe {
    CreateRemoteThread(
      handle,
      ptr::null_mut(),
      0,
      Some(mem::transmute(loadlib_addr)),
      dll_addr,
      0,
      ptr::null_mut(),
    )
  };

  unsafe { WaitForSingleObject(thread, INFINITE) };

  let mut exit_code = 0;

  unsafe {
    GetExitCodeThread(thread, &mut exit_code);
  };

  unsafe {
    CloseHandle(thread);
    VirtualFreeEx(handle, dll_addr, 0, MEM_RELEASE);
    CloseHandle(handle);
  };

  Ok(())
}
