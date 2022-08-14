use std::{error, ffi, fs, mem, os::windows::prelude::OsStrExt, ptr};

use win::{
  error::util::get_last_error,
  wintype::{Pid, Wchar},
};
use winapi::um::{
  handleapi::CloseHandle,
  libloaderapi::{GetModuleHandleA, GetProcAddress},
  memoryapi::{VirtualAllocEx, VirtualFreeEx, WriteProcessMemory},
  processthreadsapi::{CreateRemoteThread, GetExitCodeThread, OpenProcess},
  synchapi::WaitForSingleObject,
  winbase::INFINITE,
  winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS},
};

const KERNEL_MODULE_NAME: &str = "Kernel32";
const LOADLIB_PROC_NAME: &str = "LoadLibraryW";

type BoxResult<T> = Result<T, Box<dyn error::Error + Send + Sync + 'static>>;

pub fn load_module(module_path: &str, pid: u32) -> BoxResult<()> {
  let path = fs::canonicalize(module_path)?;

  let buffer = path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
  let buffer_bytelen = buffer.len() * mem::size_of::<u16>();

  let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };

  if handle.is_null() {
    panic!();
  }

  let proc_mem = unsafe {
    VirtualAllocEx(
      handle,
      ptr::null_mut(),
      buffer_bytelen,
      MEM_COMMIT | MEM_RESERVE,
      PAGE_READWRITE,
    )
  };

  if proc_mem.is_null() {
    panic!();
  }

  let success_code = unsafe {
    WriteProcessMemory(
      handle,
      proc_mem,
      buffer.as_ptr() as *const _,
      buffer_bytelen,
      ptr::null_mut(),
    )
  };

  if success_code == 0 {
    panic!();
  }

  // TODO
  let loadlib_addr = get_loadlib_addr();

  let thread = unsafe {
    CreateRemoteThread(
      handle,
      ptr::null_mut(),
      0,
      // TODO
      Some(mem::transmute(loadlib_addr)),
      proc_mem,
      0,
      ptr::null_mut(),
    )
  };

  if thread.is_null() {
    panic!();
  }

  let exit_code = unsafe {
    let mut exit_code = 0;

    WaitForSingleObject(thread, INFINITE);
    GetExitCodeThread(thread, &mut exit_code);

    exit_code
  };

  if exit_code == 0 {
    let error = get_last_error();

    // panic!("{:?}", error);
  }

  unsafe {
    CloseHandle(thread);
    VirtualFreeEx(handle, proc_mem, 0, MEM_RESERVE);
    CloseHandle(handle);
  };

  Ok(())
}

// TODO: other result type
// TODO: attributes
// TODO: better types (enum) and checks for other params

// safety
// - handle must be a valid handle to an open object
// - addr and param need to be ok
unsafe fn create_remote_thread(
  handle: *mut ffi::c_void,
  attributes: (),
  stack_size: usize,
  addr: extern "system" fn(*mut ffi::c_void) -> u32,
  param: *mut ffi::c_void,
  creation_flags: u32,
) -> BoxResult<(*mut ffi::c_void, u32)> {
  let (thread, id) = {
    let mut id = 0;

    let thread = CreateRemoteThread(
      handle,
      ptr::null_mut(),
      stack_size,
      Some(addr),
      param,
      creation_flags,
      &mut id,
    );

    (thread, id)
  };

  if thread.is_null() {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok((thread, id))
  }
}

// TODO: other result type

// safety
// - handle must be a valid handle to an open object
unsafe fn close_handle(handle: *mut ffi::c_void) -> BoxResult<()> {
  let success_code = CloseHandle(handle);

  if success_code == 0 {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(())
  }
}

// TODO: other result type
// TODO: can be safer if alloc_type and protection are enums
// TODO: is byte_len == 0 valid?

// safety
// - handle must be a valid handle to an open object
unsafe fn virtual_alloc_ex(
  handle: *mut ffi::c_void,
  addr: *mut ffi::c_void,
  byte_len: usize,
  alloc_type: u32,
  protection: u32,
) -> BoxResult<*mut ffi::c_void> {
  let mem_addr = VirtualAllocEx(handle, addr, byte_len, alloc_type, protection);

  if mem_addr.is_null() {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(mem_addr)
  }
}

// TODO: other result type
// TODO: some checks can be done here based on size and free_type

// safety
// - handle must be a valid handle to an open object
// - must point to valid memory allocated by the process
// - size must follow rules based on free_type and earlier call to virtual_alloc_ex
unsafe fn virtual_free_ex(
  handle: *mut ffi::c_void,
  addr: *mut ffi::c_void,
  size: usize,
  free_type: u32,
) -> BoxResult<()> {
  let success_code = VirtualFreeEx(handle, addr, size, free_type);

  if success_code == 0 {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(())
  }
}

// TODO: other result type
// TODO: can have checks around buffer and byte len

// safety
// - handle must be a valid handle to an open object
// - must point to valid memory allocated by the process
// - byte_len must not exceed buffer size
unsafe fn write_process_memory<T>(
  handle: *mut ffi::c_void,
  addr: *mut ffi::c_void,
  buffer: &[T],
  byte_len: usize,
) -> BoxResult<()> {
  let success_code = WriteProcessMemory(handle, addr, buffer.as_ptr() as *const _, byte_len, ptr::null_mut());

  if success_code == 0 {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(())
  }
}

// TODO: other result type
// TODO: this is safe if we restrict possible values of access with an enum
unsafe fn open_process(access: u32, inherit: bool, pid: u32) -> BoxResult<*mut ffi::c_void> {
  // the only valid values for bool are 0 or 1
  let handle = OpenProcess(access, inherit as i32, pid);

  if handle.is_null() {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(handle)
  }
}

fn get_loadlib_addr() -> fn(*mut ffi::c_void) -> u32 {
  let kernel = unsafe {
    let kernel_cstr = ffi::CString::new(KERNEL_MODULE_NAME).unwrap();

    GetModuleHandleA(kernel_cstr.as_ptr())
  };

  if kernel.is_null() {
    panic!();
  }

  let loadlib = unsafe {
    let loadlib_cstr = ffi::CString::new(LOADLIB_PROC_NAME).unwrap();

    GetProcAddress(kernel, loadlib_cstr.as_ptr())
  };

  if loadlib.is_null() {
    panic!();
  }

  unsafe { mem::transmute(loadlib) }
}
