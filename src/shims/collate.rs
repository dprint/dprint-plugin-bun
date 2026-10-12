//! Bun compares text the way `String.prototype.localeCompare` does with ICU's
//! collator, which is written in C++ and comes with megabytes of tables.
//!
//! This is the order of CLDR's root collation for ASCII and for the letters of
//! Latin-1: white space, punctuation, symbols, digits and then letters, where
//! the case and the accent of a letter only count between texts that are
//! otherwise the same. Any other character is ordered by its code point after
//! the letters, which is not what ICU does.

use std::cmp::Ordering;
use std::slice;

/// Less than, equal to or greater than zero. `is_numeric_and_base`: a run of
/// digits counts as the number that it is, and case and accents do not count.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Bun__collateUTF8(
  is_numeric_and_base: bool,
  a: *const u8,
  a_len: usize,
  b: *const u8,
  b_len: usize,
) -> i32 {
  // SAFETY: the caller guarantees the ranges
  let (a, b) = unsafe { (units(a, a_len), units(b, b_len)) };
  let (a, b) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
  compare(&elements(a.chars()), &elements(b.chars()), is_numeric_and_base) as i32
}

/// The same for UTF-16.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Bun__collateUTF16(
  is_numeric_and_base: bool,
  a: *const u16,
  a_len: usize,
  b: *const u16,
  b_len: usize,
) -> i32 {
  // SAFETY: the caller guarantees the ranges
  let (a, b) = unsafe { (units(a, a_len), units(b, b_len)) };
  let decode = |text: &[u16]| {
    let chars = char::decode_utf16(text.iter().copied());
    elements(chars.map(|it| it.unwrap_or(char::REPLACEMENT_CHARACTER)))
  };
  compare(&decode(a), &decode(b), is_numeric_and_base) as i32
}

/// What a character counts as.
#[derive(Clone, Copy)]
struct Element {
  /// What decides between two texts, unless they are the same in it.
  primary: u32,
  /// Whether the letter has an accent.
  has_accent: bool,
  is_upper: bool,
}

fn compare(a: &[Element], b: &[Element], is_numeric_and_base: bool) -> Ordering {
  let primary = match is_numeric_and_base {
    true => compare_with_numbers(a, b),
    false => a.iter().map(|it| it.primary).cmp(b.iter().map(|it| it.primary)),
  };
  if is_numeric_and_base {
    return primary;
  }
  primary
    .then_with(|| a.iter().map(|it| it.has_accent).cmp(b.iter().map(|it| it.has_accent)))
    .then_with(|| a.iter().map(|it| it.is_upper).cmp(b.iter().map(|it| it.is_upper)))
}

/// Compares what decides, with each run of digits as one number.
fn compare_with_numbers(mut a: &[Element], mut b: &[Element]) -> Ordering {
  loop {
    let (Some(first_a), Some(first_b)) = (a.first(), b.first()) else {
      return a.len().cmp(&b.len());
    };
    if is_digit(first_a) && is_digit(first_b) {
      let (number_a, rest_a) = split_number(a);
      let (number_b, rest_b) = split_number(b);
      // without leading zeros, the longer number is the greater one
      let order = number_a.len().cmp(&number_b.len()).then_with(|| {
        number_a
          .iter()
          .map(|it| it.primary)
          .cmp(number_b.iter().map(|it| it.primary))
      });
      if order != Ordering::Equal {
        return order;
      }
      (a, b) = (rest_a, rest_b);
    } else if first_a.primary != first_b.primary {
      return first_a.primary.cmp(&first_b.primary);
    } else {
      (a, b) = (&a[1..], &b[1..]);
    }
  }
}

/// The digits that the text starts with, without leading zeros, and what follows them.
fn split_number(text: &[Element]) -> (&[Element], &[Element]) {
  let len = text.iter().take_while(|it| is_digit(it)).count();
  let (number, rest) = text.split_at(len);
  let zeros = number.iter().take_while(|it| it.primary == DIGITS).count();
  (&number[zeros.min(len - 1)..], rest)
}

fn is_digit(element: &Element) -> bool {
  (DIGITS..DIGITS + 10).contains(&element.primary)
}

fn elements(chars: impl Iterator<Item = char>) -> Vec<Element> {
  chars.map(element).collect()
}

