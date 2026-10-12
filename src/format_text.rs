use std::path::Path;

use bun_lint_driver::fmt::format_for_plugin;
use bun_lint_driver::fmt::is_formatted_for_plugin;

use crate::configuration::Configuration;
use crate::options::build_format_options;

type FormatError = Box<dyn std::error::Error + Send + Sync>;

pub fn format_text(file_path: &Path, input_text: &str, config: &Configuration) -> Result<Option<String>, FormatError> {
  let options = build_format_options(config)?;
  // Bun goes by the name of the file, and writes a path with forward slashes
  let file_path = file_path.to_string_lossy().replace('\\', "/");
  let file_path = file_path.as_bytes();

  if !is_formatted_for_plugin(file_path, &options) {
    return Ok(None);
  }

  let output = match format_for_plugin(file_path, input_text.as_bytes(), &options) {
    Ok(output) => String::from_utf8(output)?,
    // the message is the one that `bun format` prints
    Err(message) => return Err(String::from_utf8_lossy(&message).into_owned().into()),
  };

  if output == input_text {
    Ok(None)
  } else {
    Ok(Some(output))
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn formats_basic_js() {
    let input = "const x=1";
    let config = crate::configuration::Configuration::default();
    let result = format_text(std::path::Path::new("test.js"), input, &config)
      .unwrap()
      .unwrap();
    assert_eq!(result, "const x = 1;\n");
  }
}
