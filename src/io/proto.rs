use byteorder::{ByteOrder, LittleEndian};
use defmt::*;
use micropb::{DecodeError, MessageDecode, MessageEncode, PbDecoder, PbEncoder, PbWrite};
use never;
use thiserror::Error;

use super::framed;
use crate::{crc, proto};

pub const SYNC_BYTE: u8 = 0xfc;
pub const MESSAGE_LENGTH_SIZE: usize = 2;
pub const MESSAGE_CRC_SIZE: usize = 2;

struct Writer<'a, T>
where
    T: framed::Sender,
{
    ftx: &'a mut T,
    crc: &'a mut crc::Stream,
}

impl<T> PbWrite for Writer<'_, T>
where
    T: framed::Sender,
{
    type Error = T::Error;

    fn pb_write(&mut self, data: &[u8]) -> Result<(), T::Error> {
        self.ftx.send_frame_fragment(data)?;
        self.crc.feed_bytes(data);
        Ok(())
    }
}

pub async fn send_target_message<T>(
    ftx: &mut T,
    msg: proto::api_::TargetMessage,
    crc: crc::Handle,
) -> Result<(), T::Error>
where
    T: framed::Sender,
{
    debug!("sending sync");
    ftx.send_sync()?;

    debug!("message is {=u16} bytes", msg.compute_size() as u16);
    #[allow(clippy::cast_possible_truncation)]
    let encoded_size = (msg.compute_size() as u16).to_le_bytes();
    ftx.send_frame_fragment(&encoded_size)?;

    let mut crc_stream = crc.stream().await;
    let mut writer = Writer {
        ftx,
        crc: &mut crc_stream,
    };
    let mut encoder = PbEncoder::new(&mut writer);
    debug!("sending message");
    msg.encode(&mut encoder)?;

    let crc_result = crc_stream.result().to_le_bytes();
    debug!("sending CRC");
    ftx.send_frame_fragment(&crc_result)
}

#[derive(Error)]
pub enum ReceiveError<T>
where
    T: framed::Receiver,
{
    #[error("sync error")]
    Sync,
    #[error("CRC-16 mismatch")]
    Crc,
    #[error("framing error")]
    Framing(T::Error),
    #[error("decode error")]
    Decode(DecodeError<never::Never>),
}

pub async fn receive_host_message<T>(
    frx: &mut T,
    crc: crc::Handle,
) -> Result<proto::api_::HostMessage, ReceiveError<T>>
where
    T: framed::Receiver,
{
    frx.receive_sync().await.or(Err(ReceiveError::Sync))?;

    debug!("got sync");

    // sync byte was seen, read the length of the message
    frx.receive_frame_fragment(0, MESSAGE_LENGTH_SIZE)
        .await
        .map_err(ReceiveError::Framing)?;

    debug!("got length");

    // get the message length from the buffer
    let message_len = LittleEndian::read_u16(&frx.buf()[..MESSAGE_LENGTH_SIZE]) as usize;

    // read the message
    let message_pos = MESSAGE_LENGTH_SIZE;
    frx.receive_frame_fragment(message_pos, message_len)
        .await
        .map_err(ReceiveError::Framing)?;

    debug!("got message");

    // read the CRC-16 of the message
    let crc_pos = message_pos + message_len;
    frx.receive_frame_fragment(crc_pos, MESSAGE_CRC_SIZE)
        .await
        .map_err(ReceiveError::Framing)?;

    debug!("got CRC");

    // get the expected CRC-16 from the buffer
    let expected_crc = LittleEndian::read_u16(&frx.buf()[crc_pos..crc_pos + MESSAGE_CRC_SIZE]);
    // compute the actual CRC-16
    let computed_crc = crc
        .compute(&frx.buf()[message_pos..message_pos + message_len])
        .await;
    // check the CRC-16
    if computed_crc != expected_crc {
        return Err(ReceiveError::Crc);
    }

    debug!("CRC valid");

    // at this point the buffer has been confirmed to contain a
    // valid message frame, so any failures beyond this point can
    // only drop the message, it cannot be re-parsed
    let mut decoder = PbDecoder::new(&frx.buf()[message_pos..message_pos + message_len]);
    let mut msg = proto::api_::HostMessage::default();
    let result = msg
        .decode(&mut decoder, message_len)
        .map_err(ReceiveError::Decode)
        .map(|_x| msg);

    frx.remove_frame(crc_pos + MESSAGE_CRC_SIZE);

    result
}
