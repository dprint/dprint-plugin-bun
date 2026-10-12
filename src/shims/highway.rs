//! Bun searches text with functions that are written in C++ on top of Google's
//! Highway. These are the ones that the formatter's crates call.
//!
//! What a function returns when it finds nothing is what Bun's `bun_highway`
//! expects of it, which is not the same for all of them.

use std::slice;

/// The index of the first `needle`, or the length.
///
/// # Safety
/// `haystack` has to be valid for reads of `haystack_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_index_of_char(haystack: *const u8, haystack_len: usize, needle: u8) -> usize {
  // SAFETY: the caller guarantees the range
  let haystack = unsafe { bytes(haystack, haystack_len) };
  memchr::memchr(needle, haystack).unwrap_or(haystack_len)
}

/// The index of the last `needle`, or the length.
///
/// # Safety
/// `haystack` has to be valid for reads of `haystack_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_last_index_of_char(haystack: *const u8, haystack_len: usize, needle: u8) -> usize {
  // SAFETY: the caller guarantees the range
  let haystack = unsafe { bytes(haystack, haystack_len) };
  memchr::memrchr(needle, haystack).unwrap_or(haystack_len)
}

/// The index of the first byte that is not `value`, or the length.
///
/// # Safety
/// `haystack` has to be valid for reads of `haystack_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_index_of_not_char(haystack: *const u8, haystack_len: usize, value: u8) -> usize {
  // SAFETY: the caller guarantees the range
  let haystack = unsafe { bytes(haystack, haystack_len) };
  haystack.iter().position(|&byte| byte != value).unwrap_or(haystack_len)
}

/// How many times `needle` is in the text.
///
/// # Safety
/// `haystack` has to be valid for reads of `haystack_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_count_char(haystack: *const u8, haystack_len: usize, needle: u8) -> usize {
  // SAFETY: the caller guarantees the range
  let haystack = unsafe { bytes(haystack, haystack_len) };
  memchr::memchr_iter(needle, haystack).count()
}

/// Where the first `needle` starts, or null.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_memmem(
  haystack: *const u8,
  haystack_len: usize,
  needle: *const u8,
  needle_len: usize,
) -> *const u8 {
  // SAFETY: the caller guarantees the ranges
  let (text, needle) = unsafe { (bytes(haystack, haystack_len), bytes(needle, needle_len)) };
  match memchr::memmem::find(text, needle) {
    // SAFETY: the index is in the haystack
    Some(index) => unsafe { haystack.add(index) },
    None => std::ptr::null(),
  }
}

/// The index at which the last `needle` starts, or `usize::MAX`.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_memrmem(
  haystack: *const u8,
  haystack_len: usize,
  needle: *const u8,
  needle_len: usize,
) -> usize {
  // SAFETY: the caller guarantees the ranges
  let (text, needle) = unsafe { (bytes(haystack, haystack_len), bytes(needle, needle_len)) };
  memchr::memmem::rfind(text, needle).unwrap_or(usize::MAX)
}

/// The index at which the first `needle` starts in UTF-16, or `usize::MAX`.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_memmem16(
  haystack: *const u16,
  haystack_len: usize,
  needle: *const u16,
  needle_len: usize,
) -> usize {
  // SAFETY: the caller guarantees the ranges
  let (text, needle) = unsafe { (units(haystack, haystack_len), units(needle, needle_len)) };
  if needle.is_empty() {
    return 0;
  }
  text
    .windows(needle.len())
    .position(|window| window == needle)
    .unwrap_or(usize::MAX)
}

/// The index at which the last `needle` starts in UTF-16, or `usize::MAX`.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_memrmem16(
  haystack: *const u16,
  haystack_len: usize,
  needle: *const u16,
  needle_len: usize,
) -> usize {
  // SAFETY: the caller guarantees the ranges
  let (text, needle) = unsafe { (units(haystack, haystack_len), units(needle, needle_len)) };
  if needle.is_empty() {
    return text.len();
  }
  text
    .windows(needle.len())
    .rposition(|window| window == needle)
    .unwrap_or(usize::MAX)
}

/// The index of the first byte of a string literal that is not just a part of
/// it, or the length: the quote, a backslash, and what is not printable ASCII.
///
/// # Safety
/// `text` has to be valid for reads of `text_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_index_of_interesting_character_in_string_literal(
  text: *const u8,
  text_len: usize,
  quote: u8,
) -> usize {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(text, text_len) };
  text
    .iter()
    .position(|&byte| byte == quote || byte == b'\\' || !(0x20..=0x7E).contains(&byte))
    .unwrap_or(text_len)
}

/// The index of the first byte that cannot be written as it is in a string
/// literal of JavaScript with the quote `quote_char`, or the length.
///
/// # Safety
/// `text` has to be valid for reads of `text_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_index_of_needs_escape_for_javascript_string(
  text: *const u8,
  text_len: usize,
  quote_char: u8,
) -> usize {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(text, text_len) };
  let is_backtick = quote_char == b'`';
  text
    .iter()
    .position(|&byte| {
      !(0x20..127).contains(&byte) || byte == b'\\' || byte == quote_char || (is_backtick && byte == b'$')
    })
    .unwrap_or(text_len)
}

/// The index of the first byte that is one of `chars`, or the length.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_index_of_any_char(
  text: *const u8,
  text_len: usize,
  chars: *const u8,
  chars_len: usize,
) -> usize {
  // SAFETY: the caller guarantees the ranges
  let (text, chars) = unsafe { (bytes(text, text_len), bytes(chars, chars_len)) };
  text.iter().position(|byte| chars.contains(byte)).unwrap_or(text_len)
}

/// The index of the last byte that is one of `chars`, or the length.
///
/// # Safety
/// Both ranges have to be valid for reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_last_index_of_any_char(
  text: *const u8,
  text_len: usize,
  chars: *const u8,
  chars_len: usize,
) -> usize {
  // SAFETY: the caller guarantees the ranges
  let (text, chars) = unsafe { (bytes(text, text_len), bytes(chars, chars_len)) };
  text.iter().rposition(|byte| chars.contains(byte)).unwrap_or(text_len)
}

/// # Safety
/// `pointer` has to be valid for reads of `len` bytes, or `len` zero.
unsafe fn bytes<'a>(pointer: *const u8, len: usize) -> &'a [u8] {
  // SAFETY: the caller guarantees the range
  unsafe { units(pointer, len) }
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
