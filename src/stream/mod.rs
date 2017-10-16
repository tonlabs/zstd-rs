//! Compress and decompress Zstd streams.
//!
//! This module provide a `Read`/`Write` interface
//! to zstd streams of arbitrary length.
//!
//! They are compatible with the `zstd` command-line tool.

pub mod raw;
pub mod bufread;
mod zio;

mod encoder;
mod decoder;
mod functions;



pub use self::encoder::{AutoFinishEncoder, Encoder};
pub use self::decoder::Decoder;
pub use self::functions::{decode_all, encode_all, copy_encode, copy_decode};

#[cfg(test)]
mod tests;
