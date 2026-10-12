//! Bun turns numbers into text and back with WebKit's WTF, which is written in
//! C++. These are the functions of it that the formatter's crates call.

use std::io::Write;
use std::slice;

/// `WTF::numberToString`: writes the number as JavaScript's `String(number)`
/// does, and returns the length.
#[unsafe(no_mangle)]
pub extern "C" fn WTF__dtoa(buf: &mut [u8; 124], number: f64) -> usize {
  let text = number_to_string(number);
  buf[..text.len()].copy_from_slice(text.as_bytes());
  text.len()
}

/// `WTF::parseDouble`: the number that the text starts with, and in `counted`
/// how many bytes of the text it is. Zero bytes if it starts with none.
///
/// # Safety
/// `bytes` has to be valid for reads of `length` bytes, and `counted` for a write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTF__parseDouble(bytes: *const u8, length: usize, counted: *mut usize) -> f64 {
  let text = match length {
    0 => &[][..],
    // SAFETY: the caller guarantees the range
    _ => unsafe { slice::from_raw_parts(bytes, length) },
  };
  let len = number_prefix_len(text);
  let number = std::str::from_utf8(&text[..len])
    .ok()
    .and_then(|text| text.parse::<f64>().ok());
  // SAFETY: the caller guarantees the pointer
  unsafe { counted.write(if number.is_some() { len } else { 0 }) };
  number.unwrap_or(0.0)
}

/// [Number::toString](https://tc39.es/ecma262/#sec-numeric-types-number-tostring)
/// with the radix 10.
fn number_to_string(number: f64) -> String {
  if number.is_nan() {
    return "NaN".to_string();
  }
  if number == 0.0 {
    return "0".to_string();
  }
  if number.is_infinite() {
    return if number < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
  }
  // Rust writes the fewest digits that give the number back, which is what
  // JavaScript asks for, as `d.ddde-x`
  let scientific = format!("{:e}", number.abs());
  let (mantissa, exponent) = scientific.split_once('e').unwrap();
  let digits = mantissa.replace('.', "");
  // the names are the ones of the specification
  let k = digits.len() as i32;
  let n = exponent.parse::<i32>().unwrap() + 1;

  let mut text = Vec::new();
  if number < 0.0 {
    text.push(b'-');
  }
  if k <= n && n <= 21 {
    text.extend_from_slice(digits.as_bytes());
    text.resize(text.len() + (n - k) as usize, b'0');
  } else if 0 < n && n <= 21 {
    let (integer, fraction) = digits.split_at(n as usize);
    write!(text, "{integer}.{fraction}").unwrap();
  } else if -6 < n && n <= 0 {
    text.extend_from_slice(b"0.");
    text.resize(text.len() + (-n) as usize, b'0');
    text.extend_from_slice(digits.as_bytes());
  } else {
    let sign = if n - 1 < 0 { '-' } else { '+' };
    let (first, rest) = digits.split_at(1);
    match rest.is_empty() {
      true => write!(text, "{first}e{sign}{}", (n - 1).abs()).unwrap(),
      false => write!(text, "{first}.{rest}e{sign}{}", (n - 1).abs()).unwrap(),
    }
  }
  String::from_utf8(text).unwrap()
}

/// How much of the start of the text is a decimal number: a sign, digits with
/// or without a point, and an exponent. Zero if there is no digit.
fn number_prefix_len(text: &[u8]) -> usize {
  let digits_from = |start: usize| text[start..].iter().take_while(|byte| byte.is_ascii_digit()).count();
  let mut index = usize::from(matches!(text.first(), Some(b'+' | b'-')));
  let integer_digits = digits_from(index);
  index += integer_digits;
  let mut fraction_digits = 0;
  if text.get(index) == Some(&b'.') {
    fraction_digits = digits_from(index + 1);
    if integer_digits + fraction_digits > 0 {
      index += 1 + fraction_digits;
    }
  }
  if integer_digits + fraction_digits == 0 {
    return 0;
  }
  if matches!(text.get(index), Some(b'e' | b'E')) {
    let sign = usize::from(matches!(text.get(index + 1), Some(b'+' | b'-')));
    let exponent_digits = digits_from((index + 1 + sign).min(text.len()));
    if exponent_digits > 0 {
      index += 1 + sign + exponent_digits;
    }
  }
  index
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn writes_numbers_like_javascript() {
    for (number, expected) in [
      (0.0, "0"),
      (-0.0, "0"),
      (1.0, "1"),
      (-1.5, "-1.5"),
      (123456789.0, "123456789"),
      (0.1, "0.1"),
      (0.000001, "0.000001"),
      (0.0000001, "1e-7"),
      (1.5e-7, "1.5e-7"),
      (1e21, "1e+21"),
      (1.2e21, "1.2e+21"),
      (123456789012345680000.0, "123456789012345680000"),
      (f64::NAN, "NaN"),
      (f64::NEG_INFINITY, "-Infinity"),
    ] {
      assert_eq!(number_to_string(number), expected);
    }
  }

  #[test]
  fn parses_the_number_at_the_start() {
    for (text, expected, expected_len) in [
      ("1", 1.0, 1),
      ("-1.5", -1.5, 4),
      ("+.5", 0.5, 3),
      ("5.", 5.0, 2),
      ("1e3", 1000.0, 3),
      ("1e", 1.0, 1),
      ("1.5x", 1.5, 3),
      ("x", 0.0, 0),
      (".", 0.0, 0),
      ("", 0.0, 0),
    ] {
      let mut len = usize::MAX;
      // SAFETY: the text and the length are live
      let number = unsafe { WTF__parseDouble(text.as_ptr(), text.len(), &mut len) };
      assert_eq!((number, len), (expected, expected_len), "{text}");
    }
  }
}
