use serde_json::Value;

use crate::configuration::Configuration;
use crate::configuration::Flavor;
use crate::configuration::Language;

/// What Bun is given as the configuration.
pub struct BunConfig {
  /// What a configuration file of Prettier or of oxfmt has.
  pub json: String,
  pub is_oxfmt: bool,
}

/// Gets the configuration for Bun to format a file of the language with.
pub fn build_bun_config_for(config: &Configuration, language: Option<Language>) -> BunConfig {
  if language == Some(Language::Toml) && config.flavor != Some(Flavor::Oxfmt) {
    build_toml_config(config)
  } else {
    build_bun_config(config)
  }
}

/// Gets the configuration for Bun, which reads it the way it reads a
/// configuration file: the options of the plugin have the names and the values
/// that they have there.
pub fn build_bun_config(config: &Configuration) -> BunConfig {
  let is_oxfmt = config.flavor == Some(Flavor::Oxfmt);
  let mut options = match serde_json::to_value(config) {
    Ok(Value::Object(options)) => options,
    _ => unreachable!("the configuration is a map"),
  };
  // the flavor is what kind of file it is, and an option that is not set is not in one
  options.remove("flavor");
  // which files are formatted is up to the plugin
  options.remove("disabledLanguages");
  options.retain(|_, value| !value.is_null());
  BunConfig {
    json: Value::Object(options).to_string(),
    is_oxfmt,
  }
}

/// Gets the configuration for Bun to format TOML with when the flavor is the
/// one of Prettier, which has no TOML: Bun only formats it the way oxfmt does.
/// So it is given a configuration of oxfmt that has the options that TOML is
/// formatted with, which have the same names and values in both.
fn build_toml_config(config: &Configuration) -> BunConfig {
  let mut options = serde_json::Map::new();
  // lines are 100 wide by default in oxfmt
  options.insert("printWidth".to_string(), config.print_width.unwrap_or(80).into());
  if let Some(tab_width) = config.tab_width {
    options.insert("tabWidth".to_string(), tab_width.into());
  }
  if let Some(use_tabs) = config.use_tabs {
    options.insert("useTabs".to_string(), use_tabs.into());
  }
  if let Some(end_of_line) = config.end_of_line {
    options.insert("endOfLine".to_string(), end_of_line.to_string().into());
  }
  if let Some(trailing_comma) = config.trailing_comma {
    options.insert("trailingComma".to_string(), trailing_comma.to_string().into());
  }
  BunConfig {
    json: Value::Object(options).to_string(),
    is_oxfmt: true,
  }
}
