use std::{error, ffi, mem};

use injector::load_module;
use win::process::{util::enum_processes, WinApiProcess};
use winapi::um::{
  libloaderapi::{GetModuleHandleA, GetProcAddress},
  winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
};

pub type BoxResult<T> = Result<T, Box<dyn error::Error + Send + Sync + 'static>>;

fn main() -> BoxResult<()> {
  let proc_ids = enum_processes()?;

  let mut th08 = 0;

  for &pid in proc_ids.iter() {
    let proc = WinApiProcess::new(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ);

    if proc.is_err() {
      continue;
    }

    let proc = proc?;

    let name = proc.get_name();

    if name.is_err() {
      continue;
    }

    let name = name?;

    if name == "th08.exe" {
      th08 = pid;
    }

    // println!("{}", name);
  }

  if th08 == 0 {
    panic!("th08 process not found");
  }

  println!("th08 pid: {}", th08);

  load_module("target/debug/hook.dll", th08)?;

  Ok(())
}
