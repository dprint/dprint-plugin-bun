//! Stands in for the `libc` crate in Bun's crates. It is the real crate on the
//! targets that have a C library and a small implementation in Rust on Wasm.
#![allow(non_camel_case_types)]

#[cfg(not(target_family = "wasm"))]
pub use real_libc::*;

#[cfg(target_family = "wasm")]
mod wasm;
#[cfg(target_family = "wasm")]
pub use wasm::*;
