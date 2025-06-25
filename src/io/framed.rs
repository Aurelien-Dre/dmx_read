use crate::{fmt::error, io, utils};
use defmt::{error, *};
use embassy_time::Duration;
use embedded_io::{ReadReady, Write};
use embedded_io_async::Read;
use micropb::heapless::Vec;
use thiserror::Error;

pub trait Sender {
    type Error;

    fn send_sync(&mut self) -> Result<(), Self::Error>;
    fn send_frame_fragment(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}

pub trait Receiver {
    type Error;

    fn buf(&self) -> &[u8];
    fn clear_buf(&mut self);
    fn remove_frame(&mut self, bytes: usize);

    async fn receive_sync(&mut self) -> Result<(), Self::Error>;
    async fn receive_frame_fragment(
        &mut self,
        fragment_pos: usize,
        fragment_len: usize,
    ) -> Result<(), Self::Error>;
}

#[derive(Error)]
pub enum AsyncReadReceiverError<T>
where
    T: Read + ReadReady,
{
    #[error("insufficent buffer capacity")]
    BufferCapacity,
    #[error("timeout expired waiting for data")]
    Timeout,
    #[error("source error")]
    Other(T::Error),
}

pub struct AsyncReadReceiver<const N: usize, T>
where
    T: Read + ReadReady,
{
    buf: &'static mut Vec<u8, N>,
    rx: T,
    sync: u8,
    first_byte_timeout: Duration,
    between_bytes_timeout: Duration,
}

impl<const N: usize, T> AsyncReadReceiver<N, T>
where
    T: Read + ReadReady,
{
    pub fn new(
        buf: &'static mut Vec<u8, N>,
        rx: T,
        sync: u8,
        first_byte_timeout: Duration,
        between_bytes_timeout: Duration,
    ) -> Self {
        Self {
            buf,
            rx,
            sync,
            first_byte_timeout,
            between_bytes_timeout,
        }
    }
}

impl<const N: usize, T> Receiver for AsyncReadReceiver<N, T>
where
    T: Read + ReadReady,
{
    type Error = AsyncReadReceiverError<T>;

    fn buf(&self) -> &[u8] {
        self.buf.as_slice()
    }

    fn clear_buf(&mut self) {
        self.buf.clear();
    }

    fn remove_frame(&mut self, bytes: usize) {
        utils::remove_leading_bytes(self.buf, bytes);
    }

    async fn receive_sync(&mut self) -> Result<(), Self::Error> {
        // look for a sync byte in the buffer; the buffer may have
        // leftover bytes from a previous cycle (due to an incomplete
        // message, or a message which failed validation)
        if let Some(sync_pos) = self.buf.iter().position(|&x| x == self.sync) {
            // if a sync byte was found, reset the buffer to the data
            // following it
            //
            utils::remove_leading_bytes(self.buf, sync_pos + 1);
        } else {
            // remove any leftover data in the buffer, since a sync
            // byte will begin a new frame
            self.buf.clear();
            // wait for a sync byte to arrive
            let mut buf = [0_u8; 1];
            loop {
                debug!("waiting for sync byte");
                match self.rx.read(&mut buf[..]).await {
                    Ok(_) => {
                        trace!("rx byte {=u8:#x}", buf[0]);
                        if buf[0] == self.sync {
                            break;
                        }
                    }
                    Err(e) => {
                        return Err(AsyncReadReceiverError::Other(e));
                    }
                }
            }
        }
        Ok(())
    }

    async fn receive_frame_fragment(
        &mut self,
        fragment_pos: usize,
        fragment_len: usize,
    ) -> Result<(), Self::Error> {
        // compute the number of bytes available in the buffer, and the
        // number of bytes to read
        let bytes_available = self.buf[fragment_pos..].len();
        let bytes_to_read = fragment_len.saturating_sub(bytes_available);
        // if there are enough bytes in the buffer, return
        if bytes_to_read == 0 {
            return Ok(());
        }

        // if the fragment will not fit in the buffer's available space,
        // return an error
        if bytes_to_read > (self.buf.capacity() - self.buf.len()) {
            return Err(AsyncReadReceiverError::BufferCapacity);
        }

        // wait for the needed bytes to arrive, with timeouts
        let read_start = fragment_pos + bytes_available;
        let read_end = read_start + bytes_to_read;
        // make temporary space in the buffer for the bytes to be read
        unsafe {
            self.buf.set_len(read_end);
        }
        match io::read_exact_with_timeouts(
            &mut self.rx,
            &mut self.buf[read_start..read_end],
            self.first_byte_timeout,
            self.between_bytes_timeout,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(io::ReadError::Timeout { bytes_read }) => {
                // set the buffer length to include the number of bytes
                // that were actually read
                unsafe {
                    self.buf.set_len(read_start + bytes_read);
                }
                Err(AsyncReadReceiverError::Timeout)
            }
            Err(io::ReadError::Other { bytes_read, source }) => {
                // set the buffer length to include the number of bytes
                // that were actually read
                unsafe {
                    self.buf.set_len(read_start + bytes_read);
                }
                Err(AsyncReadReceiverError::Other(source))
            }
        }
    }
}

pub struct WriteSender<T>
where
    T: Write,
{
    tx: T,
    sync: u8,
}

impl<T> WriteSender<T>
where
    T: Write,
{
    pub fn new(tx: T, sync: u8) -> Self {
        Self { tx, sync }
    }
}

impl<T> Sender for WriteSender<T>
where
    T: Write,
{
    type Error = T::Error;

    fn send_sync(&mut self) -> Result<(), Self::Error> {
        let buf = [self.sync; 1];
        self.tx.write_all(&buf)
    }

    fn send_frame_fragment(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.tx.write_all(buf)
    }
}
