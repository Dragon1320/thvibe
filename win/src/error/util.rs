use winapi::um::errhandlingapi::GetLastError;

use super::WinApiError;

pub fn get_last_error() -> WinApiError {
  let code = unsafe { GetLastError() };

  WinApiError::new(code)
}
