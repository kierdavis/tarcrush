use crate::Frame;
use crossbeam::channel::Sender;
use memmap::MmapOptions;
use std::fmt;
use std::fs::File;
use std::io::Seek;
use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd};
use std::path::Path;

mod map;

pub fn from_stdin<T, E>(
  callback: impl for<'a> FnOnce(&'a mut (dyn Strategy + Send)) -> Result<T, E>,
) -> Result<T, E>
where
  E: From<std::io::Error>,
{
  from_fd(std::io::stdin().lock().as_fd(), callback)
}

pub fn from_path<T, E>(
  path: &Path,
  callback: impl for<'a> FnOnce(&'a mut (dyn Strategy + Send)) -> Result<T, E>,
) -> Result<T, E>
where
  E: From<std::io::Error>,
{
  from_file(&File::open(path)?, callback)
}

pub fn from_file<T, E>(
  file: &File,
  callback: impl for<'a> FnOnce(&'a mut (dyn Strategy + Send)) -> Result<T, E>,
) -> Result<T, E>
where
  E: From<std::io::Error>,
{
  from_fd(file.as_fd(), callback)
}

pub fn from_fd<T, E>(
  fd: BorrowedFd,
  callback: impl for<'a> FnOnce(&'a mut (dyn Strategy + Send)) -> Result<T, E>,
) -> Result<T, E>
where
  E: From<std::io::Error>,
{
  let mut file_wrapper = unsafe { ManuallyDrop::new(File::from_raw_fd(fd.as_raw_fd())) };
  match file_wrapper.stream_position() {
    Ok(skip) => {
      let mapping = unsafe { MmapOptions::new().offset(skip).map(&file_wrapper) }?;
      callback(&mut map::MapStrategy::new(mapping.as_ref()))
    },
    // TODO(rust): check instead for ErrorKind::NotSeekable once io_error_more is stabilised.
    Err(err) if err.raw_os_error() == Some(29) /* ESPIPE */ => {
      todo!("different strategy for pipes")
    },
    Err(err) => Err(err.into()),
  }
}

pub trait Strategy: fmt::Debug {
  fn scan(&mut self, frames_out: Sender<Frame>);
}
