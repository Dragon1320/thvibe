use std::string::FromUtf16Error;

use crate::error::WinApiError;

#[derive(Debug)]
pub enum ModuleNameError {
  WinApiError(WinApiError),
  InvalidName(FromUtf16Error),
}

impl From<WinApiError> for ModuleNameError {
  fn from(winapi_error: WinApiError) -> Self {
    ModuleNameError::WinApiError(winapi_error)
  }
}

impl From<FromUtf16Error> for ModuleNameError {
  fn from(utf16_error: FromUtf16Error) -> Self {
    ModuleNameError::InvalidName(utf16_error)
  }
}
