//! Stands in for Bun's `bun_sys`. Formatting text needs no system call, so only the
//! parts of it that the formatter's crates name are here, and every call fails.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

pub use bun_core::{Fd, FdKind, FdNative, FileKind, Mode, Stdio, kind_from_mode};
pub use bun_errno::{E, S, SystemErrno};

use bun_core::ZStr;

pub type Maybe<T> = core::result::Result<T, Error>;
pub type Result<T> = core::result::Result<T, Error>;

/// The name of a system call.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Tag(pub u8);

impl Tag {
  pub const TODO: Tag = Tag(0);
  pub const read: Tag = Tag(1);
  pub const write: Tag = Tag(2);
  pub const open: Tag = Tag(3);
  pub const stat: Tag = Tag(4);
  pub const getcwd: Tag = Tag(5);
}

pub mod syscall {
  pub use super::Tag;
}

#[derive(Clone, Debug)]
pub struct Error {
  pub errno: u16,
  pub fd: Fd,
  pub path: Box<[u8]>,
  pub syscall: Tag,
  pub dest: Box<[u8]>,
}

impl Error {
  pub fn from_code(errno: E, syscall_tag: Tag) -> Error {
    Error {
      errno: errno as u16,
      fd: Fd::INVALID,
      path: Box::default(),
      syscall: syscall_tag,
      dest: Box::default(),
    }
  }

  pub fn with_path(&self, path: &[u8]) -> Error {
    Error {
      path: Box::from(path),
      ..self.clone()
    }
  }

  pub fn get_errno(&self) -> E {
    E::from_raw(self.errno)
  }

  pub fn get_error_code_tag_name(&self) -> Option<(&'static str, SystemErrno)> {
    let errno = SystemErrno::init(i64::from(self.errno))?;
    Some((<&'static str>::from(errno), errno))
  }

  pub fn uv_code_label(&self) -> Option<(&'static str, &'static str)> {
    let (code, _) = self.get_error_code_tag_name()?;
    Some((code, "unknown error"))
  }

  pub fn msg(&self) -> Option<&'static [u8]> {
    self.get_error_code_tag_name().map(|_| &b"unknown error"[..])
  }

  pub fn name(&self) -> &'static [u8] {
    self
      .get_error_code_tag_name()
      .map(|(name, _)| name.as_bytes())
      .unwrap_or(b"UNKNOWN")
  }
}

impl core::fmt::Display for Error {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str(core::str::from_utf8(self.name()).unwrap_or("UNKNOWN"))
  }
}

impl core::error::Error for Error {}

pub mod coreutils_error_map {
  use super::SystemErrno;

  pub fn get(_errno: SystemErrno) -> Option<&'static str> {
    None
  }
}

pub trait AsFd: Copy {
  fn as_fd(&self) -> Fd;
}

impl AsFd for Fd {
  fn as_fd(&self) -> Fd {
    *self
  }
}

impl AsFd for &Fd {
  fn as_fd(&self) -> Fd {
    **self
  }
}

pub trait FdExt: Copy + Sized {
  fn close(self);
  fn close_allowing_bad_file_descriptor(self, return_address: Option<usize>) -> Option<Error>;
}

impl FdExt for Fd {
  fn close(self) {}

  fn close_allowing_bad_file_descriptor(self, _return_address: Option<usize>) -> Option<Error> {
    None
  }
}

pub mod O {
  pub const RDONLY: i32 = 0o0;
  pub const WRONLY: i32 = 0o1;
  pub const RDWR: i32 = 0o2;
  pub const CREAT: i32 = 0o100;
  pub const EXCL: i32 = 0o200;
  pub const TRUNC: i32 = 0o1000;
  pub const APPEND: i32 = 0o2000;
  pub const DIRECTORY: i32 = 0o200000;
  pub const NOFOLLOW: i32 = 0o400000;
  pub const CLOEXEC: i32 = 0o2000000;
}

pub mod file {
  use super::AsFd;
  use super::Fd;
  use super::Maybe;
  use super::Mode;
  use super::Stat;
  use super::Tag;
  use super::unsupported;
  use bun_core::ZStr;

  pub struct File {
    pub handle: Fd,
  }

  impl File {
    pub fn from_fd(handle: Fd) -> File {
      File { handle }
    }

    pub fn handle(&self) -> Fd {
      self.handle
    }

    pub fn stdin() -> File {
      File { handle: Fd::stdin() }
    }

    pub fn openat(_dir: impl AsFd, _path: &[u8], _flags: i32, _mode: Mode) -> Maybe<File> {
      Err(unsupported(Tag::open))
    }

    pub fn make_open(_path: &[u8], _flags: i32, _mode: Mode) -> Maybe<File> {
      Err(unsupported(Tag::open))
    }

    pub fn stat(&self) -> Maybe<Stat> {
      Err(unsupported(Tag::stat))
    }

    pub fn get_end_pos(&self) -> Maybe<usize> {
      Err(unsupported(Tag::stat))
    }

    pub fn read(&self, _buf: &mut [u8]) -> Maybe<usize> {
      Err(unsupported(Tag::read))
    }

    pub fn read_all(&self, _buf: &mut [u8]) -> Maybe<usize> {
      Err(unsupported(Tag::read))
    }

