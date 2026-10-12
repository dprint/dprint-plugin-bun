//! Lists the spec files for the tests. The tests are Wasm, and not every
//! runtime that runs them can list a directory.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
  let specs_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("tests/specs");
  println!("cargo:rerun-if-changed={}", specs_dir.display());

  let mut names = fs::read_dir(&specs_dir)
    .unwrap()
    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
    .collect::<Vec<_>>();
  names.sort();

  let mut code = String::from("&[\n");
  for name in names {
    let path = specs_dir.join(&name);
    code.push_str(&format!(
      "  ({name:?}, include_str!({:?})),\n",
      path.display().to_string()
    ));
  }
  code.push_str("]\n");
  fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("specs.rs"), code).unwrap();
}
