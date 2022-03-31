use winapi::{
  shared::winerror::{ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, ERROR_PARTIAL_COPY},
  um::{
    psapi::EnumProcesses,
    winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
  },
};

use crate::{
  error::{util::get_last_error, WinApiError},
  types::{Pid, PID_SIZE},
};

use super::{error::ModuleNameError, Process};

// note: capacity is a u32 as thats what the winapi fn expects, its also
// nicer to convert a u32 -> usize than the other way around
pub fn get_process_ids(max: u32) -> Result<Vec<Pid>, WinApiError> {
  let mut pids: Vec<Pid> = Vec::with_capacity(max as usize);
  let mut written_bytes = 0;

  let code = unsafe { EnumProcesses(pids.as_mut_ptr(), max * *PID_SIZE as u32, &mut written_bytes) };

  // non-zero return value = success
  if code == 0 {
    let error = get_last_error();

    Err(error)
  } else {
    let num_pids = written_bytes as usize / *PID_SIZE;

    unsafe {
      pids.set_len(num_pids);
    }

    Ok(pids)
  }
}

pub fn get_all_process_ids(start_cap: u32) -> Result<Vec<Pid>, WinApiError> {
  let mut idx = 1;

  loop {
    let capacity = start_cap * idx;

    let pids = get_process_ids(capacity)?;

    if pids.len() < capacity as usize {
      return Ok(pids);
    }

    idx += 1;
  }
}

/// finds process by first module name and returns a pid + handle to that process
pub fn find_process_by_name(name: &str, start_cap: u32) -> Result<Option<Process>, ModuleNameError> {
  let pids = get_all_process_ids(start_cap)?;

  for pid in pids {
    let process = match Process::new(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
      Ok(process) => process,
      Err(error) => {
        if pid == 0 && error.code == ERROR_INVALID_PARAMETER {
          continue;
        }

        if error.code == ERROR_ACCESS_DENIED {
          continue;
        }

        return Err(ModuleNameError::WinApiError(error));
      }
    };

    match process.get_executable_name() {
      Ok(exe_name) => {
        if exe_name == name {
          return Ok(Some(process));
        }
      }
      Err(error) => match error {
        ModuleNameError::WinApiError(error) => {
          if error.code == ERROR_PARTIAL_COPY {
            continue;
          }
        }
        _ => return Err(error),
      },
    }
  }

  Ok(None)
}
