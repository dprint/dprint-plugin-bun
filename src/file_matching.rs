//! The files that dprint hands to the plugin. Whether and how a file is
//! formatted is up to Bun, which goes by the name of the file as Prettier does,
//! other than the files of a language that was turned off.

use std::path::Path;

use crate::configuration::Configuration;
use crate::configuration::Language;

/// The file extensions the plugin formats with the provided configuration.
pub fn file_extensions(config: &Configuration) -> Vec<String> {
  enabled_items(config, EXTENSIONS)
}

/// The names of the files the plugin formats regardless of their extension
/// with the provided configuration.
pub fn file_names(config: &Configuration) -> Vec<String> {
  enabled_items(config, FILE_NAMES)
}

/// Gets the language of a file that the plugin matches, whose
/// `<language>.enabled` option decides if the file is formatted.
pub fn language_of(file_path: &Path) -> Option<Language> {
  // dprint matches file names and extensions case insensitively
  let file_name = file_path.file_name()?.to_str()?;
  let extension = Path::new(file_name).extension().and_then(|ext| ext.to_str());
  let find = |items: &[(Language, &[&str])], value: &str| {
    let (language, _) = items
      .iter()
      .find(|(_, items)| items.iter().any(|item| item.eq_ignore_ascii_case(value)))?;
    Some(*language)
  };
  find(FILE_NAMES, file_name).or_else(|| find(EXTENSIONS, extension?))
}

fn enabled_items(config: &Configuration, items: &[(Language, &[&str])]) -> Vec<String> {
  items
    .iter()
    .filter(|(language, _)| config.is_language_enabled(*language))
    .flat_map(|(_, items)| items.iter())
    .map(|item| item.to_string())
    .collect()
}

const EXTENSIONS: &[(Language, &[&str])] = &[
  (Language::JavaScript, &["js", "mjs", "cjs", "jsx"]),
  (Language::TypeScript, &["ts", "mts", "cts", "tsx"]),
  (Language::Json, &["json"]),
  (Language::Jsonc, &["jsonc"]),
  (Language::Json5, &["json5"]),
  (Language::Css, &["css"]),
  (Language::Scss, &["scss"]),
  (Language::Less, &["less"]),
  (Language::Yaml, &["yaml", "yml"]),
  (Language::Markdown, &["md", "markdown"]),
  (Language::Mdx, &["mdx"]),
  (Language::Graphql, &["graphql", "gql", "graphqls"]),
  (Language::Handlebars, &["hbs", "handlebars"]),
  (Language::Html, &["html", "htm"]),
  (Language::Vue, &["vue"]),
  (Language::Mjml, &["mjml"]),
  (Language::Toml, &["toml"]),
  (Language::Svelte, &["svelte"]),
];

/// Files that are matched by their name, most of which are configuration
/// files without an extension that are JSON or YAML.
const FILE_NAMES: &[(Language, &[&str])] = &[
  (Language::Json, &[".babelrc", ".jshintrc", ".swcrc"]),
  (Language::Yaml, &[".prettierrc", ".lintstagedrc"]),
  (Language::Toml, &["Pipfile"]),
];

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn gets_language_of_matched_files() {
    for (path, expected) in [
      ("dir/file.ts", Some(Language::TypeScript)),
      ("file.JSX", Some(Language::JavaScript)),
      ("package.json", Some(Language::Json)),
      (".swcrc", Some(Language::Json)),
      (".PrettierRC", Some(Language::Yaml)),
      ("README.md", Some(Language::Markdown)),
      ("page.mdx", Some(Language::Mdx)),
      ("App.vue", Some(Language::Vue)),
      ("pipfile", Some(Language::Toml)),
      ("file.txt", None),
      ("Makefile", None),
    ] {
      assert_eq!(language_of(Path::new(path)), expected, "{path}");
    }
  }

  #[test]
  fn every_language_has_files() {
    for language in Language::ALL {
      assert!(EXTENSIONS.iter().any(|(l, _)| *l == language), "{language:?}");
    }
  }
}
