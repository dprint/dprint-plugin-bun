use serde_json::Value;

use crate::configuration::Configuration;
use crate::configuration::Flavor;

/// What Bun is given as the configuration.
pub struct BunConfig {
  /// What a configuration file of Prettier or of oxfmt has.
  pub json: String,
  pub is_oxfmt: bool,
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
