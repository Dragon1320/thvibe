use std::mem;

use winapi::um::psapi::EnumProcesses;

use crate::{
  error::{util::get_last_error, WinApiError, WinApiResult},
  wintype::Pid,
};

const INITIAL_BUFFER_SIZE: usize = 1024;

pub fn enum_processes() -> WinApiResult<Vec<Pid>> {
  let mut buffer_size = INITIAL_BUFFER_SIZE;

  loop {
    let mut buffer = Vec::with_capacity(buffer_size);
    let buffer_bytes = buffer.capacity() * mem::size_of::<Pid>();

    // ensure we dont pass an invalid value to winapi
    // realistically your machine would crash if running that many process but... this is windows
    if buffer_bytes > u32::MAX as usize {
      return Err(WinApiError::BufferSizeError(buffer_bytes));
    }

    let mut ret_bytes = 0;

    let res = unsafe { EnumProcesses(buffer.as_mut_ptr(), buffer_bytes as u32, &mut ret_bytes) };

    if res == 0 {
      let err = get_last_error();

      return Err(WinApiError::WinApiErrorCode(err));
    }

    if (ret_bytes as usize) < buffer_bytes {
      let ret_items = ret_bytes as usize / mem::size_of::<Pid>();

      unsafe {
        // safety
        // - checked that ret_bytes < buffer_bytes
        buffer.set_len(ret_items);
      };

      return Ok(buffer);
    }

    buffer_size *= 2;
  }
}
