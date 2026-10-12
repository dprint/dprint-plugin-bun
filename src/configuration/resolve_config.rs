use super::Configuration;
use super::EndOfLine;
use crate::options::build_bun_config;
use bun_lint_driver::fmt::check_config_for_plugin;
use dprint_core::configuration::*;

/// Resolves configuration from a collection of key value strings.
///
/// # Example
///
/// ```
/// use dprint_core::configuration::ConfigKeyMap;
/// use dprint_core::configuration::resolve_global_config;
/// use dprint_plugin_bun::configuration::resolve_config;
///
/// let mut config_map = ConfigKeyMap::new(); // get a collection of key value pairs from somewhere
/// let global_config_result = resolve_global_config(&mut config_map);
///
/// // check global_config_result.diagnostics here...
///
/// let config_result = resolve_config(
///     config_map,
///     &global_config_result.config
/// );
///
/// // check config_result.diagnostics here and use config_result.config
/// ```
pub fn resolve_config(
  config: ConfigKeyMap,
  global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
  let mut diagnostics = Vec::new();
  let mut config = config;

  let resolved_config = Configuration {
    flavor: get_nullable_value(&mut config, "flavor", &mut diagnostics),
    print_width: get_nullable_value(&mut config, "printWidth", &mut diagnostics).or(global_config.line_width),
    tab_width: get_nullable_value(&mut config, "tabWidth", &mut diagnostics).or(global_config.indent_width),
    use_tabs: get_nullable_value(&mut config, "useTabs", &mut diagnostics).or(global_config.use_tabs),
    end_of_line: get_nullable_value(&mut config, "endOfLine", &mut diagnostics).or(match global_config.new_line_kind {
      Some(NewLineKind::CarriageReturnLineFeed) => Some(EndOfLine::Crlf),
      Some(NewLineKind::LineFeed) => Some(EndOfLine::Lf),
      Some(NewLineKind::Auto) => Some(EndOfLine::Auto),
      _ => None,
    }),
    semi: get_nullable_value(&mut config, "semi", &mut diagnostics),
    single_quote: get_nullable_value(&mut config, "singleQuote", &mut diagnostics),
    jsx_single_quote: get_nullable_value(&mut config, "jsxSingleQuote", &mut diagnostics),
    quote_props: get_nullable_value(&mut config, "quoteProps", &mut diagnostics),
    trailing_comma: get_nullable_value(&mut config, "trailingComma", &mut diagnostics),
    bracket_spacing: get_nullable_value(&mut config, "bracketSpacing", &mut diagnostics),
    bracket_same_line: get_nullable_value(&mut config, "bracketSameLine", &mut diagnostics),
    arrow_parens: get_nullable_value(&mut config, "arrowParens", &mut diagnostics),
    object_wrap: get_nullable_value(&mut config, "objectWrap", &mut diagnostics),
    single_attribute_per_line: get_nullable_value(&mut config, "singleAttributePerLine", &mut diagnostics),
    experimental_operator_position: get_nullable_value(&mut config, "experimentalOperatorPosition", &mut diagnostics),
    experimental_ternaries: get_nullable_value(&mut config, "experimentalTernaries", &mut diagnostics),
    embedded_language_formatting: get_nullable_value(&mut config, "embeddedLanguageFormatting", &mut diagnostics),
    prose_wrap: get_nullable_value(&mut config, "proseWrap", &mut diagnostics),
    html_whitespace_sensitivity: get_nullable_value(&mut config, "htmlWhitespaceSensitivity", &mut diagnostics),
    vue_indent_script_and_style: get_nullable_value(&mut config, "vueIndentScriptAndStyle", &mut diagnostics),
    insert_pragma: get_nullable_value(&mut config, "insertPragma", &mut diagnostics),
    require_pragma: get_nullable_value(&mut config, "requirePragma", &mut diagnostics),
    check_ignore_pragma: get_nullable_value(&mut config, "checkIgnorePragma", &mut diagnostics),
    other: take_other_options(&mut config),
  };

  diagnostics.extend(get_unknown_property_diagnostics(config));

  // Bun says what is wrong with the options that it reads as they are written
  if diagnostics.is_empty() {
    let bun_config = build_bun_config(&resolved_config);
    if let Err(message) = check_config_for_plugin(bun_config.json.as_bytes(), bun_config.is_oxfmt) {
      diagnostics.push(ConfigurationDiagnostic {
        property_name: property_name_in(&message, &resolved_config).to_string(),
        message: String::from_utf8_lossy(&message).into_owned(),
      });
    }
  }

  ResolveConfigurationResult {
    config: resolved_config,
    diagnostics,
  }
}

/// The options that Bun reads as they are written in a configuration file.
const OTHER_OPTIONS: &[&str] = &[
  "parser",
  // which of the plugins of Prettier that sort imports to sort like
  "plugins",
  // @trivago/prettier-plugin-sort-imports and @ianvs/prettier-plugin-sort-imports
  "importOrder",
  "importOrderParserPlugins",
  "importOrderSeparation",
  "importOrderSortSpecifiers",
  "importOrderGroupNamespaceSpecifiers",
  "importOrderCaseInsensitive",
  "importOrderSideEffects",
  "importOrderSortByLength",
  "importOrderImportAttributesKeyword",
  "importOrderExclude",
  "importOrderTypeScriptVersion",
  "importOrderCaseSensitive",
  "importOrderSafeSideEffects",
  // prettier-plugin-organize-imports, and what it reads in a tsconfig.json
  "organizeImportsSkipDestructiveCodeActions",
  "organizeImportsTypeOrder",
  "tsconfig.jsx",
  "tsconfig.jsxFactory",
  "tsconfig.jsxFragmentFactory",
  "tsconfig.reactNamespace",
  // oxfmt
  "sortImports",
  "experimentalSortImports",
  "sortPackageJson",
  "experimentalSortPackageJson",
  "jsdoc",
  "insertFinalNewline",
  "sortTailwindcss",
  "experimentalTailwindcss",
];

fn take_other_options(config: &mut ConfigKeyMap) -> serde_json::Map<String, serde_json::Value> {
  let mut options = serde_json::Map::new();
  for name in OTHER_OPTIONS {
    match config.shift_remove(*name) {
      Some(ConfigKeyValue::Null) | None => {}
      Some(value) => {
        options.insert(name.to_string(), to_json(value));
      }
    }
  }
  options
}

fn to_json(value: ConfigKeyValue) -> serde_json::Value {
  match value {
    ConfigKeyValue::String(value) => value.into(),
    ConfigKeyValue::Number(value) => value.into(),
    ConfigKeyValue::Bool(value) => value.into(),
    ConfigKeyValue::Array(values) => values.into_iter().map(to_json).collect(),
    ConfigKeyValue::Object(values) => {
      serde_json::Value::Object(values.into_iter().map(|(name, value)| (name, to_json(value))).collect())
    }
    ConfigKeyValue::Null => serde_json::Value::Null,
  }
}

/// The option that a message of Bun is about, which names it.
fn property_name_in<'a>(message: &[u8], config: &'a Configuration) -> &'a str {
  let message = String::from_utf8_lossy(message);
  let mut names = config.other.keys().map(String::as_str).collect::<Vec<_>>();
  // the longest first: `importOrder` is the start of the names of other options
  names.sort_by_key(|name| std::cmp::Reverse(name.len()));
  names
    .into_iter()
    .find(|name| message.contains(name))
    .unwrap_or("plugins")
}
