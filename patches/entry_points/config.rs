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
  // A plugin of Prettier is JavaScript, which is not run here. The three that sort imports are built in.
  if let Some(plugins) = Plugins::of(json).filter(|it| it.names_others) {
    let others = (plugins.packages.iter()).filter(|it| !is_built_in_plugin(it));
    let names = others.map(|it| &it[..]).collect::<Vec<_>>().join(&b", "[..]);
    return Err([b"These plugins of Prettier cannot be loaded: ", &names[..], b"."].concat());
  }
  if json.get(b"overrides").is_some() {
    return Err(b"`overrides` is not supported.".to_vec());
  }
  let mut resolved = Resolved::default();
  let mut sort = SortSettings::default();
  resolved.options.format_javascript = Some(super::format_javascript(true));
  resolved.options.parse_javascript = Some(super::parse_javascript);
  resolved.options.embedded_html = true;
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
        let _ = resolved.options.set(name, value);
      }
      b"experimentalSortPackageJson" if is_oxfmt => {
        let _ = resolved.options.set(b"sortPackageJson", value);
      }
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
  if let Some((name, _)) = sort.of_plugins_not_named().next() {
    return Err(
      [
        name,
        b" has no effect unless `plugins` names the plugin that it is an option of.",
      ]
      .concat(),
    );
  }
  resolved.options.sort_imports = sort.compile()?;
  Ok(resolved)
}