    pub fn read_to_end(&self) -> Maybe<Vec<u8>> {
      Err(unsupported(Tag::read))
    }

    pub fn read_to_end_into(&self, _buf: &mut Vec<u8>) -> Maybe<usize> {
      Err(unsupported(Tag::read))
    }

    pub fn read_from(_dir: impl AsFd, _path: &[u8]) -> Maybe<Vec<u8>> {
      Err(unsupported(Tag::open))
    }

    pub fn write_all(&self, _bytes: &[u8]) -> Maybe<()> {
      Err(unsupported(Tag::write))
    }

    pub fn write_file(_dir: impl AsFd, _path: &ZStr, _data: &[u8]) -> Maybe<()> {
      Err(unsupported(Tag::write))
    }
  }
}
pub use file::File;

pub struct Dir {
  pub fd: Fd,
}

impl Dir {
  pub fn from_fd(fd: Fd) -> Dir {
    Dir { fd }
  }

  pub fn fd(&self) -> Fd {
    self.fd
  }
}

pub use bun_core::FileKind as EntryKind;

pub mod dir_iterator {
  use super::EntryKind;
  use super::Fd;
  use super::Result;

  pub struct Name(Vec<u8>);

  impl Name {
    pub fn slice_u8(&self) -> &[u8] {
      &self.0
    }
  }

  pub struct IteratorResult {
    pub name: Name,
    pub kind: EntryKind,
  }

  pub struct WrappedIterator;

  impl WrappedIterator {
    pub fn set_name_filter(&mut self, _filter: Option<&[u16]>) {}

    pub fn next(&mut self) -> Result<Option<IteratorResult>> {
      Ok(None)
    }
  }

  pub fn iterate(_dir: Fd) -> WrappedIterator {
    WrappedIterator
  }
}

pub fn iterate_dir(dir: Fd) -> dir_iterator::WrappedIterator {
  dir_iterator::iterate(dir)
}

/// The fields of `stat` that are read.
#[derive(Clone, Copy, Default)]
pub struct Stat {
  pub st_dev: u64,
  pub st_ino: u64,
  pub st_mode: u32,
  pub st_nlink: u64,
  pub st_uid: u32,
  pub st_gid: u32,
  pub st_size: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExistsAtType {
  File,
  Directory,
}

pub fn exists(_path: &[u8]) -> bool {
  false
}

pub fn exists_at_type(_dir: Fd, _sub: &ZStr) -> Maybe<ExistsAtType> {
  Err(unsupported(Tag::stat))
}

pub fn stat(_path: &ZStr) -> Maybe<Stat> {
  Err(unsupported(Tag::stat))
}

pub fn lstat(_path: &ZStr) -> Maybe<Stat> {
  Err(unsupported(Tag::stat))
}

pub fn fstatat(_fd: impl AsFd, _path: &ZStr) -> Maybe<Stat> {
  Err(unsupported(Tag::stat))
}

pub fn lstatat(_fd: impl AsFd, _path: &ZStr) -> Result<Stat> {
  Err(unsupported(Tag::stat))
}

pub fn realpath<'a>(_path: &ZStr, _buf: &'a mut bun_core::PathBuffer) -> Maybe<&'a [u8]> {
  Err(unsupported(Tag::open))
}

pub fn open(_path: &ZStr, _flags: i32, _mode: Mode) -> Maybe<Fd> {
  Err(unsupported(Tag::open))
}

pub fn openat(_dir: impl AsFd, _path: &ZStr, _flags: i32, _mode: Mode) -> Maybe<Fd> {
  Err(unsupported(Tag::open))
}

pub fn open_dir_absolute(_path: &[u8]) -> Maybe<Fd> {
  Err(unsupported(Tag::open))
}

pub fn mkdir_recursive(_sub_path: &[u8]) -> Maybe<()> {
  Err(unsupported(Tag::open))
}

pub fn symlink(_target: &ZStr, _link: &ZStr) -> Maybe<()> {
  Err(unsupported(Tag::open))
}

pub fn renameat(_from_dir: impl AsFd, _from: &ZStr, _to_dir: impl AsFd, _to: &ZStr) -> Maybe<()> {
  Err(unsupported(Tag::open))
}

pub fn rename(_from: &ZStr, _to: &ZStr) -> Maybe<()> {
  Err(unsupported(Tag::open))
}

pub fn unlinkat(_dir: impl AsFd, _path: &ZStr) -> Maybe<()> {
  Err(unsupported(Tag::open))
}

pub fn extended_attributes(_fd: Fd) -> Maybe<Vec<(Vec<u8>, Vec<u8>)>> {
  Err(unsupported(Tag::read))
}

pub fn fchmod(_fd: Fd, _mode: Mode) -> Maybe<()> {
  Err(unsupported(Tag::write))
}

pub fn getcwd(_buf: &mut [u8]) -> Maybe<usize> {
  Err(unsupported(Tag::getcwd))
}

pub fn read(_fd: Fd, _buf: &mut [u8]) -> Maybe<usize> {
  Err(unsupported(Tag::read))
}

pub fn write(_fd: Fd, _bytes: &[u8]) -> Maybe<usize> {
  Err(unsupported(Tag::write))
}

fn unsupported(tag: Tag) -> Error {
  Error::from_code(E::ENOSYS, tag)
}
