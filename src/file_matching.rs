//! The files that dprint hands to the plugin. Whether and how a file is
//! formatted is up to Bun, which goes by the name of the file as Prettier does.

use crate::configuration::Configuration;
use crate::configuration::Flavor;

/// The file extensions the plugin formats with the provided configuration.
pub fn file_extensions(config: &Configuration) -> Vec<String> {
  [
    JS_EXTENSIONS,
    JSON_EXTENSIONS,
    CSS_EXTENSIONS,
    YAML_EXTENSIONS,
    MARKDOWN_EXTENSIONS,
    GRAPHQL_EXTENSIONS,
    HANDLEBARS_EXTENSIONS,
    HTML_EXTENSIONS,
    oxfmt_only(config, TOML_EXTENSIONS),
  ]
  .into_iter()
  .flatten()
  .map(|ext| ext.to_string())
  .collect()
}

/// The names of the files the plugin formats regardless of their extension
/// with the provided configuration.
pub fn file_names(config: &Configuration) -> Vec<String> {
  [FILE_NAMES, oxfmt_only(config, TOML_FILE_NAMES)]
    .into_iter()
    .flatten()
    .map(|name| name.to_string())
    .collect()
}

/// Bun only formats TOML the way oxfmt does, which Prettier has no way for.
fn oxfmt_only(config: &Configuration, items: &'static [&'static str]) -> &'static [&'static str] {
  if config.flavor == Some(Flavor::Oxfmt) {
    items
  } else {
    &[]
  }
}

const JS_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"];
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc", "json5"];
const CSS_EXTENSIONS: &[&str] = &["css", "scss", "less"];
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];
const GRAPHQL_EXTENSIONS: &[&str] = &["graphql", "gql", "graphqls"];
const HANDLEBARS_EXTENSIONS: &[&str] = &["hbs", "handlebars"];
const HTML_EXTENSIONS: &[&str] = &["html", "htm", "vue", "mjml"];
const TOML_EXTENSIONS: &[&str] = &["toml"];
const TOML_FILE_NAMES: &[&str] = &["Pipfile"];

/// Configuration files without an extension that are JSON or YAML.
const FILE_NAMES: &[&str] = &[".babelrc", ".jshintrc", ".swcrc", ".prettierrc", ".lintstagedrc"];
