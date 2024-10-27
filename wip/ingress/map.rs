use crate::shingleprint::shingleprint;
use crate::ingress::Strategy;
use crate::tunables::MAX_HEAD_AND_TAIL_LEN;
use crate::tar;
use crate::Frame;
use crossbeam::channel::{self, Receiver, Sender};
use std::ops::Range;
use std::fmt;

pub struct MapStrategy<'m> {
  archive_content: &'m [u8],
}

impl<'m> MapStrategy<'m> {
  pub fn new(archive_content: &'m [u8]) -> Self {
    Self { archive_content }
  }
}

impl<'m> fmt::Debug for MapStrategy<'m> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("MapStrategy").finish_non_exhaustive()
  }
}

impl<'m> Strategy for MapStrategy<'m> {
  fn scan(&mut self, frames_out: Sender<Frame>) {
    std::thread::scope(|scope| {
      let archive_content = self.archive_content;
      let (ranges_out, ranges_in) = channel::bounded(64);
      let shingleprinting_threads: Vec<_> = (0..crate::tunables::N_SHINGLEPRINTING_THREADS)
        .map(|_| {
          let ranges_in = ranges_in.clone();
          let frames_out = frames_out.clone();
          scope.spawn(|| Self::shingleprint_frames(ranges_in, frames_out, archive_content))
        })
        .collect();
      Self::split_frames(archive_content, ranges_out);
      for thread in shingleprinting_threads {
        thread.join().expect("shingleprinting thread panicked");
      }
    })
  }
}

impl<'m> MapStrategy<'m> {
  fn split_frames(archive_content: &'m [u8], ranges_out: Sender<Range<usize>>) {
    let mut frame_offset = 0;
    let mut header_offset = 0;
    while header_offset < archive_content.len() {
      let header: &[u8] = match archive_content.get(header_offset..header_offset + 500) {
        Some(x) => x,
        None => todo!("premature EOF"),
      };
      let header: &[u8; 500] = header.try_into().unwrap();
      let header = tar::Header(header);
      if header.is_null() && frame_offset == header_offset {
        // This is likely one of the arbitrarily many zero-filled sectors at the end of the archive.
        header_offset += 512;
        frame_offset = header_offset;
        continue;
      }
      let content_len = match header.content_len() {
        Ok(x) => x,
        Err(_) => todo!("malformed tar file"),
      };
      header_offset += usize::try_from(512 + content_len.next_multiple_of(512)).unwrap();
      if !header.is_prefix() {
        ranges_out
          .send(frame_offset..header_offset)
          .expect("channel disconnected");
        frame_offset = header_offset;
      }
    }
  }
  fn shingleprint_frames(
    ranges_in: Receiver<Range<usize>>,
    frames_out: Sender<Frame>,
    archive_content: &'m [u8],
  ) {
    while let Ok(bounds) = ranges_in.recv() {
      let frame_content = &archive_content[bounds.clone()];
      let frame = if frame_content.len() <= MAX_HEAD_AND_TAIL_LEN {
        let sp = shingleprint(frame_content);
        Frame {
          bounds,
          head_sp: sp.clone(),
          tail_sp: sp,
        }
      } else {
        Frame {
          bounds,
          head_sp: shingleprint(&frame_content[..MAX_HEAD_AND_TAIL_LEN]),
          tail_sp: shingleprint(&frame_content[frame_content.len() - MAX_HEAD_AND_TAIL_LEN..]),
        }
      };
      frames_out.send(frame).expect("channel disconnected");
    }
  }
}
