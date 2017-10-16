use super::bufread;
use std::io::{self, Read, BufReader};

#[cfg(feature = "tokio")]
use tokio_io::AsyncRead;
use zstd_safe;


/// A decoder that decompress input data from another `Read`.
///
/// This allows to read a stream of compressed data
/// (good for files or heavy network stream).
pub struct Decoder<R: Read> {
    inner: bufread::Decoder<BufReader<R>>,
}

impl<R: Read> Decoder<R> {
    /// Creates a new decoder.
    pub fn new(reader: R) -> io::Result<Self> {
        Self::with_dictionary(reader, &[])
    }

    /// Creates a new decoder, using an existing dictionary.
    ///
    /// The dictionary must be the same as the one used during compression.
    pub fn with_dictionary(reader: R, dictionary: &[u8]) -> io::Result<Self> {

        let buffer_size = zstd_safe::dstream_in_size();
        bufread::Decoder::with_dictionary(
                BufReader::with_capacity(buffer_size, reader),
                dictionary)
            .map(|inner| Decoder { inner })
    }

    /// Recommendation for the size of the output buffer.
    pub fn recommended_output_size() -> usize {
        zstd_safe::dstream_out_size()
    }

    /// Acquire a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Instructs this decoder to stop after reading the first frame.
    pub fn set_single_frame(&mut self) {
        self.inner.set_single_frame();
    }

    /// Instructs this decoder to stop after reading the first frame.
    pub fn single_frame(mut self) -> Self {
        self.set_single_frame();
        self
    }

    /// Acquire a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the reader may result in surprising results if
    /// this decoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Return the inner `Read`.
    ///
    /// Calling `finish()` is not *required* after reading a stream -
    /// just use it if you need to get the `Read` back.
    pub fn finish(self) -> R {
        self.inner.finish().into_inner()
    }
}

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for Decoder<R> {
    unsafe fn prepare_uninitialized_buffer(&self, _buf: &mut [u8]) -> bool {
        false
    }
}

fn _assert_traits() {
    use std::io::Cursor;

    fn _assert_send<T: Send>(_: T) {}

    _assert_send(Decoder::new(Cursor::new(Vec::new())));
}

#[cfg(test)]
#[cfg(feature = "tokio")]
mod async_tests {
    use futures::Future;
    use partial_io::{GenWouldBlock, PartialAsyncRead,
                     PartialWithErrors};
    use quickcheck::quickcheck;
    use std::io::{self, Cursor};
    use tokio_io::{io as tokio_io, AsyncRead, AsyncWrite};

    #[test]
    fn test_async_read() {
        use stream::encode_all;

        let source = "abc".repeat(1024 * 10).into_bytes();
        let encoded = encode_all(&source[..], 1).unwrap();
        let writer =
            test_async_read_worker(&encoded[..], Cursor::new(Vec::new()))
                .unwrap();
        let output = writer.into_inner();
        assert_eq!(source, output);
    }

    #[test]
    fn test_async_read_partial() {
        quickcheck(test as fn(_) -> _);

        fn test(encode_ops: PartialWithErrors<GenWouldBlock>) {
            use stream::encode_all;

            let source = "abc".repeat(1024 * 10).into_bytes();
            let encoded = encode_all(&source[..], 1).unwrap();
            let reader = PartialAsyncRead::new(&encoded[..], encode_ops);
            let writer =
                test_async_read_worker(reader, Cursor::new(Vec::new()))
                    .unwrap();
            let output = writer.into_inner();
            assert_eq!(source, output);
        }
    }

    fn test_async_read_worker<R: AsyncRead, W: AsyncWrite>(
        r: R,
        w: W,
    ) -> io::Result<W> {
        use super::Decoder;

        let decoder = Decoder::new(r).unwrap();
        let (_, _, w) = tokio_io::copy(decoder, w).wait()?;
        Ok(w)
    }
}
