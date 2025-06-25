use embassy_futures::select::*;
use embassy_time::{Duration, Timer};
use embedded_io::ReadReady;
use embedded_io_async::Read;
use thiserror::Error;

pub mod framed;
pub mod proto;

#[derive(Error)]
pub enum ReadError<T>
where
    T: Read + ReadReady,
{
    #[error("timeout expired waiting for data")]
    Timeout { bytes_read: usize },
    #[error("source error")]
    Other { bytes_read: usize, source: T::Error },
}

/// Reads data from an object which implements both
/// [`embedded_io_async::Read`] and [`embedded_io::ReadReady`], and
/// supports two timeouts:
///
/// `first_byte`: if no data is available immediately, will wait this
/// long for the first byte to arrive
///
/// `between_bytes`: after the first byte has been read, will wait
/// this long between bytes
///
/// Returns [`ReadError::Timeout`] if a timeout expires; the
/// `bytes_read` field of the error will contain the number of bytes
/// which had been read before the timeout occurred, if any
///
/// Returns [`ReadError::Other`] if an error occurs in the source
/// object; the `bytes_read` field of the error will contain the
/// number of bytes which had been read before the error occurred, if
/// any
///
/// Returns `Ok` if no error or timeout occurs
async fn read_exact_with_timeouts<T>(
    rx: &mut T,
    mut buf: &mut [u8],
    first_byte: Duration,
    between_bytes: Duration,
) -> Result<(), ReadError<T>>
where
    T: Read + ReadReady,
{
    let mut bytes_read = 0;

    while !buf.is_empty() {
        // check for immediately-available bytes, and read them
        match rx.read_ready() {
            Ok(false) => {}
            Ok(true) => match rx.read(buf).await {
                Ok(r) => {
                    buf = &mut buf[r..];
                    bytes_read += r;
                    continue;
                }
                Err(e) => {
                    return Err(ReadError::Other {
                        source: e,
                        bytes_read,
                    });
                }
            },
            Err(e) => {
                return Err(ReadError::Other {
                    source: e,
                    bytes_read,
                });
            }
        }
        // set a timeout based on whether the first byte has been read
        let timeout = Timer::after(if bytes_read == 0 {
            first_byte
        } else {
            between_bytes
        });
        // setup a single-byte read for the next byte; if more than
        // one byte becomes available, the remainder will be read at
        // the top of the next loop iteration
        let src = rx.read(&mut buf[0..1]);
        // wait for either the read or the timeout to complete
        match select(src, timeout).await {
            Either::First(Ok(r)) => {
                buf = &mut buf[1..];
                bytes_read += r;
            }
            Either::First(Err(e)) => {
                return Err(ReadError::Other {
                    source: e,
                    bytes_read,
                });
            }
            Either::Second(()) => {
                return Err(ReadError::Timeout { bytes_read });
            }
        }
    }

    Ok(())
}
