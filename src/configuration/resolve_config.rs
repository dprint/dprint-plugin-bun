use super::Configuration;
use super::EndOfLine;
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
  };

  diagnostics.extend(get_unknown_property_diagnostics(config));

  ResolveConfigurationResult {
    config: resolved_config,
    diagnostics,
  }
}
