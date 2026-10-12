// What follows is appended by scripts/setup.ts of dprint-plugin-bun.

thread_local! {
    /// How to format with the configuration that was asked about last: it is the same for every file of a run.
    static FOR_PLUGIN: std::cell::RefCell<Option<(Vec<u8>, bool, std::rc::Rc<Resolved>)>> =
        const { std::cell::RefCell::new(None) };
}

/// How to format with `config`, the JSON that a configuration file of Prettier has, or of oxfmt if `is_oxfmt`.
fn resolved_for_plugin(config: &[u8], is_oxfmt: bool) -> Result<std::rc::Rc<Resolved>, Vec<u8>> {
  FOR_PLUGIN.with_borrow_mut(|last| {
    if let Some((_, _, resolved)) = (last.as_ref()).filter(|it| it.0 == config && it.1 == is_oxfmt) {
      return Ok(std::rc::Rc::clone(resolved));
    }
    let Some(json) = bun_lint::json::parse(config) else {
      return Err(b"The configuration is not JSON.".to_vec());
    };
    let resolved = std::rc::Rc::new(config::resolved_for_plugin(&json, is_oxfmt)?);
    *last = Some((config.to_vec(), is_oxfmt, std::rc::Rc::clone(&resolved)));
    Ok(resolved)
  })
}

/// What is wrong with `config`, if anything. For dprint-plugin-bun.
pub fn check_config_for_plugin(config: &[u8], is_oxfmt: bool) -> Result<(), Vec<u8>> {
  resolved_for_plugin(config, is_oxfmt).map(|_| ())
}

/// Whether the file at `path` is in a language that the formatter has. For dprint-plugin-bun.
pub fn is_formatted_for_plugin(path: &[u8], config: &[u8], is_oxfmt: bool) -> bool {
  resolved_for_plugin(config, is_oxfmt).is_ok_and(|resolved| Kind::with_options(path, &resolved.options).is_some())
}

/// The text of the file at `path` formatted with `config`, the way `bun format` does it, or else why the file is
/// left as it is. `range`: only what is between the two is formatted, which count UTF-16 code units. For
/// dprint-plugin-bun.
pub fn format_for_plugin(
  path: &[u8],
  text: &[u8],
  (config, is_oxfmt): (&[u8], bool),
  range: Option<(u32, u32)>,
) -> Result<Vec<u8>, Vec<u8>> {
  let resolved = resolved_for_plugin(config, is_oxfmt)?;
  let in_range;
  let resolved = match range {
    Some((start, end)) => {
      in_range = Resolved {
        options: FormatOptions {
          range_start: Some(start),
          range_end: Some(end),
          ..resolved.options.clone()
        },
        omits_final_newline: resolved.omits_final_newline,
        has_unsupported_option: false,
      };
      &in_range
    }
    None => &*resolved,
  };
  let names = Session::new();
  format(
    path,
    text,
    resolved,
    (&Interner::new_in(&names), &names),
    &mut Scratches::default(),
    true,
  )
  .map(|(formatted, _)| formatted)
  .map_err(|failure| match failure {
    Failure::Syntax(message) | Failure::Unread(message) => message,
    Failure::Bug(what) | Failure::Loss(what) => what.as_bytes().to_vec(),
  })
}
