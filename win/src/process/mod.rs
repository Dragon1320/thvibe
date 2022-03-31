use std::{cmp, ptr};

use winapi::{
  shared::{minwindef::MAX_PATH, ntdef::NULL},
  um::{
    handleapi::CloseHandle,
    processthreadsapi::OpenProcess,
    psapi::{EnumProcessModules, GetModuleBaseNameW},
  },
};

use crate::{
  error::{util::get_last_error, WinApiError},
  types::{Handle, RawHandle, RawInstance, INSTANCE_SIZE},
};

use self::error::ModuleNameError;

pub mod error;
pub mod util;

#[derive(Debug)]
pub struct Process {
  pub pid: u32,
  raw_handle: RawHandle,
}

impl Process {
  pub fn new(access: u32, inherit: bool, pid: u32) -> Result<Self, WinApiError> {
    let raw_handle = unsafe { OpenProcess(access, inherit as i32, pid) };

    if raw_handle == NULL {
      let error = get_last_error();

      Err(error)
    } else {
      Ok(Process { pid, raw_handle })
    }
  }

  pub fn borrow<'a>(&'a self) -> &'a Handle {
    unsafe { &*self.raw_handle }
  }

  pub fn borrow_mut<'a>(&'a mut self) -> &'a mut Handle {
    unsafe { &mut *self.raw_handle }
  }

  pub fn get_module_count(&self) -> Result<u32, WinApiError> {
    let mut req_bytes = 0;

    let code = unsafe { EnumProcessModules(self.raw_handle, ptr::null_mut(), 0, &mut req_bytes) };

    if code == 0 {
      let error = get_last_error();

      Err(error)
    } else {
      let num_procs = req_bytes / *INSTANCE_SIZE as u32;

      Ok(num_procs)
    }
  }

  pub fn get_modules(&self, max: u32) -> Result<Vec<Module>, WinApiError> {
    let mut instances: Vec<RawInstance> = Vec::with_capacity(max as usize);
    let mut req_bytes = 0;

    let code = unsafe {
      EnumProcessModules(
        self.raw_handle,
        instances.as_mut_ptr(),
        max * *INSTANCE_SIZE as u32,
        &mut req_bytes,
      )
    };

    if code == 0 {
      let error = get_last_error();

      Err(error)
    } else {
      let num_instances = req_bytes as usize / *INSTANCE_SIZE;

      unsafe {
        instances.set_len(cmp::min(instances.capacity(), num_instances));
      }

      instances
        .into_iter()
        .map(|instance| Module::from(self, instance))
        .collect()
    }
  }

  pub fn get_all_modules(&self) -> Result<Vec<Module>, WinApiError> {
    let req_capacity = self.get_module_count()?;

    self.get_modules(req_capacity)
  }

  pub fn get_first_module(&self) -> Result<Module, WinApiError> {
    let modules = self.get_modules(1)?;

    Ok(modules.into_iter().next().unwrap())
  }

  pub fn get_executable_name(&self) -> Result<String, ModuleNameError> {
    let module = self.get_first_module()?;

    module.get_name()
  }
}

// this is unsafe and may fail, but its better to leak the memory than panic
impl Drop for Process {
  fn drop(&mut self) {
    unsafe {
      CloseHandle(self.raw_handle);
    }
  }
}

#[derive(Debug)]
pub struct Module<'a> {
  process: &'a Process,
  instance: RawInstance,
}

impl<'a> Module<'a> {
  pub fn from(process: &'a Process, instance: RawInstance) -> Result<Self, WinApiError> {
    Ok(Module { process, instance })
  }

  pub fn get_name(&self) -> Result<String, ModuleNameError> {
    let mut name: Vec<u16> = Vec::with_capacity(MAX_PATH);

    // return value is 0 on failure
    let char_count = unsafe {
      GetModuleBaseNameW(
        // this is ok since this fn doesnt actually mutate the handle
        self.process.borrow() as *const _ as *mut _,
        self.instance,
        name.as_mut_ptr(),
        MAX_PATH as u32,
      )
    };

    if char_count == 0 {
      let error = get_last_error();

      Err(ModuleNameError::WinApiError(error))
    } else {
      unsafe {
        name.set_len(char_count as usize);
      }

      // TODO: idk if this is fine, but whatevs for now
      // apparently wide strings in windows arent actually utf-16...
      Ok(String::from_utf16(&name)?)
    }
  }
}
