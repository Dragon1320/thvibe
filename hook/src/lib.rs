#![feature(abi_thiscall)]

use std::{
  error, ffi, fmt, mem, panic, ptr, thread,
  time::{self, Duration},
};

use vibe::{init_xbone, XBone};

type Pvoid = *mut ffi::c_void;
type PthreadStartRoutine = extern "system" fn(parameter: Pvoid) -> u32;

type BoxResult<T> = Result<T, Box<dyn error::Error + Send + Sync + 'static>>;
type WinApiResult<T> = Result<T, WinApiErrorCode>;

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DllReason {
  DllProcessDetach = 0,
  DllProcessAttach = 1,
  DllThreadAtach = 2,
  DllThreadDetach = 3,
}

#[repr(i32)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Bool {
  False = 0,
  True = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum ThreadCreationFlags {
  CreateImmediate = 0,
  CreateSuspended = 0x00000004,
  StackSizeParamIsAReservation = 0x00010000,
}

// TODO: make an enum of all error codes
#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct WinApiErrorCode(u32);

impl WinApiErrorCode {
  fn new(code: u32) -> Self {
    Self(code)
  }
}

impl fmt::Display for WinApiErrorCode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "winapi error code: {}", self.0)
  }
}

impl error::Error for WinApiErrorCode {}

extern "system" {
  fn GetLastError() -> u32;
  fn DisableThreadLibraryCalls(dll_module_handle: Pvoid) -> Bool;
  fn CreateThread(
    thread_attributes: Pvoid,
    stack_size: usize,
    start_addr: PthreadStartRoutine,
    thread_parameter: Pvoid,
    creation_flags: ThreadCreationFlags,
    thread_id_ptr: *mut u32,
  ) -> Pvoid;
  fn FreeLibraryAndExitThread(module_handle: Pvoid, exit_code: u32);
  fn AllocConsole() -> Bool;
  fn GetModuleHandleA(module_name: *const i8) -> Pvoid;
  // TODO: better def
  fn VirtualProtect(addr: Pvoid, size: usize, new_protect: u32, old_protect: *mut u32) -> Bool;
  fn VirtualAlloc(addr: Pvoid, size: usize, alloc_type: u32, protect: u32) -> Pvoid;
}

fn get_last_error() -> WinApiErrorCode {
  let code = unsafe { GetLastError() };

  WinApiErrorCode::new(code)
}

// safety
// - handle must be valid
unsafe fn disable_thread_library_calls(dll_module_handle: Pvoid) -> WinApiResult<()> {
  let success_code = DisableThreadLibraryCalls(dll_module_handle);

  if success_code == Bool::False {
    let error = get_last_error();

    Err(error)
  } else {
    Ok(())
  }
}

// safety
// - thread_attributes, start_addr, thread_parameter must be valid
unsafe fn create_thread(
  thread_attributes: Pvoid,
  stack_size: usize,
  start_addr: PthreadStartRoutine,
  thread_parameter: Pvoid,
  creation_flags: ThreadCreationFlags,
) -> WinApiResult<(Pvoid, u32)> {
  let (handle, id) = {
    let mut id = 0;

    let handle = CreateThread(
      thread_attributes,
      stack_size,
      start_addr,
      thread_parameter,
      creation_flags,
      &mut id,
    );

    (handle, id)
  };

  if handle.is_null() {
    let error = get_last_error();

    Err(error)
  } else {
    Ok((handle, id))
  }
}

// safety
// - handle must be valid
unsafe fn free_library_and_exit_thread(module_handle: Pvoid, exit_code: u32) {
  FreeLibraryAndExitThread(module_handle, exit_code);
}

fn alloc_console() -> WinApiResult<()> {
  let success_code = unsafe { AllocConsole() };

  if success_code == Bool::False {
    let error = get_last_error();

    Err(error)
  } else {
    Ok(())
  }
}

