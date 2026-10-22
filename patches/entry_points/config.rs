// What follows is appended by scripts/setup.ts of dprint-plugin-bun.

/// How to format a file with the configuration `json`, which is what a configuration file of Prettier has, or of
/// oxfmt if `is_oxfmt`. It is `Configs::options_for` without what that reads of the disk and of the command line.
/// `Err`: what is wrong with the configuration.
pub(super) fn resolved_for_plugin(json: &Json, is_oxfmt: bool) -> Result<Resolved, Vec<u8>> {
  let invalid = |name: &[u8], value: &[u8]| [b"Invalid ", name, b" value: ", value, b"."].concat();
  let all = settings(json, is_oxfmt);
  if is_oxfmt {
    check_widths(|name| all.iter().rfind(|it| it.0 == name).map(|it| &it.1[..])).map_err(|fatal| fatal.0)?;
  }
  // Tailwind and plugins that Bun does not implement need JavaScript, which is not run here.
  let plugins = Plugins::from_options(json);
  if let Some(plugins) = &plugins {
    let others = (plugins.packages.iter())
      .filter(|it| !is_built_in_plugin(it) || package_of(it) == TAILWIND_PLUGIN);
    let names = others.map(|it| &it[..]).collect::<Vec<_>>().join(&b", "[..]);
    if plugins.has_unknown_plugins || !names.is_empty() {
      return Err([b"These plugins of Prettier cannot be loaded: ", &names[..], b"."].concat());
    }
  }
  if json.get(b"overrides").is_some() {
    return Err(b"`overrides` is not supported.".to_vec());
  }
  let mut package_json = (!is_oxfmt && plugins.as_ref().is_some_and(|it| it.sorts_package_json))
    .then(PackageJsonPlugin::default);
  let mut sets_package_json_sorting = false;
  let mut resolved = Resolved::default();
  let mut sort = SortSettings::default();
  resolved.options.format_javascript = Some(super::format_block);
  resolved.options.parse_javascript = Some(super::parse_javascript);
  resolved.options.parse_parts = Some(super::parse_parts);
  resolved.options.embedded_html = true;
  resolved.options.toml = true;
  resolved.options.svelte.is_on = !is_oxfmt || svelte_in(&all).unwrap_or(true);
  if is_oxfmt {
    let _ = resolved.options.set(b"flavor", b"oxfmt");
    let _ = resolved.options.set(b"printWidth", b"100");
    // It sorts the keys of a `package.json` unless it is told not to.
    let _ = resolved.options.set(b"sortPackageJson", b"true");
  }
  for (name, value) in all.iter().map(|it| (&it.0[..], &it.1[..])) {
    match name {
      // `experimentalSortImports` is the old name.
      name if is_oxfmt && name.ends_with(b"ortImports") => {
        sort.set(b"sortImports", value);
      }
      name if sort.set(name, value) => {}
      b"insertFinalNewline" if is_oxfmt => resolved.omits_final_newline = value == b"false",
      b"sortPackageJson" | b"sortPackageJson.sortScripts" if is_oxfmt => {
        sets_package_json_sorting = true;
        let _ = resolved.options.set(name, value);
      }
      b"experimentalSortPackageJson" if is_oxfmt => {
        let _ = resolved.options.set(b"sortPackageJson", value);
      }
      b"packageSortOrder" if package_json.is_some() => {
        package_json = Some(PackageJsonPlugin {
          first: package_sort_order(value)?,
          ..Default::default()
        });
      }
      b"svelteSortOrder" | b"svelteAllowShorthand" | b"svelteIndentScriptAndStyle"
      | b"svelte.sortOrder" | b"svelte.allowShorthand" | b"svelte.indentScriptAndStyle" => {
        if resolved.options.svelte.is_on {
          resolved.options.set(name, value).map_err(|_| invalid(name, value))?;
        }
      }
      // The Svelte switch was handled before its options, regardless of their order.
      b"svelte" if is_oxfmt => {},
      name if name == b"jsdoc" || name.starts_with(b"jsdoc.") => {
        resolved.options.set(name, value).map_err(|_| invalid(name, value))?;
      }
      // The order of the classes is known to the Tailwind of the project alone, which is JavaScript.
      name if is_tailwind(name) => {
        if value != b"false" {
          return Err(
            [
              name,
              b" is not supported: it needs the tailwindcss package of the project.",
            ]
            .concat(),
          );
        }
      }
      name if OPTIONS.contains(&name) => {
        resolved
          .options
          .set(name, value)
          .map_err(|_| match not_an_integer(name, value, false) {
            Some(refusal) if !is_oxfmt => refusal,
            _ => invalid(name, value),
          })?;
      }
      name if OTHER_KEYS.contains(&name) => {}
      name => return Err([b"Unknown option: ", name, b"."].concat()),
    }
  }
  // Prettier does not know the options of a plugin that is not named.
  if let Some((name, _)) = sort.options_of_unlisted_plugins().next() {
    return Err(
      [
        name,
        b" has no effect unless `plugins` names the plugin that it is an option of.",
      ]
      .concat(),
    );
  }
  resolved.options.sort_imports = sort.compile()?;
  if let Some(plugin) = package_json.filter(|_| !sets_package_json_sorting) {
    resolved.options.sort_package_json = Some(SortPackageJson {
      plugin: Some(Arc::new(plugin)),
      ..Default::default()
    });
  }
  Ok(resolved)
}
