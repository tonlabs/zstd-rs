use super::raw::{Operation, Status};
use std::io::{self, BufRead, Read, Write};
use zstd_safe;

pub struct Reader<R, D> {
    reader: R,
    operation: D,
    pub needs_data: bool,
}

pub struct Writer<W, D> {
    writer: W,
    operation: D,

    offset: usize,
    buffer: Vec<u8>,
    finished: bool,
}

impl<R: BufRead, D: Operation> Reader<R, D> {
    /// Creates a new Reader.
    pub fn new(reader: R, operation: D) -> Self {
        Reader {
            reader,
            operation,
            needs_data: true,
        }
    }

    /// Acquire a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Acquire a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the reader may result in surprising results if
    /// this decoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Returns the inner `BufRead`.
    pub fn finish(self) -> R {
        self.reader
    }
}

impl<R: BufRead, D: Operation> Read for Reader<R, D> {
    /// Performs the job of a `Read::read()`
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        // We will loop until *something* is written to `dst`.
        // Errors can happen:
        // * When reading more data from the reader.
        // * When decompressing, if bad data is found.
        loop {
            let eof;
            let status = {
                // If ANY error happen here, just forward it. We can safely resume.
                let src = self.reader.fill_buf()?;
                // If src is empty, we just need to finish the stream.
                eof = src.is_empty();

                let mut input_buffer =
                    zstd_safe::InBuffer { src: src, pos: 0 };
                let mut output_buffer =
                    zstd_safe::OutBuffer { dst: dst, pos: 0 };

                // Process the current buffer.
                // If ANY error happen here, it's also safe to return:
                // we didn't consume anything.
                let hint = if eof {
                    // We only accept EOF if we've had a hint=0 result before.
                    if self.needs_data {
                        // If need data but can't get any, give up.
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "incomplete frame",
                        ));
                    } else {
                        // If EOF has been reached, finish the stream.
                        // We need to keep calling this until the result = 0
                        self.operation.finish(&mut output_buffer)?
                    }
                } else {
                    // If we still have data to process, process it.
                    self.operation.run(&mut input_buffer, &mut output_buffer)?
                };

                Status {
                    bytes_read: input_buffer.pos,
                    bytes_written: output_buffer.pos,
                    remaining: hint,
                }
            };

            if !eof {
                self.needs_data = status.remaining != 0;
            }


            self.reader.consume(status.bytes_read);

            // Stop here if either:
            // * Something was written: there is no shame in returning now.
            // * EOF was reached: no point in reading more from an empty book.
            // * `dst` is empty: something's fishy here...
            if status.bytes_written != 0 || eof || dst.is_empty() {
                return Ok(status.bytes_written);
            }
        }
    }
}




impl<W: Write, D: Operation> Writer<W, D> {
    /// Returns a new Writer.
    pub fn new(writer: W, operation: D) -> Self {
        let buffer_size = zstd_safe::cstream_out_size();

        Writer {
            writer,
            operation,

            offset: 0,
            buffer: Vec::with_capacity(buffer_size),
            finished: false,
        }
    }

    /// Acquires a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Acquires a mutable reference to the underlying writer.
    ///
    /// Note that mutation of the writer may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Returns the inner writer.
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Write the current buffer.
    fn write_from_offset(&mut self) -> io::Result<()> {
        while self.offset < self.buffer.len() {
            let consumed = self.writer.write(&self.buffer[self.offset..])?;
            if consumed == 0 {
                // ?
                return Err(io::ErrorKind::WriteZero.into());
            }
            self.offset += consumed;
        }
        Ok(())
    }

    /// Expands the buffer before writing there.
    unsafe fn expand_buffer(&mut self) {
        let capacity = self.buffer.capacity();
        self.buffer.set_len(capacity);
    }

    /// Helpful for debugging.
    #[allow(dead_code)]
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Helpful for debugging.
    #[allow(dead_code)]
    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn finish(&mut self) -> io::Result<()> {
        loop {
            self.write_from_offset()?;

            if self.finished {
                return Ok(());
            }

            let (bytes_written, hint) = {
                // Flush zstd's internal buffers.
                unsafe { self.expand_buffer() };
                let mut dst = zstd_safe::OutBuffer::around(&mut self.buffer);
                let hint = self.operation.finish(&mut dst);

                (dst.pos, hint)
            };
            self.offset = 0;
            unsafe { self.buffer.set_len(bytes_written) };
            let hint = hint?;

            self.finished = hint == 0;

            self.write_from_offset()?;
        }
    }
}

impl<W: Write, D: Operation> Write for Writer<W, D> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Here again, we'll loop until ANY data has been consumed.
        // As soon as some input has been taken, we can't afford to take any
        // chances, as returning an error would make the user give us the
        // same input twice.
        loop {
            // First, write any pending data in `self.buffer`.
            self.write_from_offset()?;

            // Then, refill the buffer.
            let (bytes_read, bytes_written, hint) = {
                unsafe { self.expand_buffer() };
                let mut dst = zstd_safe::OutBuffer::around(&mut self.buffer);
                let mut src = zstd_safe::InBuffer::around(buf);
                let hint = self.operation.run(&mut src, &mut dst);

                (src.pos, dst.pos, hint)
            };
            self.offset = 0;
            unsafe { self.buffer.set_len(bytes_written) };
            let _ = hint?;

            // This is it, boys! We did something valuable with our lives!
            if bytes_read != 0 || buf.is_empty() {
                return Ok(bytes_read);
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // Before filling the buffer, make sure it's empty.
        self.write_from_offset()?;

        let (bytes_written, hint) = {
            // Flush zstd's internal buffers.
            unsafe { self.expand_buffer() };
            let mut dst = zstd_safe::OutBuffer::around(&mut self.buffer);
            let hint = self.operation.flush(&mut dst);

            (dst.pos, hint)
        };

        self.offset = 0;
        unsafe { self.buffer.set_len(bytes_written) };
        let _ = hint?;

        // Aaand empty the thing again.
        self.write_from_offset()?;
        Ok(())
    }
}
