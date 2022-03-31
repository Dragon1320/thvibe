use std::{error, fmt};

pub mod util;

#[derive(Debug, Clone, Copy)]
pub struct WinApiError {
  pub code: u32,
}

impl WinApiError {
  pub fn new(code: u32) -> Self {
    WinApiError { code }
  }
}

impl fmt::Display for WinApiError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "winapi error code: {}", self.code)
  }
}

impl error::Error for WinApiError {}
