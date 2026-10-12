//! Bun matches the regular expressions of JavaScript with Yarr, the engine of
//! JavaScriptCore, which is written in C++. These are the functions that
//! Bun's `bun_yarr` calls, on top of the `regress` crate.

use std::ffi::c_void;
use std::slice;

use regress::Flags;
use regress::Match;
use regress::Regex;

/// Compiles a pattern. `characters` is Latin-1 if `is_8_bit`, otherwise UTF-16.
/// Returns null if the pattern is refused, with why in `refusal`. Otherwise
/// `offsets_size` is how many offsets `Yarr__RegularExpression__exec` writes.
///
/// # Safety
/// `characters` has to be valid for reads of `length` characters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Yarr__RegularExpression__compile(
  characters: *const c_void,
  length: u32,
  is_8_bit: bool,
  flags: u16,
  offsets_size: &mut u32,
  refusal: &mut u8,
) -> *mut c_void {
  // SAFETY: the caller guarantees the characters
  let pattern = unsafe { code_points(characters, length, is_8_bit) };
  let has_flag = |flag: u16| flags & flag != 0;
  let regex_flags = Flags {
    icase: has_flag(IGNORE_CASE),
    multiline: has_flag(MULTILINE),
    dot_all: has_flag(DOT_ALL),
    unicode: has_flag(UNICODE),
    unicode_sets: has_flag(UNICODE_SETS),
    no_opt: false,
  };
  let Ok(regex) = Regex::from_unicode(pattern.iter().copied(), regex_flags) else {
    *refusal = REFUSAL_SYNTAX;
    return std::ptr::null_mut();
  };
  let expression = RegularExpression {
    regex,
    group_count: count_groups(&pattern),
    is_sticky: has_flag(STICKY),
    reads_code_points: has_flag(UNICODE) || has_flag(UNICODE_SETS),
  };
  *offsets_size = expression.offsets_len() as u32;
  Box::into_raw(Box::new(expression)).cast()
}

/// Whether there is a match that starts at `start` or after it. After a match
/// `offsets` has the start and the end of the match and of each group, where
/// the start of a group that took no part is `u32::MAX`.
///
/// # Safety
/// `this` has to be what `Yarr__RegularExpression__compile` returned,
/// `characters` valid for reads of `length` characters, and `offsets` valid
/// for writes of the `offsets_size` of the expression.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Yarr__RegularExpression__exec(
  this: *mut c_void,
  characters: *const c_void,
  length: u32,
  is_8_bit: bool,
  start: u32,
  offsets: *mut u32,
) -> bool {
  // SAFETY: the caller guarantees the expression, the characters and the offsets
  let (expression, text, offsets) = unsafe {
    let expression = &*this.cast::<RegularExpression>();
    let text = utf16(characters, length, is_8_bit);
    (
      expression,
      text,
      slice::from_raw_parts_mut(offsets, expression.offsets_len()),
    )
  };
  let Some(found) = expression.find(&text, start as usize) else {
    return false;
  };
  offsets.fill(NO_MATCH);
  let groups = std::iter::once(Some(found.range.clone())).chain(found.captures.iter().cloned());
  for (pair, group) in offsets.as_chunks_mut::<2>().0.iter_mut().zip(groups) {
    if let Some(range) = group {
      *pair = [range.start as u32, range.end as u32];
    }
  }
  true
}

/// Frees a compiled pattern.
///
/// # Safety
/// `this` has to be what `Yarr__RegularExpression__compile` returned, and is
/// not to be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Yarr__RegularExpression__deinit(this: *mut c_void) {
  // SAFETY: the caller guarantees the expression
  drop(unsafe { Box::from_raw(this.cast::<RegularExpression>()) });
}

/// Bun starts JavaScriptCore's options with this before its first pattern.
#[unsafe(no_mangle)]
pub fn __bun_yarr_initialize() {}

// the flags of `bun_yarr::flags`
const IGNORE_CASE: u16 = 1 << 2;
const MULTILINE: u16 = 1 << 3;
const DOT_ALL: u16 = 1 << 4;
const UNICODE: u16 = 1 << 5;
const UNICODE_SETS: u16 = 1 << 6;
const STICKY: u16 = 1 << 7;

/// `Error::Syntax` of `bun_yarr`.
const REFUSAL_SYNTAX: u8 = 3;
/// `bun_yarr::NO_MATCH`
const NO_MATCH: u32 = u32::MAX;

struct RegularExpression {
  regex: Regex,
  group_count: usize,
  /// The `y` flag: a match has to start where the search does.
  is_sticky: bool,
  /// The `u` or `v` flag: a surrogate pair is one character.
  reads_code_points: bool,
}

impl RegularExpression {
  /// The match and each group have a start and an end.
  fn offsets_len(&self) -> usize {
    (self.group_count + 1) * 2
  }

