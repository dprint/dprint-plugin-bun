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
  let file_names = file_extensions(&config)
    .into_iter()
    .map(|ext| format!("file.{ext}"))
    .chain(file_names(&config));
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
    (serde_json::json!({ "markdown.enabled": "no" }), "markdown.enabled"),
    (serde_json::json!({ "svelte.enabled": false }), "svelte.enabled"),
  ] {
    let diagnostics = resolve(config.clone()).diagnostics;
    assert_eq!(
      diagnostics.iter().map(|d| d.property_name.as_str()).collect::<Vec<_>>(),
      vec![property_name],
      "{config}"
    );
  }
}

#[test]
fn should_not_have_diagnostics_for_the_defaults() {
  let schema: serde_json::Value = serde_json::from_str(include_str!("../deployment/schema.json")).unwrap();
  let defaults = schema["properties"]
    .as_object()
    .unwrap()
    .iter()
    .filter(|(name, _)| *name != "$schema")
    .filter_map(|(name, property)| Some((name.clone(), property.get("default")?.clone())))
    .collect::<serde_json::Map<_, _>>();
  assert!(defaults.contains_key("importOrderSeparation"));

  // every option at once, and each one on its own
  let mut configs = vec![defaults.clone()];
  configs.extend(
    defaults
      .iter()
      .map(|(name, value)| [(name.clone(), value.clone())].into_iter().collect()),
  );
  for mut config in configs {
    for flavor in ["prettier", "oxfmt"] {
      config.insert("flavor".to_string(), flavor.into());
      let config = serde_json::Value::Object(config.clone());
      let result = resolve(config.clone());
      assert!(
        result.diagnostics.is_empty(),
        "{config}: {}",
        result.diagnostics[0].message
      );
      let text = format_text(&PathBuf::from("file.ts"), "const   a = 1", &result.config).unwrap();
      assert_eq!(
        text.as_deref(),
        Some(
          "const a = 1;
"
        ),
        "{config}"
      );
    }
  }
}

#[test]
fn should_have_diagnostics_for_what_bun_refuses() {
  for (config, property_name, message) in [
    (
      serde_json::json!({ "plugins": ["prettier-plugin-svelte"] }),
      "plugins",
      "These plugins of Prettier cannot be loaded: prettier-plugin-svelte.",
    ),
    (
      serde_json::json!({ "importOrder": ["^[./]"] }),
      "importOrder",
      "importOrder has no effect unless `plugins` names the plugin that it is an option of.",
    ),
    (
      serde_json::json!({ "plugins": ["@trivago/prettier-plugin-sort-imports"], "importOrder": ["("] }),
      "importOrder",
      "Invalid regular expression in importOrder: Invalid regular expression: /(/: Unterminated group",
    ),
    (
      serde_json::json!({ "flavor": "oxfmt", "sortTailwindcss": true }),
      "sortTailwindcss",
      "sortTailwindcss is not supported: it needs the tailwindcss package of the project.",
    ),
    (
      serde_json::json!({ "flavor": "oxfmt", "jsdoc": { "lineWrappingStyle": "wide" } }),
      "jsdoc",
      "Invalid jsdoc.lineWrappingStyle value: wide.",
    ),
  ] {
    let diagnostics = resolve(config.clone()).diagnostics;
    assert_eq!(
      diagnostics
        .iter()
        .map(|d| (d.property_name.as_str(), d.message.as_str()))
        .collect::<Vec<_>>(),
      vec![(property_name, message)],
      "{config}"
    );
  }
}

#[test]
fn should_format_a_range() {
  let config = Configuration::default();
  let text = "const   a = 1;\nconst   b = 2;\nconst   c = 3;\n";
  let start = text.find("const   b").unwrap();
  let result = format_range(
    &PathBuf::from("file.ts"),
    text,
    Some(start..start + "const   b = 2;".len()),
    &config,
  )
  .unwrap();
  assert_eq!(
    result.as_deref(),
    Some("const   a = 1;\nconst b = 2;\nconst   c = 3;\n")
  );
}

#[test]
fn should_format_a_range_after_characters_of_more_than_one_byte() {
  let config = Configuration::default();
  let text = "const   a = \"é𝒳\";\nconst   b = 2;\n";
  let start = text.find("const   b").unwrap();
  let result = format_range(&PathBuf::from("file.ts"), text, Some(start..text.len()), &config).unwrap();
  assert_eq!(result.as_deref(), Some("const   a = \"é𝒳\";\nconst b = 2;\n"));
}

#[test]
fn should_omit_the_final_newline_for_oxfmt() {
  let config = resolve(serde_json::json!({ "flavor": "oxfmt", "insertFinalNewline": false })).config;
  let result = format_text(&PathBuf::from("file.ts"), "const   a = 1\n", &config).unwrap();
  assert_eq!(result.as_deref(), Some("const a = 1;"));
}

