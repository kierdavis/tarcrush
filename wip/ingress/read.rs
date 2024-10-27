#[derive(Debug)]
pub struct ReadStrategy {
}

impl Strategy for ReadStrategy {
  fn scan(&mut self, frames_out: Sender<Frame>) {
    let memory_usage = AtomicUsize::new(0);
    std::thread::scope(|scope| {
      let (buffers_to_write_out, buffers_to_write_in) = channel::bounded(64);
      let (buffers_to_shingleprint_out, buffers_to_shingleprint_in) = channel::bounded(64);
      let (recycled_buffers_out, recycled_buffers_in) = channel::bounded(64);
      let shingleprinting_threads: Vec<_> = (0..crate::tunables::N_SHINGLEPRINTING_THREADS)
        .map(|_| {
          let ranges_in = ranges_in.clone();
          let frames_out = frames_out.clone();
          scope.spawn(|| Self::shingleprint_frames(ranges_in, frames_out, archive_content))
        })
        .collect();
      let writing_thread = scope.spawn(|| 
    })
  }
}

fn reading_thread<'sess>(
  mut src: impl Read,
  recycled_buffers_in: Receiver<Arc<Buffer<'sess>>>,
  buffers_to_write_out: Sender<Arc<Buffer<'sess>>>,
  buffers_to_shingleprint_out: Sender<FrameBuffers<'sess>>,
  memory_usage: &'sess AtomicUsize,
) -> Result<(), Error> {
  let mut frame_start = 0;
  let mut read_so_far = 0;
  'eachframe: loop {
    // Read TAR headers into the head buffer until we know the total frame length.
    let head_buf = get_buffer(&recycled_buffers_in, memory_usage);
    let head_vec = head_buf.get_mut().unwrap();
    let mut frame_end = frame_start;
    'eachheader: loop {
      let header_start = frame_end;
      let header_end = header_start + 512;
      assert!(head_vec.len() + (header_end-read_so_far) <= head_vec.capacity());
      match (&mut src).take(header_end-read_so_far).read_to_end(head_vec) {
        Ok(n) => { read_so_far += n; },
        Err(e) => return Err(Error::IngressIO(e)),
      }
      if read_so_far == frame_start { break 'eachframe; } // Clean EOF.
      if read_so_far < header_end { return Err(Error::MalformedInput(read_so_far, "premature EOF")); }
      let header: &[u8] = &head_vec[header_start-frame_start..header_end-frame_start];
      let header: &[u8; 512] = header.try_into().unwrap();
      let header = tar::Header(header);
      if header.is_null() && frame_end == frame_start {
        // This is likely one of the arbitrarily many zero-filled sectors at the end of the archive.
        head_vec.clear();
        frame_start = header_end;
        frame_end = header_end;
        continue 'eachheader;
      }
      let content_len = match header.content_len() {
        Ok(x) => x,
        Err(_) => return Err(Error::MalformedInput(header_start+124, "malformed length field")),
      };
      frame_end += usize::try_from(512 + content_len.next_multiple_of(512)).unwrap();
      if !header.is_prefix() { break }
    }
    let fb = if frame_end-frame_start <= head_vec.capacity() {
      // The head and tail overlap/abutt, and can be stored in the same buffer.
      // Read the rest of the frame into the head buffer.
      assert!(head_vec.len() + (frame_end-read_so_far) <= head_vec.capacity());
      match (&mut src).take(frame_end-read_so_far).read_to_end(head_vec) {
        Ok(n) => { read_so_far += n; },
        Err(e) => return Err(Error::IngressIO(e)),
      }
      if read_so_far < frame_end { return Err(Error::MalformedInput(read_so_far, "premature EOF")); }
      assert_eq!(head_vec.len(), frame_end-frame_start);
      if let Err(_) = buffers_to_write_out.send(head_buf.clone()) {
        return Err(Error::CompanionThreadDied);
      }
      FrameBuffers { bounds: frame_start..frame_end, head: head_buf, tail: None }
    } else {
      // The head and tail don't overlap; there's bytes to discard in between.
      // Read the read of the head and ship it off to the writing thread.
      let head_end = frame_start + MAX_HEAD_AND_TAIL_LEN;
      assert!(head_vec.len() + (head_end-read_so_far) <= head_vec.capacity());
      match (&mut src).take(head_end-read_so_far).read_to_end(head_vec) {
        Ok(n) => { read_so_far += n; },
        Err(e) => return Err(Error::IngressIO(e)),
      }
      if read_so_far < head_end { return Err(Error::MalformedInput(read_so_far, "premature EOF")); }
      assert_eq!(head_vec.len(), MAX_HEAD_AND_TAIL_LEN);
      if let Err(_) = buffers_to_write_out.send(head_buf.clone()) {
        return Err(Error::CompanionThreadDied);
      }
      // Ship intermediate bytes directly to the writing thread.
      let tail_start = frame_end - MAX_HEAD_AND_TAIL_LEN;
      while read_so_far < tail_start {
        let chunk_buf = get_buffer(&recycled_buffers_in, memory_usage);
        let chunk_vec = chunk_buf.get_mut().unwrap();
        let chunk_end = std::cmp::min(tail_start, read_so_far + chunk_vec.capacity());
        match (&mut src).take(chunk_end-read_so_far).read_to_end(chunk_vec) {
          Ok(n) => { read_so_far += n; },
          Err(e) => return Err(Error::IngressIO(e)),
        }
        if chunk_vec.is_empty() { return Err(Error::MalformedInput(read_so_far, "premature EOF")); }
        if let Err(_) = buffers_to_write_out.send(chunk_buf) {
          return Err(Error::CompanionThreadDied);
        }
      }
      // Read the entire tail.
      let tail_buf = get_buffer(&recycled_buffers_in, memory_usage);
      let tail_vec = tail_buf.get_mut().unwrap();
      assert!(tail_vec.len() + (frame_end-read_so_far) <= tail_vec.capacity());
      match (&mut src).take(frame_end-read_so_far).read_to_end(tail_vec) {
        Ok(n) => { read_so_far += n; },
        Err(e) => return Err(Error::IngressIO(e)),
      }
      if read_so_far < frame_end { return Err(Error::MalformedInput(read_so_far, "premature EOF")); }
      assert_eq!(tail_vec.len(), MAX_HEAD_AND_TAIL_LEN);
      if let Err(_) = buffers_to_write_out(tail_buf.clone()) {
        return Err(Error::CompanionThreadDied);
      }
      FrameBuffers { bounds: frame_start..frame_end, head: head_buf, tail: Some(tail_buf) }
    };
    if let Err(_) = buffers_to_shingleprint_out.send(fb) {
      return Err(Error::CompanionThreadDied);
    }
    frame_start = frame_end;
  }
  Ok(())
}

