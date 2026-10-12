//! The files that dprint hands to the plugin. Whether and how a file is
//! formatted is up to Bun, which goes by the name of the file as Prettier does.

/// The file extensions the plugin formats.
pub fn file_extensions() -> Vec<String> {
  [
    JS_EXTENSIONS,
    JSON_EXTENSIONS,
    CSS_EXTENSIONS,
    YAML_EXTENSIONS,
    MARKDOWN_EXTENSIONS,
    GRAPHQL_EXTENSIONS,
    HANDLEBARS_EXTENSIONS,
    HTML_EXTENSIONS,
  ]
  .into_iter()
  .flatten()
  .map(|ext| ext.to_string())
  .collect()
}

/// The names of the files the plugin formats regardless of their extension.
pub fn file_names() -> Vec<String> {
  FILE_NAMES.iter().map(|name| name.to_string()).collect()
}

const JS_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"];
const JSON_EXTENSIONS: &[&str] = &["json", "jsonc", "json5"];
const CSS_EXTENSIONS: &[&str] = &["css", "scss", "less"];
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdx"];
const GRAPHQL_EXTENSIONS: &[&str] = &["graphql", "gql", "graphqls"];
const HANDLEBARS_EXTENSIONS: &[&str] = &["hbs", "handlebars"];
const HTML_EXTENSIONS: &[&str] = &["html", "htm", "vue", "mjml"];

/// Configuration files without an extension that are JSON or YAML.
const FILE_NAMES: &[&str] = &[".babelrc", ".jshintrc", ".swcrc", ".prettierrc", ".lintstagedrc"];
