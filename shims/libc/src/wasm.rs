use std::alloc::Layout;

pub use core::ffi::c_char;
pub use core::ffi::c_int;
pub use core::ffi::c_long;
pub use core::ffi::c_uint;
pub use core::ffi::c_void;

pub type size_t = usize;
pub type ssize_t = isize;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct max_align_t {
  _data: [u8; 16],
}

const HEADER_SIZE: usize = 16;
const ALIGN: usize = 16;

/// # Safety
/// Same as C's `malloc`.
pub unsafe fn malloc(size: usize) -> *mut c_void {
  let Some(total) = size.checked_add(HEADER_SIZE) else {
    return core::ptr::null_mut();
  };
  let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
    return core::ptr::null_mut();
  };
  // SAFETY: the layout has a non-zero size
  let base = unsafe { std::alloc::alloc(layout) };
  if base.is_null() {
    return core::ptr::null_mut();
  }
  // SAFETY: the allocation starts with the header
  unsafe {
    base.cast::<usize>().write(total);
    base.add(HEADER_SIZE).cast()
  }
}

/// # Safety
/// Same as C's `realloc`.
pub unsafe fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
  if ptr.is_null() {
    // SAFETY: same contract as `malloc`
    return unsafe { malloc(new_size) };
  }
  let Some(new_total) = new_size.checked_add(HEADER_SIZE) else {
    return core::ptr::null_mut();
  };
  // SAFETY: `ptr` came from `malloc`, so the header precedes it
  unsafe {
    let base = ptr.cast::<u8>().sub(HEADER_SIZE);
    let total = base.cast::<usize>().read();
    let new_base = std::alloc::realloc(base, Layout::from_size_align_unchecked(total, ALIGN), new_total);
    if new_base.is_null() {
      return core::ptr::null_mut();
    }
    new_base.cast::<usize>().write(new_total);
    new_base.add(HEADER_SIZE).cast()
  }
}

/// # Safety
/// Same as C's `free`.
pub unsafe fn free(ptr: *mut c_void) {
  if ptr.is_null() {
    return;
  }
  // SAFETY: `ptr` came from `malloc`, so the header precedes it
  unsafe {
    let base = ptr.cast::<u8>().sub(HEADER_SIZE);
    let total = base.cast::<usize>().read();
    std::alloc::dealloc(base, Layout::from_size_align_unchecked(total, ALIGN));
  }
}

/// # Safety
/// Same as C's `strlen`.
pub unsafe fn strlen(s: *const c_char) -> usize {
  // SAFETY: the caller guarantees a NUL-terminated string
  unsafe { core::ffi::CStr::from_ptr(s).to_bytes().len() }
}

pub use core::ffi::c_short;

pub const AT_FDCWD: c_int = -100;
pub const ENOMEM: c_int = 12;
pub const ENOSYS: c_int = 38;

pub type sighandler_t = usize;
pub const SIG_DFL: sighandler_t = 0;
pub const SA_RESETHAND: c_int = 0x8000_0000u32 as c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigset_t {
  _bits: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigaction {
  pub sa_sigaction: sighandler_t,
  pub sa_mask: sigset_t,
  pub sa_flags: c_int,
}

/// # Safety
/// `set` has to be valid for writes.
pub unsafe fn sigemptyset(set: *mut sigset_t) -> c_int {
  // SAFETY: the caller guarantees the pointer
  unsafe { set.write(sigset_t { _bits: [0; 4] }) };
  0
}

/// There are no signals, so this does nothing.
///
/// # Safety
/// None.
pub unsafe fn sigaction(_signum: c_int, _act: *const sigaction, _oldact: *mut sigaction) -> c_int {
  0
}

pub const POLLIN: c_short = 0x1;
pub const POLLOUT: c_short = 0x4;
pub const POLLERR: c_short = 0x8;
pub const POLLHUP: c_short = 0x10;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pollfd {
  pub fd: c_int,
  pub events: c_short,
  pub revents: c_short,
}

/// There are no file descriptors to wait for, so this always fails.
///
/// # Safety
/// None.
pub unsafe fn poll(_fds: *mut pollfd, _nfds: usize, _timeout: c_int) -> c_int {
  -1
}

pub const MS_ASYNC: c_int = 1;

/// Every address of the linear memory is mapped.
///
/// # Safety
/// None.
pub unsafe fn msync(_addr: *mut c_void, _len: usize, _flags: c_int) -> c_int {
  0
}

/// # Safety
/// Both pointers have to be valid for reads of `n` bytes or up to a NUL.
pub unsafe fn strncasecmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
  for i in 0..n {
    // SAFETY: the caller guarantees both strings are readable up to `n` or their NUL
    let (x, y) = unsafe { (*a.add(i) as u8, *b.add(i) as u8) };
    let (x, y) = (x.to_ascii_lowercase(), y.to_ascii_lowercase());
    if x != y {
      return x as c_int - y as c_int;
    }
    if x == 0 {
      break;
    }
  }
  0
}
