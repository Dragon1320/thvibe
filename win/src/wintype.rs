use std::ffi;

pub type Pid = u32;
pub type Wchar = u16;
pub type ProcessHandle = *mut ffi::c_void;
