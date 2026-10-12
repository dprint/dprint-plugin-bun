use dprint_core::configuration::ParseConfigurationError;
use dprint_core::generate_str_to_from;
use serde::{Deserialize, Serialize};

/// The options of `bun format`, which are the ones of a configuration file of
/// Prettier, or of oxfmt with that flavor, with the same names and values. An
/// option that is not set has the default of `bun format`.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
  pub flavor: Option<Flavor>,
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
  /// The options that Bun reads as they are written: the ones that are about
  /// the order of imports and the ones of oxfmt.
  #[serde(flatten)]
  pub other: serde_json::Map<String, serde_json::Value>,
  /// The languages that were turned off with their `<language>.enabled` option.
  #[serde(default)]
  pub disabled_languages: Vec<Language>,
}

impl Configuration {
  /// Gets if the files of the language are formatted, which is the case
  /// unless the language was turned off with its `<language>.enabled` option.
  pub fn is_language_enabled(&self, language: Language) -> bool {
    !self.disabled_languages.contains(&language)
  }
}

/// A language that can be turned off with its `<language>.enabled` option.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
  JavaScript,
  TypeScript,
  Json,
  Jsonc,
  Json5,
  Css,
  Scss,
  Less,
  Yaml,
  Markdown,
  Mdx,
  Graphql,
  Handlebars,
  Html,
  Vue,
  Mjml,
  Toml,
}

impl Language {
  pub const ALL: [Language; 17] = [
    Language::JavaScript,
    Language::TypeScript,
    Language::Json,
    Language::Jsonc,
    Language::Json5,
    Language::Css,
    Language::Scss,
    Language::Less,
    Language::Yaml,
    Language::Markdown,
    Language::Mdx,
    Language::Graphql,
    Language::Handlebars,
    Language::Html,
    Language::Vue,
    Language::Mjml,
    Language::Toml,
  ];

  /// The name of the option that decides if the language's files are formatted.
  pub fn enabled_option_name(&self) -> &'static str {
    match self {
      Language::JavaScript => "javascript.enabled",
      Language::TypeScript => "typescript.enabled",
      Language::Json => "json.enabled",
      Language::Jsonc => "jsonc.enabled",
      Language::Json5 => "json5.enabled",
      Language::Css => "css.enabled",
      Language::Scss => "scss.enabled",
      Language::Less => "less.enabled",
      Language::Yaml => "yaml.enabled",
      Language::Markdown => "markdown.enabled",
      Language::Mdx => "mdx.enabled",
      Language::Graphql => "graphql.enabled",
      Language::Handlebars => "handlebars.enabled",
      Language::Html => "html.enabled",
      Language::Vue => "vue.enabled",
      Language::Mjml => "mjml.enabled",
      Language::Toml => "toml.enabled",
    }
  }
}

/// Whose output to produce where the two differ, and whose options there are.
#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Flavor {
  Prettier,
  Oxfmt,
}

generate_str_to_from![Flavor, [Prettier, "prettier"], [Oxfmt, "oxfmt"]];

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
