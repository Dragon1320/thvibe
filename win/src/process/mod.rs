use std::{ffi, mem, os::windows::prelude::OsStringExt, ptr};

use winapi::{
  shared::minwindef::MAX_PATH,
  um::{handleapi::CloseHandle, processthreadsapi::OpenProcess, psapi::GetModuleBaseNameW},
};

use crate::{
  error::{util::get_last_error, WinApiCodeResult, WinApiError, WinApiResult},
  wintype::{Pid, ProcessHandle, Wchar},
};

pub mod util;

#[derive(Debug)]
pub struct WinApiProcess {
  handle: ProcessHandle,
}

impl WinApiProcess {
  pub fn new(pid: Pid, access: u32) -> WinApiCodeResult<Self> {
    let handle = unsafe { OpenProcess(access, 0, pid) };

    if handle.is_null() {
      let err = get_last_error();

      return Err(err);
    }

    Ok(Self { handle })
  }

  pub fn get_name(&self) -> WinApiResult<String> {
    let mut buffer = Vec::with_capacity(MAX_PATH);
    let buffer_chars = buffer.capacity() / mem::size_of::<Wchar>();

    let ret_chars = unsafe {
      // safety
      // - cannot overflow u32, buffer capped at MAX_PATH
      GetModuleBaseNameW(self.handle, ptr::null_mut(), buffer.as_mut_ptr(), buffer_chars as u32)
    };

    if ret_chars == 0 {
      let err = get_last_error();

      return Err(WinApiError::WinApiErrorCode(err));
    }

    unsafe {
      // safety
      // - cannot be higher than buffer_chars
      buffer.set_len(ret_chars as usize);
    }

    let name = ffi::OsString::from_wide(&buffer).into_string();

    match name {
      Ok(name) => Ok(name),
      Err(os_string) => Err(WinApiError::StringParseError(os_string)),
    }
  }
}

impl Drop for WinApiProcess {
  // if bad stuff happens, print error and (probably) leak memory
  fn drop(&mut self) {
    let res = unsafe { CloseHandle(self.handle) };

    if res == 0 {
      let err = get_last_error();

      eprintln!("{:?}", err);
    }
  }
}
