//! Raw in-memory stream (de)compression.
//!
//! This module defines a `Decoder` and an `Encoder` to decode/encode streams
//! of data using buffers.
//!
//! They are mostly thin wrappers around `DStream`/`CStream`.

use parse_code;
use std::io;
use zstd_safe;

/// An in-memory decoder for streams of data.
pub struct Decoder {
    context: zstd_safe::DStream,
}

/// An in-memory encoder for streams of data.
pub struct Encoder {
    context: zstd_safe::CStream,
}

/// Represents an abstract compression/decompression operation.
///
/// This trait covers both `Decoder` and `Encoder`.
pub trait Operation {
    /// Performs a single step of this operation.
    ///
    /// Should return a hint for the next input size.
    fn run(
        &mut self,
        input: &mut zstd_safe::InBuffer,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize>;

    /// Flushes internal buffers, if any.
    ///
    /// Returns number of bytes still in internal buffer.
    fn flush(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize>;

    /// Finishes the operation, writing any footer if necessary.
    ///
    /// Returns the number of bytes still to write.
    /// Keep calling this method until it returns `0`.
    fn finish(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize>;
}

/// Describes the result of a compression/decompression call.
pub struct Status {
    /// Number of bytes expected for next input (this is just a hint).
    pub remaining: usize,

    /// Number of bytes read from the input.
    pub bytes_read: usize,

    /// Number of bytes written to the output.
    pub bytes_written: usize,
}

impl Decoder {
    /// Returns a new decoder.
    pub fn new() -> io::Result<Self> {
        Self::with_dictionary(&[])
    }

    /// Returns a new decoder, initialized with the given dictionary.
    pub fn with_dictionary(dictionary: &[u8]) -> io::Result<Self> {
        let mut context = zstd_safe::create_dstream();
        parse_code(
            zstd_safe::init_dstream_using_dict(&mut context, dictionary),
        )?;
        Ok(Decoder { context })
    }

    /// Performs a decompression step.
    ///
    /// Reads some data from `input`, writes some data to `output`.
    /// Returns `0` if we just finished a frame, otherwise an estimation of
    /// the bytes remaining bytes still to read.
    ///
    /// The position of each buffer will be updated.
    pub fn decompress(
        &mut self,
        input: &mut zstd_safe::InBuffer,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        parse_code(zstd_safe::decompress_stream(
            &mut self.context,
            output,
            input,
        ))
    }

    /// Performs a decompression step.
    ///
    /// This is a convenience wrapper around `decompress` if you don't want to deal with
    /// `InBuffer`/`OutBuffer`.
    pub fn decompress_buffers(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> io::Result<Status> {
        let mut input = zstd_safe::InBuffer { src: input, pos: 0 };
        let mut output = zstd_safe::OutBuffer {
            dst: output,
            pos: 0,
        };

        let remaining = self.decompress(&mut input, &mut output)?;

        Ok(Status {
            remaining,
            bytes_read: input.pos,
            bytes_written: output.pos,
        })
    }
}



impl Encoder {
    /// Creates a new encoder.
    pub fn new(level: i32) -> Self {
        Self::with_dictionary(level, &[]).unwrap()
    }

    /// Creates a new encoder using the given dictionary.
    pub fn with_dictionary(level: i32, dictionary: &[u8]) -> io::Result<Self> {
        let mut context = zstd_safe::create_cstream();

        // Initialize the stream with an existing dictionary
        parse_code(zstd_safe::init_cstream_using_dict(
            &mut context,
            dictionary,
            level,
        ))?;

        Ok(Self::with_context(context))
    }

    /// Returns an encoder using a prepared context.
    pub fn with_context(context: zstd_safe::CStream) -> Self {
        Encoder { context }
    }

    /// Performs a compression step.
    ///
    /// Reads some data from input, writes some data to output.
    /// Returns a hint for the number of bytes to give as input on next step.
    ///
    /// The position value on each buffer will be updated.
    pub fn compress(
        &mut self,
        input: &mut zstd_safe::InBuffer,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        parse_code(
            zstd_safe::compress_stream(&mut self.context, output, input),
        )
    }

    /// Performs a compression step.
    ///
    /// This is a convenience wrapper around `compress` if you don't want to deal with
    /// `InBuffer`/`OutBuffer`.
    pub fn compress_buffers(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> io::Result<Status> {
        let mut input = zstd_safe::InBuffer { src: input, pos: 0 };
        let mut output = zstd_safe::OutBuffer {
            dst: output,
            pos: 0,
        };

        let remaining = self.compress(&mut input, &mut output)?;

        Ok(Status {
            remaining,
            bytes_read: input.pos,
            bytes_written: output.pos,
        })
    }

    /// Returns the number of bytes still present in internal buffer.
    pub fn flush(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        parse_code(zstd_safe::flush_stream(&mut self.context, output))
    }

    /// Returns the number of bytes still present in internal buffer.
    ///
    /// If result is not 0, you should call this again.
    pub fn finish(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        parse_code(zstd_safe::end_stream(&mut self.context, output))
    }
}

impl Operation for Encoder {
    fn run(
        &mut self,
        input: &mut zstd_safe::InBuffer,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        self.compress(input, output)
    }

    fn flush(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        self.flush(output)
    }

    fn finish(
        &mut self,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        self.finish(output)
    }
}

impl Operation for Decoder {
    fn run(
        &mut self,
        input: &mut zstd_safe::InBuffer,
        output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        self.decompress(input, output)
    }

    fn flush(
        &mut self,
        _output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        Ok(0)
    }

    fn finish(
        &mut self,
        _output: &mut zstd_safe::OutBuffer,
    ) -> io::Result<usize> {
        Ok(0)
    }
}
