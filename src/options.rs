use bun_format::FormatOptions;

use crate::configuration::Configuration;

/// Gets the options of Bun's formatter for the configuration.
///
/// Bun has a setter that takes the names and the values of Prettier's options
/// as they are written in a configuration file, which is what the
/// configuration of the plugin has.
pub fn build_format_options(config: &Configuration) -> Result<FormatOptions, String> {
  let mut options = FormatOptions::default();
  let mut set = |name: &str, value: Option<String>| match value {
    Some(value) => options
      .set(name.as_bytes(), value.as_bytes())
      .map_err(|_| format!("Invalid value for the option \"{name}\": {value}")),
    None => Ok(()),
  };

  set("printWidth", config.print_width.map(|value| value.to_string()))?;
  set("tabWidth", config.tab_width.map(|value| value.to_string()))?;
  set("useTabs", config.use_tabs.map(|value| value.to_string()))?;
  set("endOfLine", config.end_of_line.map(|value| value.to_string()))?;
  set("semi", config.semi.map(|value| value.to_string()))?;
  set("singleQuote", config.single_quote.map(|value| value.to_string()))?;
  set("jsxSingleQuote", config.jsx_single_quote.map(|value| value.to_string()))?;
  set("quoteProps", config.quote_props.map(|value| value.to_string()))?;
  set("trailingComma", config.trailing_comma.map(|value| value.to_string()))?;
  set("bracketSpacing", config.bracket_spacing.map(|value| value.to_string()))?;
  set(
    "bracketSameLine",
    config.bracket_same_line.map(|value| value.to_string()),
  )?;
  set("arrowParens", config.arrow_parens.map(|value| value.to_string()))?;
  set("objectWrap", config.object_wrap.map(|value| value.to_string()))?;
  set(
    "singleAttributePerLine",
    config.single_attribute_per_line.map(|value| value.to_string()),
  )?;
  set(
    "experimentalOperatorPosition",
    config.experimental_operator_position.map(|value| value.to_string()),
  )?;
  set(
    "experimentalTernaries",
    config.experimental_ternaries.map(|value| value.to_string()),
  )?;
  set(
    "embeddedLanguageFormatting",
    config.embedded_language_formatting.map(|value| value.to_string()),
  )?;
  set("proseWrap", config.prose_wrap.map(|value| value.to_string()))?;
  set(
    "htmlWhitespaceSensitivity",
    config.html_whitespace_sensitivity.map(|value| value.to_string()),
  )?;
  set(
    "vueIndentScriptAndStyle",
    config.vue_indent_script_and_style.map(|value| value.to_string()),
  )?;
  set("insertPragma", config.insert_pragma.map(|value| value.to_string()))?;
  set("requirePragma", config.require_pragma.map(|value| value.to_string()))?;
  set(
    "checkIgnorePragma",
    config.check_ignore_pragma.map(|value| value.to_string()),
  )?;

  Ok(options)
}