#[test]
fn should_format_toml_for_every_flavor() {
  for config in [
    serde_json::json!({}),
    serde_json::json!({ "flavor": "prettier" }),
    serde_json::json!({ "flavor": "oxfmt" }),
    // options that oxfmt does not have, which TOML is not formatted with
    serde_json::json!({ "parser": "babel", "experimentalTernaries": true, "endOfLine": "auto" }),
  ] {
    let config = resolve(config).config;
    assert!(file_extensions(&config).contains(&"toml".to_string()));
    assert!(file_names(&config).contains(&"Pipfile".to_string()));
    for file_name in ["file.toml", "Pipfile"] {
      assert_eq!(
        format_text(&PathBuf::from(file_name), "a=1\n", &config)
          .unwrap()
          .as_deref(),
        Some("a = 1\n"),
        "{file_name}"
      );
    }
  }
}

#[test]
fn should_format_toml_with_the_options_of_prettier() {
  let path = PathBuf::from("file.toml");
  let text = "a = [\"aaaaaaaaaaaaaaaaaaaaaaaaaaaa\", \"bbbbbbbbbbbbbbbbbbbbbbbbbbbb\", \"cccccccccccccccccccccccc\"]\n";
  // the text is 94 wide: lines are 80 wide by default, and not the 100 of oxfmt
  let config = Configuration::default();
  assert_eq!(
    format_text(&path, text, &config).unwrap().as_deref(),
    Some(
      "a = [\n  \"aaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n  \"bbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\n  \"cccccccccccccccccccccccc\",\n]\n"
    )
  );
  let config = resolve(serde_json::json!({ "flavor": "oxfmt" })).config;
  assert_eq!(format_text(&path, text, &config).unwrap(), None);

  let config = resolve(serde_json::json!({ "printWidth": 100 })).config;
  assert_eq!(format_text(&path, text, &config).unwrap(), None);

  let config = resolve(serde_json::json!({ "useTabs": true, "trailingComma": "none", "endOfLine": "crlf" })).config;
  assert_eq!(
    format_text(&path, text, &config).unwrap().as_deref(),
    Some(
      "a = [\r\n\t\"aaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\r\n\t\"bbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\r\n\t\"cccccccccccccccccccccccc\"\r\n]\r\n"
    )
  );
}

#[test]
fn should_not_format_disabled_languages() {
  let config = resolve(serde_json::json!({
    "markdown.enabled": false,
    "typescript.enabled": false,
    "yaml.enabled": false,
    "css.enabled": true,
    "mdx.enabled": null,
  }));
  assert!(config.diagnostics.is_empty());
  let config = config.config;
  let extensions = file_extensions(&config);
  for ext in ["md", "markdown", "ts", "tsx", "yaml", "yml"] {
    assert!(!extensions.contains(&ext.to_string()), "{ext}");
  }
  for ext in ["mdx", "js", "jsx", "css", "json"] {
    assert!(extensions.contains(&ext.to_string()), "{ext}");
  }
  let names = file_names(&config);
  assert!(!names.contains(&".prettierrc".to_string()));
  assert!(names.contains(&".babelrc".to_string()));

  for (file_name, text) in [
    ("file.md", "#   Title\n"),
    ("FILE.MD", "#   Title\n"),
    ("file.ts", "const   a = 1\n"),
    ("file.yml", "a:   1\n"),
    (".prettierrc", "a:   1\n"),
  ] {
    let result = format_text(&PathBuf::from(file_name), text, &config).unwrap();
    assert_eq!(result, None, "{file_name}");
  }
  for (file_name, text, expected) in [
    ("file.js", "const   a = 1\n", "const a = 1;\n"),
    ("file.css", "a{b:c}\n", "a {\n  b: c;\n}\n"),
    ("file.mdx", "#   Title\n", "# Title\n"),
  ] {
    let result = format_text(&PathBuf::from(file_name), text, &config).unwrap();
    assert_eq!(result.as_deref(), Some(expected), "{file_name}");
  }
}

#[test]
fn should_not_match_files_of_each_disabled_language() {
  use dprint_plugin_bun::configuration::Language;

  let all = resolve(serde_json::json!({ "flavor": "oxfmt" })).config;
  let all_extensions = file_extensions(&all);
  for language in Language::ALL {
    let config = resolve(serde_json::json!({ "flavor": "oxfmt", language.enabled_option_name(): false }));
    assert!(config.diagnostics.is_empty(), "{language:?}");
    assert!(!config.config.is_language_enabled(language));
    let extensions = file_extensions(&config.config);
    assert!(extensions.len() < all_extensions.len(), "{language:?}");
    // the files that are still matched are still formatted
    for ext in extensions {
      let file_name = format!("file.{ext}");
      let result = format_text(&PathBuf::from(&file_name), "{  }\n\n\n", &config.config);
      assert!(!matches!(result, Ok(None)), "{file_name}");
    }
  }
}

#[test]
fn should_not_format_toml_when_disabled() {
  for flavor in ["prettier", "oxfmt"] {
    let config = resolve(serde_json::json!({ "flavor": flavor, "toml.enabled": false })).config;
    assert!(!file_extensions(&config).contains(&"toml".to_string()));
    assert!(!file_names(&config).contains(&"Pipfile".to_string()));
    assert_eq!(
      format_text(&PathBuf::from("file.toml"), "a=1\n", &config).unwrap(),
      None
    );
  }
}

fn resolve(config: serde_json::Value) -> ResolveConfigurationResult<Configuration> {
  let config: ConfigKeyMap = serde_json::from_value(config).unwrap();
  resolve_config(config, &GlobalConfiguration::default())
}
