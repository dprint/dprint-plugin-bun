//! Lists the spec files for the tests. The tests are Wasm, and not every
//! runtime that runs them can list a directory.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
  generate_string_width_table();
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

/// copies Bun's Unicode classification tables so width calculations use its Unicode version.
fn generate_string_width_table() {
  let path = "vendor/bun/src/jsc/bindings/stringWidthTables.h";
  println!("cargo:rerun-if-changed={path}");
  let header = fs::read_to_string(path).unwrap();
  let mut code = String::new();
  for (stage, ty) in [(1, "u16"), (2, "u8"), (3, "u8")] {
    let name = format!("kGraphemeBreakStage{stage}");
    let (_, declaration) = header.split_once(&name).unwrap();
    let (length, values) = declaration.split_once(" = {").unwrap();
    let (values, _) = values.split_once("};").unwrap();
    code.push_str(&format!(
      "static STAGE{stage}: [{ty}; {}] = [{values}];\n",
      &length[1..length.len() - 1]
    ));
  }
  fs::write(
    Path::new(&env::var("OUT_DIR").unwrap()).join("string_width_table.rs"),
    code,
  )
  .unwrap();
}