/// The order of the punctuation and the symbols of ASCII in the root collation.
const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
const DIGITS: u32 = 0x100;
const LETTERS: u32 = 0x200;
/// After every letter.
const OTHERS: u32 = 0x1000;

fn element(c: char) -> Element {
  let plain = |primary: u32| Element {
    primary,
    has_accent: false,
    is_upper: false,
  };
  let letter = |base: char, has_accent: bool| Element {
    // `ß` comes right after `s`, and `æ` right after `a`
    primary: LETTERS + (base.to_ascii_lowercase() as u32 - 'a' as u32) * 2,
    has_accent,
    is_upper: c.is_uppercase(),
  };
  match c {
    '\t' | '\n' | '\x0B' | '\x0C' | '\r' => plain(c as u32),
    ' ' => plain(0x1E),
    '\u{A0}' => plain(0x1F),
    '0'..='9' => plain(DIGITS + (c as u32 - '0' as u32)),
    'a'..='z' | 'A'..='Z' => letter(c, false),
    'À'..='Å' | 'à'..='å' => letter('a', true),
    'Ç' | 'ç' => letter('c', true),
    'È'..='Ë' | 'è'..='ë' => letter('e', true),
    'Ì'..='Ï' | 'ì'..='ï' => letter('i', true),
    'Ñ' | 'ñ' => letter('n', true),
    'Ò'..='Ö' | 'ò'..='ö' | 'Ø' | 'ø' => letter('o', true),
    'Ù'..='Ü' | 'ù'..='ü' => letter('u', true),
    'Ý' | 'ý' | 'ÿ' => letter('y', true),
    'Æ' | 'æ' => Element {
      primary: letter('a', false).primary + 1,
      ..letter('a', false)
    },
    'ß' => Element {
      primary: letter('s', false).primary + 1,
      ..letter('s', false)
    },
    _ => match PUNCTUATION.find(c) {
      Some(index) => plain(0x20 + index as u32),
      None => plain(OTHERS + c as u32),
    },
  }
}

/// # Safety
/// `pointer` has to be valid for reads of `len` units, or `len` zero.
unsafe fn units<'a, T>(pointer: *const T, len: usize) -> &'a [T] {
  if len == 0 {
    return &[];
  }
  // SAFETY: the caller guarantees the range
  unsafe { slice::from_raw_parts(pointer, len) }
}

#[cfg(test)]
mod test {
  use super::*;

  fn collate(a: &str, b: &str, is_numeric_and_base: bool) -> Ordering {
    // SAFETY: both are live texts
    unsafe { Bun__collateUTF8(is_numeric_and_base, a.as_ptr(), a.len(), b.as_ptr(), b.len()) }.cmp(&0)
  }

  #[test]
  fn orders_like_the_root_collation() {
    // what `["…"].sort((a, b) => a.localeCompare(b, "en"))` gives
    let sorted = [
      "", " a", "_a", "-a", "./a", "@a/b", "$a", "1", "10", "2", "a", "A", "á", "ab", "b", "B", "react", "React", "zod",
    ];
    for pair in sorted.windows(2) {
      assert_eq!(collate(pair[0], pair[1], false), Ordering::Less, "{pair:?}");
      assert_eq!(collate(pair[1], pair[0], false), Ordering::Greater, "{pair:?}");
    }
    assert_eq!(collate("a", "a", false), Ordering::Equal);
  }

  #[test]
  fn orders_numbers_and_ignores_case() {
    for (a, b, expected) in [
      ("a2", "a10", Ordering::Less),
      ("a02", "a2", Ordering::Equal),
      ("A", "a", Ordering::Equal),
      ("é", "e", Ordering::Equal),
      ("a10b", "a10a", Ordering::Greater),
      ("a", "a1", Ordering::Less),
      ("0", "00", Ordering::Equal),
    ] {
      assert_eq!(collate(a, b, true), expected, "{a} {b}");
    }
  }

  #[test]
  fn compares_utf16() {
    let (a, b) = ([0x61u16, 0x62], [0x61u16, 0x63]);
    // SAFETY: both are live texts
    let order = unsafe { Bun__collateUTF16(false, a.as_ptr(), a.len(), b.as_ptr(), b.len()) };
    assert!(order < 0);
  }
}
