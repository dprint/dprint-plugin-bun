pub mod configuration;
mod file_matching;
mod format_text;
mod options;
mod shims;

pub use file_matching::file_extensions;
pub use file_matching::file_names;

pub use format_text::format_text;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm_plugin;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use wasm_plugin::*;
