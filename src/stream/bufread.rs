//! Decoder and Encoder around Buffered readers
use super::raw;
use super::zio;
use std::io::{self, BufRead, Read};

/// Decoder around any `BufRead`.
pub struct Decoder<R> {
    // Input reader
    reader: zio::Reader<R, raw::Decoder>,

    single_frame: bool,
    fused: bool,
}

impl<R: BufRead> Decoder<R> {
    /// Returns a new stream Decoder.
    pub fn new(reader: R) -> io::Result<Self> {
        Self::with_dictionary(reader, &[])
    }

    /// Returns a new stream Decoder using the given dictionary.
    pub fn with_dictionary(reader: R, dictionary: &[u8]) -> io::Result<Self> {
        raw::Decoder::with_dictionary(dictionary).map(|decoder| {
            Decoder {
                reader: zio::Reader::new(reader, decoder),
                single_frame: false,
                fused: false,
            }
        })
    }

    /// Instructs this decoder to stop after reading the first frame.
    pub fn set_single_frame(&mut self) {
        self.single_frame = true;
    }

    /// Acquire a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.reader.get_ref()
    }

    /// Acquire a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the reader may result in surprising results if
    /// this decoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut()
    }

    /// Returns the inner `BufRead`.
    pub fn finish(self) -> R {
        self.reader.finish()
    }
}

impl<R: BufRead> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.fused {
            return Ok(0);
        }

        let result = self.reader.read(buf)?;

        if self.single_frame && !self.reader.needs_data {
            self.fused = true;
        }

        Ok(result)
    }
}
