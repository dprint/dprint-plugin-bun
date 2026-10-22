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

/// offsets after line feeds.
///
/// # Safety
/// `p` is readable for `len` bytes; `out` has room for one offset per line feed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_line_starts(p: *const u8, len: usize, out: *mut u32) -> usize {
  // safety: the caller passes a slice, as a pointer and a length.
  let text = unsafe { bytes(p, len) };
  let mut written = 0;
  for (i, _) in text.iter().enumerate().filter(|it| *it.1 == b'\n') {
    // safety: `out` has room for one number per line feed in the text.
    unsafe { out.add(written).write(i as u32 + 1) };
    written += 1;
  }
  written
}
/// one bit per 64-byte block containing a control, non-ASCII byte or backslash.
///
/// # Safety
/// `p` is readable for `len` bytes; `out` has room for `len.div_ceil(4096)` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_odd_blocks(p: *const u8, len: usize, out: *mut u64) {
  // safety: the caller passes a slice, as a pointer and a length.
  let text = unsafe { bytes(p, len) };
  for (word, blocks) in text.chunks(64 * 64).enumerate() {
    let is_odd = |c: &u8| !matches!(c, 0x20..=0x5B | 0x5D..=0x7E | b'\t' | b'\n' | b'\r');
    let marks = blocks.chunks(64).enumerate();
    let marks = marks.fold(0u64, |marks, (i, block)| {
      marks | u64::from(block.iter().any(is_odd)) << i
    });
    // safety: `out` has one word per 64 blocks of 64 bytes, counting partial ones.
    unsafe { out.add(word).write(marks) };
  }
}
/// bitmaps of bytes ending identifier, blank and string runs.
///
/// # Safety
/// `p` is readable for `len` bytes; `out` holds three planes `plane_len` bytes apart,
/// each with room for `len.div_ceil(64) * 8` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn highway_scanner_run_stops(p: *const u8, len: usize, out: *mut u8, plane_len: usize) {
  // safety: the caller passes a slice, as a pointer and a length.
  let text = unsafe { bytes(p, len) };
  // safety: `out` has three bitmaps, `plane_len` bytes apart, each with 8 bytes per 64 bytes of text.
  let planes = unsafe { core::slice::from_raw_parts_mut(out, 2 * plane_len + len.div_ceil(64) * 8) };
  for (block, chunk) in text.chunks(64).enumerate() {
    let mut stops = [u64::MAX; 3];
    for (i, &c) in chunk.iter().enumerate() {
      let is_name = c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
      let is_blank = c == b'\t' || c == b' ';
      let is_plain = (0x20..0x80).contains(&c) && !matches!(c, b'"' | b'\'' | b'`' | b'$' | b'\\');
      for (plane, is_in_run) in stops.iter_mut().zip([is_name, is_blank, is_plain]) {
        *plane &= !(u64::from(is_in_run) << i);
      }
    }
    for (plane, stops) in stops.iter().enumerate() {
      let at = plane * plane_len + block * 8;
      planes[at..at + 8].copy_from_slice(&stops.to_le_bytes());
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn finds_line_starts_across_blocks() {
    let mut text = [b'a'; 130];
    for i in [0, 63, 64, 129] {
      text[i] = b'\n';
    }
    let mut out = [0; 4];
    let count = unsafe { highway_line_starts(text.as_ptr(), text.len(), out.as_mut_ptr()) };
    assert_eq!(count, 4);
    assert_eq!(out, [1, 64, 65, 130]);
  }

  #[test]
  fn marks_odd_blocks_and_clears_padding() {
    let mut text = vec![b'a'; 4097];
    text[63] = b'\\';
    text[4096] = 0x80;
    let mut out = [u64::MAX; 2];
    unsafe { highway_odd_blocks(text.as_ptr(), text.len(), out.as_mut_ptr()) };
    assert_eq!(out, [1, 1]);
    for byte in 0..=255 {
      let mut out = [0];
      unsafe { highway_odd_blocks(&byte, 1, out.as_mut_ptr()) };
      let plain = matches!(byte, 0x20..=0x5B | 0x5D..=0x7E | b'\t' | b'\n' | b'\r');
      assert_eq!(out[0], u64::from(!plain), "{byte}");
    }
  }

  #[test]
  fn scanner_stops_cover_all_bytes_and_padding() {
    let text = (0..=255).chain(*b"a").collect::<Vec<u8>>();
    let plane_len = text.len().div_ceil(64) * 8;
    let mut out = vec![0; plane_len * 3];
    unsafe { highway_scanner_run_stops(text.as_ptr(), text.len(), out.as_mut_ptr(), plane_len) };
    for (i, &byte) in text.iter().enumerate() {
      let expected = [
        !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')),
        !matches!(byte, b' ' | b'\t'),
        !(0x20..0x80).contains(&byte) || matches!(byte, b'"' | b'\'' | b'`' | b'$' | b'\\'),
      ];
      for (plane, expected) in expected.into_iter().enumerate() {
        assert_eq!(out[plane * plane_len + i / 8] >> (i % 8) & 1, u8::from(expected));
      }
    }
    for plane in 0..3 {
      assert_eq!(out[plane * plane_len + 32], if plane == 1 { 0xFF } else { 0xFE });
      assert!(
        out[plane * plane_len + 33..(plane + 1) * plane_len]
          .iter()
          .all(|&it| it == 0xFF)
      );
    }
  }
}