// TODO: better error type
fn get_module_handle(module_name: Option<&str>) -> BoxResult<Pvoid> {
  // cstr needs to be alive for as long we need need the pointer
  // TODO: this is messy and can probably be written in a better way...
  let (_cstr, ptr) = match module_name {
    Some(name) => {
      let cstr = ffi::CString::new(name)?;
      let ptr = cstr.as_ptr();

      (cstr, ptr)
    }
    None => {
      let cstr = ffi::CString::default();
      let ptr = ptr::null();

      (cstr, ptr)
    }
  };

  let handle = unsafe { GetModuleHandleA(ptr) };

  if handle.is_null() {
    let error = get_last_error();

    Err(Box::new(error))
  } else {
    Ok(handle)
  }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(dll_module_handle: Pvoid, call_reason: DllReason, reserved: Pvoid) -> Bool {
  // TODO: tmp
  alloc_console();

  match call_reason {
    DllReason::DllProcessDetach => {
      // reserved is null when free library has been called or dll load has failed
      if reserved.is_null() {
        // free library called or dll load failed
        // need to clean up any threads or heap resources
        match panic::catch_unwind(|| dll_cleanup(dll_module_handle)) {
          Ok(detach_result) => match detach_result {
            Ok(_) => {}
            Err(error) => eprintln!("dll_cleanup errored: {:?}", error),
          },
          Err(error) => eprintln!("dll_cleanup panicked: {:?}", error),
        }
      } else {
        // process is terminating
        // all threads stopped at this point, let the os reclaim any resources
        match panic::catch_unwind(|| dll_detach(dll_module_handle)) {
          Ok(detach_result) => match detach_result {
            Ok(_) => {}
            Err(error) => eprintln!("dll_detach errored: {:?}", error),
          },
          Err(error) => eprintln!("dll_detach panicked: {:?}", error),
        }
      }

      Bool::True
    }
    DllReason::DllProcessAttach => {
      match unsafe { disable_thread_library_calls(dll_module_handle) } {
        Ok(_) => {}
        Err(error) => {
          eprintln!("disable_thread_library_calls errored: {:?}", error);

          return Bool::False;
        }
      };

      match unsafe {
        create_thread(
          ptr::null_mut(),
          0,
          pthread_dll_attach_wrapper,
          dll_module_handle,
          ThreadCreationFlags::CreateImmediate,
        )
      } {
        Ok(_) => {}
        Err(error) => {
          eprintln!("create_thread errored: {:?}", error);

          return Bool::False;
        }
      }

      Bool::True
    }
    _ => Bool::True,
  }
}

// TODO: probably should terminate this thread if DllProcessDetach is called?
extern "system" fn pthread_dll_attach_wrapper(dll_module_handle: Pvoid) -> u32 {
  // it is undefined behaviour to unwind from rust into foreign code
  // so we need to catch any panics
  match panic::catch_unwind(|| dll_attach(dll_module_handle)) {
    Ok(attach_result) => match attach_result {
      Ok(_) => {}
      Err(error) => eprintln!("dll_attach errored: {:?}", error),
    },
    Err(error) => eprintln!("dll_attach panicked: {:?}", error),
  }

  // unsafe { free_library_and_exit_thread(dll_module_handle, 1) };

  // unreachable!();

  1
}

static mut ORIG_ADDR: usize = 0;
static mut XBONE: Option<XBone> = None;

// tmp
const HEALTH_FN_OFFSET: usize = 0x3c641;
const THIS_OFFSET: usize = 0x124d380;
const JMP_REL_32: u8 = 0xe9;

// TODO: better error type
fn dll_attach(dll_module_handle: Pvoid) -> BoxResult<()> {
  println!("dll attach");

  // let object_addr = unsafe { (*(process_handle.add(THIS_OFFSET) as *mut u32)) as *mut u32 };
  // let hook_offset = unsafe { (hook as *mut u8).sub(health_fn_addr as usize).sub(5) as usize };
  // println!("{}", std::mem::size_of::<usize>());

  fn vprotecc(base_addr: *mut usize, size: usize) {
    unsafe {
      let mut old_protect = 0;

      // PAGE_EXECUTE_READWRITE = 0x40
      let success_code = VirtualProtect(base_addr as Pvoid, size, 0x40, &mut old_protect);

      if success_code == Bool::False {
        panic!();
      }
    }
  }

  fn vallocc(size: usize) -> *mut usize {
    let mem_ptr = unsafe {
      // MEM_COMMIT = 0x1000
      // MEM_RESERVE = 0x2000
      // PAGE_EXECUTE_READWRITE = 0x40
      VirtualAlloc(ptr::null_mut(), size, 0x1000 | 0x2000, 0x40)
    };

    if mem_ptr.is_null() {
      panic!();
    }

    mem_ptr as *mut usize
  }

  fn setup_hook(base_addr: *mut usize, jump_addr: *mut usize) -> Vec<u8> {
    vprotecc(base_addr, 5);

    let mut stolen_bytes = Vec::with_capacity(5);

    unsafe {
      ptr::copy_nonoverlapping(base_addr as *mut u8, stolen_bytes.as_mut_ptr(), 5);

      stolen_bytes.set_len(5);
    }

    let hook_offset = jump_addr as usize - base_addr as usize - 5;

    let hook_bytes = Some(JMP_REL_32)
      .into_iter()
      .chain(hook_offset.to_le_bytes())
      .collect::<Vec<_>>();

    unsafe {
      ptr::copy_nonoverlapping(hook_bytes.as_ptr(), base_addr as *mut u8, 5);
    }

    stolen_bytes
  }

  fn setup_trampoline(base_addr: *mut usize, jump_addr: *mut usize) -> extern "thiscall" fn(*mut usize, i32) -> i32 {
    const ALLOC_SIZE: usize = 1024;

    let stolen_bytes = setup_hook(base_addr, jump_addr);
    let alloc = vallocc(ALLOC_SIZE);

    // (base + 5) - (alloc + 5) - 5
    let base_offset = (base_addr as isize + 5) - (alloc as isize + 5) - 5;

    let tramp_bytes = stolen_bytes
      .into_iter()
      .chain(Some(JMP_REL_32))
      .chain(base_offset.to_le_bytes())
      .collect::<Vec<_>>();

    let orig = unsafe {
      ptr::copy_nonoverlapping(tramp_bytes.as_ptr(), alloc as *mut u8, 10);

      mem::transmute(alloc)
    };

    // tmp
    unsafe {
      ORIG_ADDR = alloc as usize;
    };
    //

    orig
  }

  // base addr
  let process_addr = get_module_handle(None)? as usize;

  println!("base addr: {:x?}", process_addr);

  // hook health fn
  let health_fn_addr = process_addr + HEALTH_FN_OFFSET;

  let orig = setup_trampoline(health_fn_addr as *mut usize, hook as *mut usize);

  // test
  let object_addr = process_addr + THIS_OFFSET;
  let other = unsafe { *(object_addr as *mut usize) };

  // orig(other as *mut usize, -1);

  // println!("lol");

  // unsafe {
  //   ptr::copy_nonoverlapping(payload.as_ptr(), health_fn_addr, 5);
  // }

  // println!("old bytes: {:?}", prev_bytes);
  // println!("payload: {:?}", payload);

  // // i love trampolining
  // let mem_ptr = unsafe {
  //   // MEM_COMMIT = 0x1000
  //   // MEM_RESERVE = 0x2000
  //   // PAGE_EXECUTE_READWRITE = 0x40
  //   VirtualAlloc(ptr::null_mut(), 1024, 0x1000 | 0x2000, 0x40)
  // };

  // if mem_ptr.is_null() {
  //   panic!();
  // }

  // println!("trampoline addr: {:?}", mem_ptr);

  // let mut trampoline_jmp = Vec::with_capacity(5);
  // trampoline_jmp.push(JMP_REL_32);

  // unsafe {
  //   let trampoline_offset = health_fn_addr.add(5).sub((mem_ptr as *mut u8).add(5) as usize).sub(5) as usize;
  //   let offset_bytes = trampoline_offset.to_le_bytes();

  //   ptr::copy_nonoverlapping(offset_bytes.as_ptr(), trampoline_jmp.as_mut_ptr().add(1), 4);
  //   trampoline_jmp.set_len(5);
  // }

  // unsafe {
  //   ptr::copy_nonoverlapping(prev_bytes.as_ptr(), mem_ptr as *mut _, 5);
  //   ptr::copy_nonoverlapping(trampoline_jmp.as_ptr(), mem_ptr.add(5) as *mut _, 5);
  // }

  // let f: extern "thiscall" fn(*const usize, i32) -> i32 = unsafe { mem::transmute(health_fn_ptr) };

  // f(other as *const usize, -1);

  let xbone = init_xbone()?;

  unsafe {
    XBONE = Some(xbone);
  }

  thread::sleep(time::Duration::from_millis(5000));

  Ok(())
}

// TODO: better error type
fn dll_detach(dll_module_handle: Pvoid) -> BoxResult<()> {
  println!("dll detach");

  Ok(())
}

// TODO: better error type
fn dll_cleanup(dll_module_handle: Pvoid) -> BoxResult<()> {
  println!("dll cleanup");

  Ok(())
}

extern "thiscall" fn hook(object: *mut usize, delta: i32) {
  println!("owo hooked {:?} {:?}", object, delta);

  unsafe {
    match &XBONE {
      Some(xbone) => xbone.vibe(1.0, Duration::from_millis(5000)).unwrap(),
      None => {}
    }
  }

  let orig: extern "thiscall" fn(*mut usize, i32) -> i32 = unsafe { mem::transmute(ORIG_ADDR as *mut usize) };

  orig(object, delta);
}
