//! What Bun's crates ask of the rest of Bun.

use std::ffi::c_void;

/// The lowest address that the stack may reach, which Bun's recursive code
/// keeps a distance from.
#[unsafe(no_mangle)]
pub extern "C" fn Bun__StackCheck__getMaxStack() -> *mut c_void {
  stack_low() as *mut c_void
}

/// Bun finds the bounds of the stack of a thread with this. They are looked up
/// when they are asked for instead.
#[unsafe(no_mangle)]
pub extern "C" fn Bun__StackCheck__initialize() {}

/// Bun's crash handler reports that the memory ran out, and then aborts.
#[unsafe(no_mangle)]
pub fn __bun_crash_handler_out_of_memory() -> ! {
  panic!("bun: out of memory")
}

/// Puts the terminal back as Bun found it, which nothing here changes.
#[unsafe(no_mangle)]
pub extern "C" fn bun_restore_stdio() {}

/// The stack of the Wasm module is the part of its memory that the linker sets
/// aside for it, and it grows down.
#[cfg(target_family = "wasm")]
fn stack_low() -> usize {
  unsafe extern "C" {
    static __stack_low: u8;
  }
  &raw const __stack_low as usize
}

/// Assumes that a megabyte below the first frame that asks belongs to the
/// stack, which is the least that the main thread of a system has.
#[cfg(not(target_family = "wasm"))]
fn stack_low() -> usize {
  const STACK_SIZE: usize = 1024 * 1024;
  thread_local! {
    static STACK_LOW: usize = {
      let on_the_stack = 0u8;
      (&raw const on_the_stack as usize).saturating_sub(STACK_SIZE)
    };
  }
  STACK_LOW.with(|low| *low)
}
