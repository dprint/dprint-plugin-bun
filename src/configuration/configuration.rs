use dprint_core::configuration::ParseConfigurationError;
use dprint_core::generate_str_to_from;
use serde::{Deserialize, Serialize};

/// The options of `bun format`, which are the ones of Prettier with the same
/// names. An option that is not set has the default of `bun format`.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
  pub print_width: Option<u32>,
  pub tab_width: Option<u8>,
  pub use_tabs: Option<bool>,
  pub end_of_line: Option<EndOfLine>,
  pub semi: Option<bool>,
  pub single_quote: Option<bool>,
  pub jsx_single_quote: Option<bool>,
  pub quote_props: Option<QuoteProps>,
  pub trailing_comma: Option<TrailingComma>,
  pub bracket_spacing: Option<bool>,
  pub bracket_same_line: Option<bool>,
  pub arrow_parens: Option<ArrowParens>,
  pub object_wrap: Option<ObjectWrap>,
  pub single_attribute_per_line: Option<bool>,
  pub experimental_operator_position: Option<OperatorPosition>,
  pub experimental_ternaries: Option<bool>,
  pub embedded_language_formatting: Option<EmbeddedLanguageFormatting>,
  pub prose_wrap: Option<ProseWrap>,
  pub html_whitespace_sensitivity: Option<HtmlWhitespaceSensitivity>,
  pub vue_indent_script_and_style: Option<bool>,
  pub insert_pragma: Option<bool>,
  pub require_pragma: Option<bool>,
  pub check_ignore_pragma: Option<bool>,
}

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndOfLine {
  Lf,
  Crlf,
  Cr,
  Auto,
}

generate_str_to_from![EndOfLine, [Lf, "lf"], [Crlf, "crlf"], [Cr, "cr"], [Auto, "auto"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuoteProps {
  AsNeeded,
  Consistent,
  Preserve,
}

generate_str_to_from![
  QuoteProps,
  [AsNeeded, "as-needed"],
  [Consistent, "consistent"],
  [Preserve, "preserve"]
];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrailingComma {
  All,
  Es5,
  None,
}

generate_str_to_from![TrailingComma, [All, "all"], [Es5, "es5"], [None, "none"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArrowParens {
  Always,
  Avoid,
}

generate_str_to_from![ArrowParens, [Always, "always"], [Avoid, "avoid"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectWrap {
  Preserve,
  Collapse,
}

generate_str_to_from![ObjectWrap, [Preserve, "preserve"], [Collapse, "collapse"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperatorPosition {
  Start,
  End,
}

generate_str_to_from![OperatorPosition, [Start, "start"], [End, "end"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmbeddedLanguageFormatting {
  Auto,
  Off,
}

generate_str_to_from![EmbeddedLanguageFormatting, [Auto, "auto"], [Off, "off"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProseWrap {
  Always,
  Never,
  Preserve,
}

generate_str_to_from![ProseWrap, [Always, "always"], [Never, "never"], [Preserve, "preserve"]];

#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HtmlWhitespaceSensitivity {
  Css,
  Strict,
  Ignore,
}

generate_str_to_from![
  HtmlWhitespaceSensitivity,
  [Css, "css"],
  [Strict, "strict"],
  [Ignore, "ignore"]
];
