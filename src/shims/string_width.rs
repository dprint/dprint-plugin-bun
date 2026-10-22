//! the UTF-8 terminal width operation from Bun's `stringWidth.cpp` and
//! `ANSIHelpers.h`. This entry point sums codepoint widths, without clustering.

include!(concat!(env!("OUT_DIR"), "/string_width_table.rs"));

/// # Safety
/// `ptr` must be readable for `len` bytes, or `len` must be zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Bun__visibleWidthExcludeANSI_utf8(ptr: *const u8, len: usize) -> usize {
  let text = if len == 0 {
    &[]
  } else {
    unsafe { std::slice::from_raw_parts(ptr, len) }
  };
  visible_width(text)
}

fn visible_width(text: &[u8]) -> usize {
  let mut at = 0;
  let mut width = 0;
  while at < text.len() {
    if text[at] == 0x1B || (text[at] == 0xC2 && text.get(at + 1).is_some_and(|&it| is_introducer(it))) {
      at += consume_ansi(&text[at..]);
      continue;
    }
    let (cp, len) = decode(&text[at..]);
    let index = usize::from(STAGE1[(cp >> 8) as usize]) + (cp & 0xFF) as usize;
    let packed = STAGE3[usize::from(STAGE2[index])];
    width += match (packed >> 5) & 3 {
      3 => 1,
      it => usize::from(it),
    };
    at += len;
  }
  width
}

fn decode(text: &[u8]) -> (u32, usize) {
  let first = text[0];
  if first < 0x80 {
    return (u32::from(first), 1);
  }
  let len = match first {
    0xC0..=0xDF => 2,
    0xE0..=0xEF => 3,
    0xF0..=0xF7 => 4,
    _ => 1,
  };
  let consumed = len.min(text.len());
  if len == 1 || text.len() < len || text[1..len].iter().any(|it| it & 0xC0 != 0x80) {
    return (0xFFFD, consumed);
  }
  let mut cp = u32::from(first & (0x7F >> len));
  for &byte in &text[1..len] {
    cp = (cp << 6) | u32::from(byte & 0x3F);
  }
  let minimum = match len {
    2 => 0x80,
    3 => 0x800,
    _ => 0x10000,
  };
  (if cp < minimum || cp > 0x10FFFF { 0xFFFD } else { cp }, consumed)
}

fn is_introducer(byte: u8) -> bool {
  matches!(byte, 0x90 | 0x98 | 0x9B | 0x9D..=0x9F)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ansi {
  Start,
  Esc,
  Next,
  Csi,
  Osc,
  String,
  OscEsc,
  StringEsc,
}

fn consume_ansi(text: &[u8]) -> usize {
  let mut state = Ansi::Start;
  let mut at = 0;
  while at < text.len() {
    let byte = text[at];
    if matches!(state, Ansi::OscEsc | Ansi::StringEsc) {
      if byte == b'\\' {
        state = Ansi::Start;
        at += 1;
        continue;
      }
      state = Ansi::Esc;
    }
    state = match state {
      Ansi::Start => match byte {
        0x1B => Ansi::Esc,
        0xC2 if text.get(at + 1).is_some_and(|&it| is_introducer(it)) => {
          at += 1;
          match text[at] {
            0x9B => Ansi::Csi,
            0x9D => Ansi::Osc,
            _ => Ansi::String,
          }
        }
        _ => return at,
      },
      Ansi::Esc => match byte {
        0x1B => Ansi::Esc,
        0x18 | 0x1A => Ansi::Start,
        b'[' => Ansi::Csi,
        b']' => Ansi::Osc,
        b'P' | b'X' | b'^' | b'_' => Ansi::String,
        0x20..=0x2F => Ansi::Next,
        0x30..=0x7E => Ansi::Start,
        _ => return at,
      },
      Ansi::Next => {
        if byte == 0x1B {
          Ansi::Esc
        } else {
          while text.get(at + 1).is_some_and(|it| it & 0xC0 == 0x80) {
            at += 1;
          }
          Ansi::Start
        }
      }
      Ansi::Csi | Ansi::Osc | Ansi::String => {
        if byte == 0x1B {
          match state {
            Ansi::Osc => Ansi::OscEsc,
            Ansi::String => Ansi::StringEsc,
            _ => Ansi::Esc,
          }
        } else if byte == 0xC2 {
          at += 1;
          if at == text.len() {
            return at;
          }
          if text[at] == 0x9C { Ansi::Start } else { state }
        } else if matches!(byte, 0x18 | 0x1A)
          || (state == Ansi::Csi && (0x40..=0x7E).contains(&byte))
          || (state == Ansi::Osc && byte == 7)
        {
          Ansi::Start
        } else {
          state
        }
      }
      Ansi::OscEsc | Ansi::StringEsc => unreachable!(),
    };
    at += 1;
  }
  at
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn widths_use_buns_unicode_table_and_utf8_semantics() {
    for (text, width) in [
      ("abc", 3),
      ("a\t\n\r", 1),
      ("中Ａ", 4),
      ("e\u{301}", 1),
      ("\u{ad}\u{202e}\u{2069}", 0),
      ("\u{4dc0}\u{1fa89}", 4),
      ("👩\u{200d}💻", 4),
    ] {
      assert_eq!(visible_width(text.as_bytes()), width, "{text:?}");
    }
    assert_eq!(visible_width(&[0xED, 0xA0, 0x80]), 0);
    assert_eq!(visible_width(&[0xFF, 0xC0, b'a', b'b']), 3);
    assert_eq!(unsafe { Bun__visibleWidthExcludeANSI_utf8(std::ptr::null(), 0) }, 0);
  }

  #[test]
  fn excludes_ansi_sequences_and_handles_abort_and_truncation() {
    for (text, width) in [
      ("\x1b[31mred\x1b[0m", 3),
      ("\x1b]8;;https://example.com\x07link\x1b]8;;\x1b\\", 4),
      ("\x1bPpayload\x1b\\x", 1),
      ("\x1b(Bx", 1),
      ("\x1b7x", 1),
      ("\u{9b}31mx\u{9b}0m", 1),
      ("\u{9d}title\u{9c}x", 1),
      ("\x1b[1\x18x", 1),
      ("\x1b[1\x1b[0mx", 1),
      ("\x1b[", 0),
      ("\x1b\nx", 1),
      ("\x1b]payload", 0),
    ] {
      assert_eq!(visible_width(text.as_bytes()), width, "{text:?}");
    }
  }
}
