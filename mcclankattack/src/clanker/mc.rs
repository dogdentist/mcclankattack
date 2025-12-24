use anyhow::anyhow;
use tokio::io::AsyncReadExt;

pub const MC_VERSION_1_21_11: i32 = 774;

pub const PCKT_HANDSHAKE_ID: i32 = 0x00;
pub const PCKT_LOGIN_START_ID: i32 = 0x00;
pub const PCKT_SET_ENCRYPTION_ID: i32 = 0x01;
pub const PCKT_SET_COMPRESSION_ID: i32 = 0x03;
pub const PCKT_LOGIN_SUCCESS_ID: i32 = 0x02;
pub const PCKT_LOGIN_ACK_ID: i32 = 0x03;
pub const PCKT_CLIENTBOUND_KNOWN_PACKS_ID: i32 = 0x0E;
pub const PCKT_SERVERBOUND_KNOWN_PACKS_ID: i32 = 0x07;
pub const PCKT_FINISH_CONFIGURATION_ID: i32 = 0x03;
pub const PCKT_ACK_FINISH_CONFIGURATION_ID: i32 = 0x03;
pub const PCKT_LOGIN_PLAY_ID: i32 = 0x30;
pub const PCKT_SYNCHRONIZE_PLAYER_POSITION_ID: i32 = 0x46;
pub const PCKT_CONFIRM_TELEPORTATION_ID: i32 = 0x00;
pub const PCKT_CHUNK_DATA_AND_UPDATE_LIGHT_ID: i32 = 0x2C;
pub const PCKT_PLAYER_LOADED_ID: i32 = 0x2B;
pub const PCKT_CLIENTBOUND_KEEP_ALIVE_ID: i32 = 0x2B;
pub const PCKT_SERVERBOUND_KEEP_ALIVE_ID: i32 = 0x1B;
pub const PCKT_CHAT_MESSAGE_ID: i32 = 0x08;
pub const PCKT_DISCONNECT_ID: i32 = 0x20;

pub const PCKT_HANDSHAKE_LOGIN_INTENT: i32 = 2;

pub fn write_varint(destination: &mut Vec<u8>, mut value: i32) {
    while value & !0x7F != 0 {
        destination.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }

    destination.push(value as u8);
}

pub fn write_port(destination: &mut Vec<u8>, value: u16) {
    destination.extend_from_slice(&value.to_be_bytes());
}

pub fn write_long(destination: &mut Vec<u8>, value: i64) {
    destination.extend_from_slice(&value.to_be_bytes());
}

pub fn write_string(destination: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len() as i32;

    write_varint(destination, len);
    destination.extend_from_slice(bytes);
}

pub async fn read_conn_varint(client: &mut tokio::net::tcp::OwnedReadHalf) -> anyhow::Result<i32> {
    let mut value = 0i32;
    let mut shift = 0;

    loop {
        let mut byte = [0u8; 1];
        if client.read_exact(&mut byte).await.map_err(|e| {
            anyhow!(
                "error occured while reading from the server, error: {}",
                e.to_string()
            )
        })? < 1
        {
            return Err(anyhow!("unexpected closing of stream"));
        }

        let byte = byte[0];
        value |= ((byte & 0x7F) as i32) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
    }

    Ok(value)
}

pub fn read_varint<'a, I>(mut iter: I) -> anyhow::Result<i32>
where
    I: Iterator<Item = &'a u8>,
{
    let mut value = 0i32;
    let mut shift = 0;

    loop {
        if let Some(&byte) = iter.next() {
            value |= ((byte & 0x7F) as i32) << shift;

            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
        } else {
            return Err(anyhow!("unexpected end of input while reading varint"));
        }
    }

    Ok(value)
}

pub fn read_string<'a, I>(mut iter: I) -> anyhow::Result<String>
where
    I: Iterator<Item = &'a u8>,
{
    let length = read_varint(&mut iter)? as usize;
    let mut buffer = Vec::with_capacity(length);

    for _ in 0..length {
        if let Some(byte) = iter.next() {
            buffer.push(*byte);
        } else {
            return Err(anyhow!("unexpected end of input while reading string"));
        }
    }

    String::from_utf8(buffer).map_err(|e| anyhow!("failed to decode string: {}", e))
}
