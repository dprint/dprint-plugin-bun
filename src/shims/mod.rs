//! Bun's formatter is a part of a program that is linked with libraries in C
//! and C++. These are the functions of them that the formatter's crates call,
//! written in Rust so that the formatter can be built on its own and for Wasm.

mod collate;
mod highway;
mod mimalloc;
mod runtime;
mod simdutf;
mod string_width;
mod wtf;
mod yarr;
