extern crate dprint_development;
extern crate dprint_plugin_bun;

use std::path::PathBuf;

use dprint_core::configuration::*;
use dprint_development::*;
use dprint_plugin_bun::configuration::Configuration;
use dprint_plugin_bun::configuration::resolve_config;
use dprint_plugin_bun::*;

/// The names and the texts of the files in tests/specs, which build.rs lists.
const SPEC_FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/specs.rs"));

#[test]
fn test_specs() {
  let global_config = GlobalConfiguration::default();
  let parse_options = ParseSpecOptions {
    default_file_name: "file.ts",
  };
  // `run_specs` of dprint-development runs the specs on threads, which there
  // are none of in the Wasm that the tests are built for
  assert!(!SPEC_FILES.is_empty());
  let mut failures = Vec::new();
  for (spec_file, spec_text) in SPEC_FILES {
    let specs = parse_specs(spec_text.to_string(), &parse_options);
    let has_only = specs.iter().any(|spec| spec.is_only);
    for spec in specs {
      if spec.skip || (has_only && !spec.is_only) {
        continue;
      }
      let spec_config: ConfigKeyMap = serde_json::from_value(spec.config.clone().into()).unwrap();
      let config_result = resolve_config(spec_config, &global_config);
      ensure_no_diagnostics(&config_result.diagnostics);
      let format = |text: &str| {
        format_text(&PathBuf::from(&spec.file_name), text, &config_result.config)
          .map(|formatted| formatted.unwrap_or_else(|| text.to_string()))
          .unwrap_or_else(|err| format!("{err}"))
      };

      let actual = format(&spec.file_text);
      let mut fail = |kind: &str, actual: &str| {
        failures.push(format!(
          "{} -- {} ({kind})\n{}",
          spec_file,
          spec.message,
          pretty_assertions::StrComparison::new(&spec.expected_text, actual)
        ));
      };
      if actual != spec.expected_text {
        fail("formatting", &actual);
      } else if !spec.skip_format_twice {
        let twice = format(&actual);
        if twice != actual {
          fail("formatting twice", &twice);
        }
      }
    }
  }

  assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn should_fail_on_parse_error() {
  let config = Configuration::default();
  for (file_name, text) in [
    ("file.ts", "const t string = 5;"),
    ("file.json", "{ \"a\": }"),
    ("file.css", "a { color: red"),
    ("file.graphql", "query {"),
    ("file.yaml", "a: [1, 2"),
  ] {
    let result = format_text(&PathBuf::from(file_name), text, &config);
    assert!(result.is_err(), "expected an error for {file_name}");
  }
}

#[test]
fn should_have_position_in_parse_errors() {
  let config = Configuration::default();
  let err = format_text(&PathBuf::from("file.ts"), "const a = 1;\nconst = ;\n", &config).unwrap_err();
  assert!(err.to_string().contains("(2:7)"), "{err}");
}

#[test]
fn should_not_format_unknown_files() {
  let config = Configuration::default();
  for file_name in ["file.txt", "file.rs", "Cargo.lock"] {
    let result = format_text(&PathBuf::from(file_name), "a:   1", &config).unwrap();
    assert_eq!(result, None, "{file_name}");
  }
}

#[test]
fn should_format_every_matched_file() {
  let config = Configuration::default();
  let file_names = file_extensions()
    .into_iter()
    .map(|ext| format!("file.{ext}"))
    .chain(file_names());
  for file_name in file_names {
    // text that is valid, and not formatted, in none of the languages
    let result = format_text(&PathBuf::from(&file_name), "{  }\n\n\n", &config);
    assert!(!matches!(result, Ok(None)), "{file_name}");
  }
}

#[test]
fn should_format_handlebars() {
  // like Prettier, without a line break at the end, which a spec cannot say
  let config = Configuration::default();
  let result = format_text(
    &PathBuf::from("file.hbs"),
    "<div   class=\"a\">{{#if b}}{{c}}{{/if}}</div>\n",
    &config,
  )
  .unwrap();
  assert_eq!(result.as_deref(), Some("<div class=\"a\">{{#if b}}{{c}}{{/if}}</div>"));
}

#[test]
fn should_use_global_config() {
  let global_config = GlobalConfiguration {
    line_width: Some(20),
    use_tabs: Some(true),
    indent_width: Some(8),
    new_line_kind: Some(NewLineKind::CarriageReturnLineFeed),
  };
  let config = resolve_config(ConfigKeyMap::new(), &global_config).config;
  let result = format_text(&PathBuf::from("file.ts"), "call(argument1, argument2);\n", &config).unwrap();
  assert_eq!(
    result.as_deref(),
    Some("call(\r\n\targument1,\r\n\targument2,\r\n);\r\n")
  );
}

#[test]
fn should_have_diagnostics_for_invalid_config() {
  for (config, property_name) in [
    (serde_json::json!({ "semi": "never" }), "semi"),
    (serde_json::json!({ "trailingComma": "always" }), "trailingComma"),
    (serde_json::json!({ "quoteProps": "asNeeded" }), "quoteProps"),
    (serde_json::json!({ "unknown": true }), "unknown"),
  ] {
    let diagnostics = resolve(config.clone()).diagnostics;
    assert_eq!(
      diagnostics.iter().map(|d| d.property_name.as_str()).collect::<Vec<_>>(),
      vec![property_name],
      "{config}"
    );
  }
}

fn resolve(config: serde_json::Value) -> ResolveConfigurationResult<Configuration> {
  let config: ConfigKeyMap = serde_json::from_value(config).unwrap();
  resolve_config(config, &GlobalConfiguration::default())
}
