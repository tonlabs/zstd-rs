//! Compress and decompress individual blocks.
//!
//! These methods process all the input data at once.
//! It is therefore best used with relatively small blocks
//! (like small network packets).

mod compressor;
mod decompressor;

pub use self::compressor::Compressor;
pub use self::decompressor::Decompressor;

use std::io;

/// Compresses a single block of data to the given destination buffer.
///
/// Returns the number of bytes written, or an error if something happened
/// (for instance if the destination buffer was too small).
///
/// A level of `0` uses zstd's default (currently `3`).
pub fn compress_to_buffer(
    source: &[u8],
    destination: &mut [u8],
    level: i32,
) -> io::Result<usize> {
    Compressor::new().compress_to_buffer(source, destination, level)
}

/// Compresses a block of data and returns the compressed result.
///
/// A level of `0` uses zstd's default (currently `3`).
pub fn compress(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    Compressor::new().compress(data, level)
}

/// Deompress a single block of data to the given destination buffer.
///
/// Returns the number of bytes written, or an error if something happened
/// (for instance if the destination buffer was too small).
pub fn decompress_to_buffer(
    source: &[u8],
    destination: &mut [u8],
) -> io::Result<usize> {
    Decompressor::new().decompress_to_buffer(source, destination)
}

/// Decompresses a block of data and returns the decompressed result.
///
/// The decompressed data should be less than `capacity` bytes,
/// or an error will be returned.
pub fn decompress(data: &[u8], capacity: usize) -> io::Result<Vec<u8>> {
    Decompressor::new().decompress(data, capacity)
}

#[cfg(test)]
mod tests {
    use super::{compress, decompress};

    #[test]
    fn test_direct() {
        // Can we include_str!("assets/example.txt")?
        // It's excluded from the packaging step, so maybe not.
        let text = r#"
            ’Twas brillig, and the slithy toves
            Did gyre and gimble in the wade;
            All mimsy were the borogoves,
            And the mome raths outgrabe.


            "Beware the Jabberwock, my son!
            The jaws that bite, the claws that catch!
            Beware the Jubjub bird, and shun
            The frumious Bandersnatch!"


            He took his vorpal sword in hand:
            Long time the manxome foe he sought—
            So rested he by the Tumtum tree,
            And stood awhile in thought.


            And as in uffish thought he stood,
            The Jabberwock, with eyes of flame,
            Came whiffling through the tulgey wood,
            And burbled as it came!


            One, two! One, two! And through and through
            The vorpal blade went snicker-snack!
            He left it dead, and with its head
            He went galumphing back.


            "And hast thou slain the Jabberwock?
            Come to my arms, my beamish boy!
            O frabjous day! Callooh! Callay!"
            He chortled in his joy.


            ’Twas brillig, and the slithy toves
            Did gyre and gimble in the wabe;
            All mimsy were the borogoves,
            And the mome raths outgrabe.
        "#;

        crate::test_cycle_unwrap(
            text.as_bytes(),
            |data| compress(data, 1),
            |data| decompress(data, text.len()),
        );
    }
}