  fn find(&self, text: &[u16], start: usize) -> Option<Match> {
    if start > text.len() {
      return None;
    }
    let found = match self.reads_code_points {
      true => self.regex.find_from_utf16(text, start).next(),
      false => self.regex.find_from_ucs2(text, start).next(),
    };
    found.filter(|found| !self.is_sticky || found.range.start == start)
  }
}

/// How many capturing groups the pattern has: the parentheses that are not
/// escaped, not in a class, and not followed by `?`, unless they have a name.
fn count_groups(pattern: &[u32]) -> usize {
  let is = |index: usize, c: char| pattern.get(index) == Some(&(c as u32));
  let mut count = 0;
  let mut is_in_class = false;
  let mut index = 0;
  while index < pattern.len() {
    if is(index, '\\') {
      index += 1;
    } else if is_in_class {
      is_in_class = !is(index, ']');
    } else if is(index, '[') {
      is_in_class = true;
    } else if is(index, '(') {
      let is_named = is(index + 1, '?') && is(index + 2, '<') && !is(index + 3, '=') && !is(index + 3, '!');
      if !is(index + 1, '?') || is_named {
        count += 1;
      }
    }
    index += 1;
  }
  count
}

/// The pattern as code points, where half of a surrogate pair is itself.
///
/// # Safety
/// `characters` has to be valid for reads of `length` characters.
unsafe fn code_points(characters: *const c_void, length: u32, is_8_bit: bool) -> Vec<u32> {
  // SAFETY: the caller guarantees the characters
  let units = unsafe { utf16(characters, length, is_8_bit) };
  char::decode_utf16(units.iter().copied())
    .map(|decoded| match decoded {
      Ok(c) => u32::from(c),
      Err(error) => u32::from(error.unpaired_surrogate()),
    })
    .collect()
}

/// The text as UTF-16, in which a character of Latin-1 is one unit as well, so
/// that an offset is the same in both.
///
/// # Safety
/// `characters` has to be valid for reads of `length` characters.
unsafe fn utf16(characters: *const c_void, length: u32, is_8_bit: bool) -> Vec<u16> {
  let length = length as usize;
  if length == 0 {
    return Vec::new();
  }
  // SAFETY: the caller guarantees the characters
  unsafe {
    match is_8_bit {
      true => slice::from_raw_parts(characters.cast::<u8>(), length)
        .iter()
        .map(|&byte| u16::from(byte))
        .collect(),
      false => (0..length)
        .map(|index| characters.cast::<u16>().add(index).read_unaligned())
        .collect(),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn exec(pattern: &str, flags: u16, text: &str, start: u32) -> Option<Vec<u32>> {
    let (mut offsets_size, mut refusal) = (0, 0);
    // SAFETY: the pattern and the text are live, and the offsets have the size that was asked for
    unsafe {
      let expression = Yarr__RegularExpression__compile(
        pattern.as_ptr().cast(),
        pattern.len() as u32,
        true,
        flags,
        &mut offsets_size,
        &mut refusal,
      );
      assert!(!expression.is_null(), "{pattern}");
      let mut offsets = vec![0; offsets_size as usize];
      let is_match = Yarr__RegularExpression__exec(
        expression,
        text.as_ptr().cast(),
        text.len() as u32,
        true,
        start,
        offsets.as_mut_ptr(),
      );
      Yarr__RegularExpression__deinit(expression);
      is_match.then_some(offsets)
    }
  }

  #[test]
  fn matches_with_groups() {
    assert_eq!(exec("^@/(.*)$", 0, "@/db", 0), Some(vec![0, 4, 2, 4]));
    assert_eq!(exec("(a)|(b)", 0, "xb", 0), Some(vec![1, 2, NO_MATCH, NO_MATCH, 1, 2]));
    assert_eq!(exec("(?:a)(?<name>b)[(]\\(", 0, "ab((", 0), Some(vec![0, 4, 1, 2]));
    assert_eq!(exec("^[./]", 0, "react", 0), None);
    assert_eq!(exec("A", IGNORE_CASE, "xa", 0), Some(vec![1, 2]));
  }

  #[test]
  fn matches_from_the_start_only_when_sticky() {
    assert_eq!(exec("b", 0, "ab", 0), Some(vec![1, 2]));
    assert_eq!(exec("b", STICKY, "ab", 0), None);
    assert_eq!(exec("b", STICKY, "ab", 1), Some(vec![1, 2]));
  }

  #[test]
  fn refuses_a_pattern_that_is_not_one() {
    let pattern = "(";
    let (mut offsets_size, mut refusal) = (0, 0);
    // SAFETY: the pattern is live
    let expression = unsafe {
      Yarr__RegularExpression__compile(
        pattern.as_ptr().cast(),
        pattern.len() as u32,
        true,
        0,
        &mut offsets_size,
        &mut refusal,
      )
    };
    assert!(expression.is_null());
    assert_eq!(refusal, REFUSAL_SYNTAX);
  }
}