fn get_buffer<'sess>(recycled: &Receiver<Arc<Buffer<'sess>>>, memory_usage: &'sess AtomicUsize) -> Arc<Buffer<'sess>> {
  // Have we reached our memory usage target already?
  if memory_usage.load(atomic::Ordering::Relaxed) >= INGRESS_BUFFER_MEMORY_TARGET {
    // Wait for a recycled buffer.
    let deadline = Instant::now() + Duration::from_millis(50);
    loop {
      match recycled.recv_deadline(deadline) {
        Ok(arc) => match Arc::get_mut(arc) {
          Some(buf) => { buf.clear(); return arc },
          None => continue, // Another thread still holds a reference to this buffer.
        },
        Err(_) => break, // Disconnected.
      }
    }
  } else {
    // See if we can get a recycled buffer without blocking; otherwise we'll allocate a new one.
    loop {
      match recycled.try_recv() {
        Ok(arc) => match Arc::get_mut(arc) {
          Some(buf) => { buf.clear(); return arc },
          None => continue, // Another thread still holds a reference to this buffer.
        },
        Err(_) => break, // Disconnected or would block.
      }
    }
  }
  // Can't recycle.
  Buffer::new(memory_usage)
}

fn writing_thread<'sess>(
  buffers_in: Receiver<Arc<Buffer<'sess>>>,
  mut dest: impl Write,
  recycled_buffers_out: Sender<Arc<Buffer<'sess>>>,
) -> Result<(), Error> {
  while let Ok(buf) = buffers_in.recv() {
    let mut slice = buf.as_slice();
    while !slice.is_empty() {
      match dest.write(slice) {
        Ok(n) => { slice = &slice[n..]; },
        Err(e) if e.is_interrupted() => {},
        Err(e) => return Err(Error::SpoolIO(e)),
      }
    }
    // If channel is full or disconnected, just deallocate the buffer instead.
    let _ = recycled_buffers_out.send(buf);
  }
  Ok(())
}

fn shingleprinting_thread<'sess>(
  buffers_in: Receiver<FrameBuffers<'sess>>,
  frames_out: Sender<Frame>,
  recycled_buffers_out: Sender<Arc<Buffer<'sess>>>,
) -> Result<(), Error> {
  while let Ok(fb) = buffers_in.recv() {
    let frame = if let Some(tail) = fb.tail {
      Frame {
        bounds: fb.bounds,
        head_sp: shingleprint(&fb.head),
        tail_sp: shingleprint(&tail),
      }
      let _ = recycled_buffers_out.send(tail);
    } else if fb.head.len() > MAX_HEAD_AND_TAIL_LEN {
      Frame {
        bounds: fb.bounds,
        head_sp: shingleprint(&fb.head[..MAX_HEAD_AND_TAIL_LEN]),
        tail_sp: shingleprint(&fb.head[fb.head.len()-MAX_HEAD_AND_TAIL_LEN..]),
      }
    } else {
      let sp = shingleprint(&fb.head);
      Frame { bounds, head_sp: sp, tail_sp: sp }
    };
    let _ = recycled_buffers_out.send(fb.head);
    if let Err(_) = frames_out.send(frame) {
      return Err(Error::CompanionThreadDied);
    }
  }
  Ok(())
}

#[derive(Debug)]
struct FrameBuffers<'sess> {
  bounds: Range<usize>,
  head: Arc<Buffer<'sess>>,
  tail: Option<Arc<Buffer<'sess>>>,
}

const BUFFER_CAP: usize = MAX_HEAD_AND_TAIL_LEN * 2;

#[derive(Debug)]
struct Buffer<'sess> {
  data: Vec<u8>,
  memory_usage: &'sess AtomicUsize,
}

impl<'sess> Buffer<'sess> {
  fn new(memory_usage: &'sess AtomicUsize) -> Self {
    memory_usage.fetch_add(BUFFER_CAP, atomic::Ordering::Relaxed);
    Self {
      data: Vec::with_capacity(BUFFER_CAP),
      memory_usage,
    }
  }
}

impl<'sess> Drop for Buffer<'sess> {
  fn drop(&mut self) {
    self.memory_usage.fetch_sub(BUFFER_CAP, atomic::Ordering::Relaxed);
  }
}

impl<'sess> Deref for Buffer<'sess> {
  type Target = Vec<u8>;
  fn deref(&self) -> &Vec<u8> {
    &self.data
  }
}

impl<'sess> DerefMut for Buffer<'sess> {
  fn deref_mut(&mut self) -> &mut Vec<u8> {
    &mut self.data
  }
}
