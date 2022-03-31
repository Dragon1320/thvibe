use std::{ffi, mem};

use winapi::shared::minwindef::HINSTANCE__;

pub type Pid = u32;

pub type Handle = ffi::c_void;
pub type RawHandle = *mut Handle;

pub type Instance = HINSTANCE__;
pub type RawInstance = *mut Instance;

lazy_static! {
  pub static ref PID_SIZE: usize = mem::size_of::<Pid>();
  pub static ref HANDLE_SIZE: usize = mem::size_of::<RawHandle>();
  pub static ref INSTANCE_SIZE: usize = mem::size_of::<RawInstance>();
}
