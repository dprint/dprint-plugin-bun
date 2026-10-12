use std::ops::Range;
use std::path::Path;

use bun_lint_driver::fmt::format_for_plugin;
use bun_lint_driver::fmt::is_formatted_for_plugin;

use crate::configuration::Configuration;
use crate::file_matching::language_of;
use crate::options::build_bun_config_for;

type FormatError = Box<dyn std::error::Error + Send + Sync>;

pub fn format_text(file_path: &Path, input_text: &str, config: &Configuration) -> Result<Option<String>, FormatError> {
  format_range(file_path, input_text, None, config)
}

/// Formats what is in the range of bytes, or all of the text without one.
pub fn format_range(
  file_path: &Path,
  input_text: &str,
  range: Option<Range<usize>>,
  config: &Configuration,
) -> Result<Option<String>, FormatError> {
  let language = language_of(file_path);
  if language.is_some_and(|language| !config.is_language_enabled(language)) {
    return Ok(None);
  }

  let bun_config = build_bun_config_for(config, language);
  let bun_config = (bun_config.json.as_bytes(), bun_config.is_oxfmt);
  // Bun goes by the name of the file, and writes a path with forward slashes
  let file_path = file_path.to_string_lossy().replace('\\', "/");
  let file_path = file_path.as_bytes();

  if !is_formatted_for_plugin(file_path, bun_config.0, bun_config.1) {
    return Ok(None);
  }

  // Bun counts in UTF-16 code units, as Prettier does
  let to_utf16_offset = |offset: usize| match input_text.get(..offset) {
    Some(before) => Ok(before.encode_utf16().count() as u32),
    None => Err("The range is not at the boundary of a character."),
  };
  let range = match range {
    Some(range) => Some((to_utf16_offset(range.start)?, to_utf16_offset(range.end)?)),
    None => None,
  };

  let output = match format_for_plugin(file_path, input_text.as_bytes(), bun_config, range) {
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
