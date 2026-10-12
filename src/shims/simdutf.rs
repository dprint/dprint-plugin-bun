//! Bun validates and converts text with simdutf, which is written in C++. These
//! are the functions of it that the formatter's crates call.

use std::slice;

/// `simdutf::result`: how it went, and where. After an error `count` is the
/// index of it in the input. After a success it is how much was validated or
/// written.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimdutfResult {
  pub status: i32,
  pub count: usize,
}

/// `simdutf::validate_utf8`
///
/// # Safety
/// `buf` has to be valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__validate_utf8(buf: *const u8, len: usize) -> bool {
  // SAFETY: the caller guarantees the range
  std::str::from_utf8(unsafe { bytes(buf, len) }).is_ok()
}

/// `simdutf::validate_utf8_with_errors`
///
/// # Safety
/// `buf` has to be valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__validate_utf8_with_errors(buf: *const u8, len: usize) -> SimdutfResult {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(buf, len) };
  match std::str::from_utf8(text) {
    Ok(_) => SimdutfResult {
      status: SUCCESS,
      count: len,
    },
    Err(error) => SimdutfResult {
      status: utf8_error_at(text, error.valid_up_to()),
      count: error.valid_up_to(),
    },
  }
}

/// `simdutf::validate_ascii`
///
/// # Safety
/// `buf` has to be valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__validate_ascii(buf: *const u8, len: usize) -> bool {
  // SAFETY: the caller guarantees the range
  unsafe { bytes(buf, len) }.is_ascii()
}

/// `simdutf::validate_ascii_with_errors`
///
/// # Safety
/// `buf` has to be valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__validate_ascii_with_errors(buf: *const u8, len: usize) -> SimdutfResult {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(buf, len) };
  match text.iter().position(|byte| !byte.is_ascii()) {
    Some(index) => SimdutfResult {
      status: TOO_LARGE,
      count: index,
    },
    None => SimdutfResult {
      status: SUCCESS,
      count: len,
    },
  }
}

/// `simdutf::utf16_length_from_utf8`: how many code units the text has in
/// UTF-16. Like simdutf, it does not validate.
///
/// # Safety
/// `input` has to be valid for reads of `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__utf16_length_from_utf8(input: *const u8, length: usize) -> usize {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(input, length) };
  text
    .iter()
    .map(|&byte| match byte {
      // a continuation byte is a part of a character that has been counted
      0x80..=0xBF => 0,
      // what takes four bytes takes a surrogate pair
      0xF0..=0xFF => 2,
      _ => 1,
    })
    .sum()
}

/// `simdutf::convert_utf8_to_utf16le_with_errors`
///
/// # Safety
/// `buf` has to be valid for reads of `len` bytes, and `utf16_output` for
/// writes of what the text is in UTF-16.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn simdutf__convert_utf8_to_utf16le_with_errors(
  buf: *const u8,
  len: usize,
  utf16_output: *mut u16,
) -> SimdutfResult {
  // SAFETY: the caller guarantees the range
  let text = unsafe { bytes(buf, len) };
  let (valid, error) = match std::str::from_utf8(text) {
    Ok(valid) => (valid, None),
    Err(error) => {
      let index = error.valid_up_to();
      // SAFETY: it is the part that is valid
      let valid = unsafe { std::str::from_utf8_unchecked(&text[..index]) };
      (valid, Some((utf8_error_at(text, index), index)))
    }
  };
  let mut written = 0;
  for unit in valid.encode_utf16() {
    // SAFETY: the caller guarantees room for the whole text
    unsafe { utf16_output.add(written).write_unaligned(unit.to_le()) };
    written += 1;
  }
  match error {
    Some((status, index)) => SimdutfResult { status, count: index },
    None => SimdutfResult {
      status: SUCCESS,
      count: written,
    },
  }
}

// `simdutf::error_code`
const SUCCESS: i32 = 0;
const HEADER_BITS: i32 = 1;
const TOO_SHORT: i32 = 2;
const TOO_LONG: i32 = 3;
const OVERLONG: i32 = 4;
const TOO_LARGE: i32 = 5;
const SURROGATE: i32 = 6;

/// What is wrong with the character at `index`, which is where the text stops
/// being UTF-8.
fn utf8_error_at(text: &[u8], index: usize) -> i32 {
  let lead = text[index];
  let (len, min) = match lead {
    0x80..=0xBF => return TOO_LONG,
    0xC0..=0xDF => (2, 0x80),
    0xE0..=0xEF => (3, 0x800),
    0xF0..=0xF7 => (4, 0x1_0000),
    _ => return HEADER_BITS,
  };
  let rest = &text[index + 1..];
  if rest.len() < len - 1 || rest[..len - 1].iter().any(|byte| byte & 0xC0 != 0x80) {
    return TOO_SHORT;
  }
  let code_point = rest[..len - 1]
    .iter()
    .fold(u32::from(lead) & (0x7F >> len), |code_point, byte| {
      (code_point << 6) | u32::from(byte & 0x3F)
    });
  match code_point {
    _ if code_point < min => OVERLONG,
    0xD800..=0xDFFF => SURROGATE,
    _ => TOO_LARGE,
  }
}

/// # Safety
/// `pointer` has to be valid for reads of `len` bytes, or `len` zero.
unsafe fn bytes<'a>(pointer: *const u8, len: usize) -> &'a [u8] {
  if len == 0 {
    return &[];
  }
  // SAFETY: the caller guarantees the range
  unsafe { slice::from_raw_parts(pointer, len) }
}
