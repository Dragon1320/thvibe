use std::{error, ffi, fmt};

pub type BoxResult<T> = Result<T, Box<dyn error::Error + Send + Sync + 'static>>;
pub type WinApiResult<T> = Result<T, WinApiError>;
pub type WinApiCodeResult<T> = Result<T, WinApiErrorCode>;

pub mod util;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum WinApiError {
  WinApiErrorCode(WinApiErrorCode),
  StringParseError(ffi::OsString),
  BufferSizeError(usize),
}

impl fmt::Display for WinApiError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      WinApiError::WinApiErrorCode(winapi_code) => winapi_code.fmt(f),
      WinApiError::StringParseError(os_string) => write!(f, "error parsing winapi string - {:?}", os_string),
      WinApiError::BufferSizeError(size) => write!(f, "invalid buffer size - {}", size),
    }
  }
}

impl error::Error for WinApiError {}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct WinApiErrorCode(u32);

impl WinApiErrorCode {
  fn new(code: u32) -> Self {
    Self(code)
  }
}

impl fmt::Display for WinApiErrorCode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "winapi error code - {}", self.0)
  }
}

impl error::Error for WinApiErrorCode {}
