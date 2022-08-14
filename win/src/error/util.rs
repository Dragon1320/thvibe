use winapi::um::errhandlingapi::GetLastError;

use super::WinApiErrorCode;

pub fn get_last_error() -> WinApiErrorCode {
  let code = unsafe { GetLastError() };

  WinApiErrorCode::new(code)
}
