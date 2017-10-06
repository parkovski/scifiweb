#![allow(bad_style)]

use std::sync::Mutex;
use std::ptr::{null, null_mut};
use std::mem::{transmute, zeroed, size_of, uninitialized};
use std::slice;
use std::io::{Write, StderrLock};
use std::panic::{PanicInfo, set_hook};
use std::process::abort;
use std::env::current_exe;
use std::ffi::{OsStr, OsString, CString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use winapi::minwindef::{HMODULE, BOOL, DWORD, PDWORD, TRUE, FALSE};
use winapi::winnt::{PCWSTR, LPWSTR, PVOID, HANDLE};
use winapi::basetsd::{DWORD64, PDWORD64};
use winapi::dbghelp::{
  SYMBOL_INFOW, PSYMBOL_INFOW,
  IMAGEHLP_LINEW64, PIMAGEHLP_LINEW64,
};
use kernel32::{
  LoadLibraryExW, GetProcAddress, FreeLibrary,
  GetCurrentProcess,
  RtlCaptureStackBackTrace,
  GetLastError, FormatMessageW, LocalFree,
};

fn wstr<S: AsRef<OsStr> + ?Sized>(text: &S) -> Vec<u16> {
  text.as_ref().encode_wide().collect()
}

unsafe fn wstr_slice<'a>(wstr: *const u16) -> &'a [u16] {
  let mut i = 0;
  loop {
    if *wstr.offset(i) == 0 { break; }
    i += 1;
  }
  slice::from_raw_parts(wstr, i as usize)
}

/// Note: On XP this + frames to skip must be less than 62.
/// Don't intend to run on XP though.
const MAX_FRAMES: usize = 100;
const SYM_INFO_BYTES: usize = 2000;

type SymInitializeWFn = unsafe extern "system" fn(HANDLE, PCWSTR, BOOL) -> BOOL;
type SymSetOptionsFn = unsafe extern "system" fn(DWORD) -> DWORD;
type SymFromAddrWFn = unsafe extern "system" fn(
  HANDLE, DWORD64, PDWORD64, PSYMBOL_INFOW,
) -> BOOL;
type SymGetLineFromAddrW64Fn = unsafe extern "system" fn(
  HANDLE, DWORD64, PDWORD, PIMAGEHLP_LINEW64,
) -> BOOL;
type SymCleanupFn = unsafe extern "system" fn(HANDLE) -> BOOL;

macro_rules! get_proc_address {
  ($module:expr, $name:expr) => (
    {
      let addr = GetProcAddress(
        $module,
        CString::new($name).unwrap().as_ptr() as *const _,
      );
      if addr == null() {
        None
      } else {
        Some(transmute(addr))
      }
    }
  );
}

struct DbgHelp {
  pub SymFromAddrW: SymFromAddrWFn,
  pub SymGetLineFromAddrW64: SymGetLineFromAddrW64Fn,

  pub process: HANDLE,
  dll: HMODULE,
  SymCleanup: Option<SymCleanupFn>,
}

impl DbgHelp {
  pub unsafe fn new() -> Result<Self, DWORD> {
    let (dll_name, dll_load_flags) = match current_exe() {
      Ok(ref mut path) => {
        path.pop();
        path.push("dbghelp.dll");
        if path.exists() {
          (wstr(path), 0x8 /* LOAD_WITH_ALTERED_SEARCH_PATH */)
        } else {
          (wstr("dbghelp.dll"), 0)
        }
      }
      Err(_) => (wstr("dbghelp.dll"), 0),
    };
    let dll = LoadLibraryExW(dll_name.as_ptr(), null_mut(), dll_load_flags);
    if dll == null_mut() {
      return Err(GetLastError());
    }
    let process = GetCurrentProcess();
    let maybe_SymSetOptions: Option<SymSetOptionsFn>
      = get_proc_address!(dll, "SymSetOptions");
    if let Some(SymSetOptions) = maybe_SymSetOptions {
      SymSetOptions(0x12 /* SYMOPT_LOAD_LINES | SYMOPT_UNDNAME */);
    }
    let maybe_SymInitializeW: Option<SymInitializeWFn>
      = get_proc_address!(dll, "SymInitializeW");
    if let Some(SymInitializeW) = maybe_SymInitializeW {
      if SymInitializeW(process, null(), TRUE) == FALSE {
        let err = GetLastError();
        FreeLibrary(dll);
        return Err(err);
      }
    }
    let SymCleanup: Option<SymCleanupFn>
      = get_proc_address!(dll, "SymCleanup");
    let SymFromAddrW: SymFromAddrWFn =
      if let Some(f) = get_proc_address!(dll, "SymFromAddrW") {
        f
      } else {
        let err = GetLastError();
        FreeLibrary(dll);
        SymCleanup.map(|sc| sc(process));
        return Err(err);
      };
    let SymGetLineFromAddrW64: SymGetLineFromAddrW64Fn
      = if let Some(f) = get_proc_address!(dll, "SymGetLineFromAddrW64") {
        f
      } else {
        let err = GetLastError();
        FreeLibrary(dll);
        SymCleanup.map(|sc| sc(process));
        return Err(err);
      };
    Ok(DbgHelp {
      SymFromAddrW,
      SymGetLineFromAddrW64,
      process,
      dll,
      SymCleanup,
    })
  }
}

impl Drop for DbgHelp {
  fn drop(&mut self) { unsafe {
    if let Some(SymCleanup) = self.SymCleanup {
      SymCleanup(self.process);
    }
    FreeLibrary(self.dll);
  }}
}

lazy_static! {
  /// Dbghelp is a single threaded library.
  static ref STACK_TRACE_MUTEX: Mutex<()> = Mutex::new(());
}

fn print_last_error(stderr: &mut StderrLock, err: DWORD) { unsafe {
  let mut buffer: LPWSTR = zeroed();
  // Flags = FORMAT_MESSAGE_ALLOCATE_BUFFER
  //       | FORMAT_MESSAGE_FROM_SYSTEM
  //       | FORMAT_MESSAGE_IGNORE_INSERTS
  //       | FORMAT_MESSAGE_MAX_WIDTH_MASK
  if FormatMessageW(
    0x13FF, null(), err,
    0, (&mut buffer) as *mut *mut _ as *mut _,
    0, null_mut()
  ) == 0
  {
    let _ = write!(stderr, "Unknown system error ({:#X})\n", err);
    return;
  }
  let message = OsString::from_wide(wstr_slice(buffer));
  LocalFree(buffer as *mut _);
  let _ = write!(
    stderr,
    "System error: {} ({:#X})\n",
    message.to_string_lossy(),
    err,
  );
}}

fn print_stack_trace(mut stderr: StderrLock) { unsafe {
  // If it's poisoned, that's a double panic, so just abort.
  let _thread_guard = match STACK_TRACE_MUTEX.lock() {
    Ok(guard) => guard,
    Err(_) => abort(),
  };
  let mut stack: [PVOID; MAX_FRAMES] = [uninitialized(); MAX_FRAMES];
  // Frames to skip:
  // - self::print_stack_trace
  // - self::panic_hook
  // - Fn::call (calls the panic hook)
  // - rust_panic_with_hook
  // Puts the top of the stack trace at begin_panic.
  let frames = RtlCaptureStackBackTrace(
    4,
    MAX_FRAMES as u32,
    stack.as_mut_ptr(),
    null_mut(),
  );
  if frames == 0 {
    print_last_error(&mut stderr, GetLastError());
    return;
  }
  let d = match DbgHelp::new() {
    Ok(d) => d,
    Err(e) => {
      print_last_error(&mut stderr, e);
      let _ = write!(stderr, "Stack trace:\n");
      for i in 0..frames {
        let _ = write!(stderr, "{}: {:#X}\n", i, stack[i as usize] as usize);
      }
      return
    }
  };
  let _ = stderr.write("Stack trace:\n".as_bytes());
  for i in 0..frames {
    let mut _sym: [u8; SYM_INFO_BYTES] = [uninitialized(); SYM_INFO_BYTES];
    let psym: PSYMBOL_INFOW = transmute(_sym.as_mut_ptr());
    (*psym).SizeOfStruct = size_of::<SYMBOL_INFOW>() as u32;
    (*psym).MaxNameLen = SYM_INFO_BYTES as u32 - (*psym).SizeOfStruct - 20;
    let mut displacement: DWORD64 = 0;
    if (d.SymFromAddrW)(
      d.process,
      stack[i as usize] as DWORD64,
      &mut displacement,
      psym
    ) == FALSE
    {
      let _ = write!(stderr, "{}: {:#X}\n", i, stack[i as usize] as usize);
      continue;
    }
    let mut line: IMAGEHLP_LINEW64 = zeroed();
    let mut displacement: DWORD = 0;
    line.SizeOfStruct = size_of::<IMAGEHLP_LINEW64>() as u32;
    if (d.SymGetLineFromAddrW64)(
      d.process,
      stack[i as usize] as DWORD64,
      &mut displacement,
      &mut line
    ) == TRUE
    {
      let _ = write!(
        stderr,
        "{}: {}\n  {} Line {}\n",
        i,
        OsString::from_wide(wstr_slice((*psym).Name.as_ptr())).to_string_lossy(),
        OsString::from_wide(wstr_slice(line.FileName)).to_string_lossy(),
        line.LineNumber,
      );
    } else {
      let _ = write!(
        stderr,
        "{}: {}\n  {:#X}\n",
        i,
        OsString::from_wide(wstr_slice((*psym).Name.as_ptr())).to_string_lossy(),
        stack[i as usize] as usize,
      );
    }
  }
}}

fn panic_hook(panic_info: &PanicInfo) {
  let mut _stderr = ::std::io::stderr();
  let mut stderr = _stderr.lock();

  let _payload = panic_info.payload();
  let payload = {
    if let Some(s) = _payload.downcast_ref::<&str>() {
      s
    } else if let Some(s) = _payload.downcast_ref::<String>() {
      s
    } else {
      ""
    }
  };
  let _ = if !payload.is_empty() {
    write!(
      stderr,
      "\nPanic: {}",
      payload,
    )
  } else {
    write!(stderr, "\nPanic")
  };
  let _ = if let Some(location) = panic_info.location() {
    write!(stderr, "\n  at {} line {}\n", location.file(), location.line())
  } else {
    write!(stderr, "\n")
  };

  print_stack_trace(stderr);
}

pub fn set_panic_hook() {
  set_hook(Box::new(panic_hook));
}
