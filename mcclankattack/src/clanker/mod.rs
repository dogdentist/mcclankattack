use std::{
    io::{Read, Write},
    sync::Arc,
};

use anyhow::{Context, anyhow};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::Mutex,
};

use crate::service;

mod mc;
mod pckt;

fn generate_player_uuid(username: &str) -> [u8; 16] {
    let mut prefix = b"OfflinePlayer:".to_vec();
    prefix.extend_from_slice(username.as_bytes());

    let hash = md5::compute(prefix);
    let mut uuid: [u8; 16] = [0; 16];

    uuid.copy_from_slice(hash.as_ref());
    uuid[6] = (uuid[6] & 0x0F) | 0x30;
    uuid[8] = (uuid[8] & 0x3F) | 0x80;

    uuid
}

type ClankerObject = Arc<Mutex<Clanker>>;

pub struct ClankerIo<T> {
    pub conn: T,
    pub clanker: ClankerObject,
}

impl<T> ClankerIo<T> {
    pub fn new(conn: T, clanker: ClankerObject) -> ClankerIo<T> {
        ClankerIo { conn, clanker }
    }
}

impl ClankerIo<OwnedWriteHalf> {
    /*
     * data must include the packet id
     */
    async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let (compression_enabled, compression_threshold) = {
            let clanker = self.clanker.lock().await;

            (clanker.compression, clanker.compression_threshold)
        };

        let mut inner = Vec::new();

        if compression_enabled {
            let data_len = data.len();

            if data_len >= compression_threshold {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&data)?;
                mc::write_varint(&mut inner, data_len as i32);
                inner.extend_from_slice(&encoder.finish().context("failed to finish compression")?);
            } else {
                mc::write_varint(&mut inner, 0);
                inner.extend_from_slice(&data);
            }
        } else {
            inner.extend_from_slice(&data);
        }

        let mut packet = Vec::new();
        mc::write_varint(&mut packet, inner.len() as i32);
        packet.extend_from_slice(&inner);

        if self
            .conn
            .write(&packet)
            .await
            .map_err(|e| anyhow!("failed to send a packet, error: {}", e.to_string()))?
            < packet.len()
        {
            return Err(anyhow!("unexpected closing of connection"));
        }

        Ok(())
    }
}

impl ClankerIo<OwnedReadHalf> {
    /*
     * .0 = packet id
     * .1 = packet data
     */
    async fn read(&mut self) -> anyhow::Result<(i32, Vec<u8>)> {
        let compression_enabled = { (self.clanker.lock().await).compression };

        let packet_len = mc::read_conn_varint(&mut self.conn).await? as usize;
        let mut packet = vec![0u8; packet_len];

        if self
            .conn
            .read_exact(&mut packet)
            .await
            .map_err(|e| anyhow!("failed to receive a packet, error: {}", e.to_string()))?
            < packet_len
        {
            return Err(anyhow!("unexpected closing of connection"));
        }

        let packet = if compression_enabled {
            let mut packet_iter = packet.iter();
            let uncompressed_len = mc::read_varint(&mut packet_iter)?;

            // != 0 is compressed
            if uncompressed_len > 0 {
                let packet: Vec<u8> = packet_iter.cloned().collect();
                let mut decoder = ZlibDecoder::new(&packet[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            } else {
                packet_iter.cloned().collect()
            }
        } else {
            packet
        };

        let mut packet_iter = packet.iter();
        let packet_id = mc::read_varint(&mut packet_iter)?;
        let packet: Vec<u8> = packet_iter.cloned().collect();

        Ok((packet_id, packet))
    }
}

pub struct Clanker {
    pub name: String,
    pub uuid: [u8; 16],
    pub remote_address: String,
    pub compression: bool,
    pub compression_threshold: usize,
}

impl Clanker {
    pub async fn new(
        name: String,
        remote_address: String,
    ) -> anyhow::Result<(TcpStream, ClankerObject)> {
        let uuid = generate_player_uuid(&name);

        let conn = tokio::net::TcpStream::connect(&remote_address)
            .await
            .map_err(|e| anyhow!("connection failed, {}", e.to_string()))?;

        Ok((
            conn,
            Arc::new(Mutex::new(Clanker {
                name,
                uuid,
                remote_address: remote_address.clone(),
                compression: false,
                compression_threshold: 0,
            })),
        ))
    }
}

pub async fn join_game(
    clanker: ClankerObject,
    conn_rx: &mut ClankerIo<OwnedReadHalf>,
    conn_tx: &mut Arc<Mutex<ClankerIo<OwnedWriteHalf>>>,
) -> anyhow::Result<()> {
    enum State {
        Hello,
        Configuration,
        Game,
    }

    let (remote_address, name, uuid) = {
        let clanker = clanker.lock().await;

        (
            clanker.remote_address.clone(),
            clanker.name.clone(),
            clanker.uuid,
        )
    };

    let handshake_packet = pckt::handshake(remote_address)?;
    conn_tx.lock().await.write(&handshake_packet).await?;

    let login_start_packet = pckt::login_start(&name, &uuid);
    conn_tx.lock().await.write(&login_start_packet).await?;

    let mut state = State::Hello;

    loop {
        match state {
            State::Hello => {
                let (packet_id, packet) = conn_rx.read().await?;

                // println!("> @HELLO {packet_id:02X} | {packet:?}");

                match packet_id {
                    mc::PCKT_SET_ENCRYPTION_ID => {
                        return Err(anyhow!(
                            "set encryption packet is not supported, this attacker is for offline servers btw, not online"
                        ));
                    }
                    mc::PCKT_SET_COMPRESSION_ID => {
                        let mut clanker = clanker.lock().await;

                        clanker.compression_threshold = mc::read_varint(packet.iter())? as usize;
                        clanker.compression = true;
                    }
                    mc::PCKT_LOGIN_SUCCESS_ID => {
                        let login_ack_packet = pckt::login_ack();
                        conn_tx.lock().await.write(&login_ack_packet).await?;

                        state = State::Configuration;
                    }
                    _ => {
                        return Err(anyhow!("unknown packet id '{packet_id:02X}' was provided"));
                    }
                }
            }
            State::Configuration => {
                let (packet_id, packet) = conn_rx.read().await?;

                // println!("> @CONF {packet_id:02X} | {packet:?}");

                match packet_id {
                    mc::PCKT_CLIENTBOUND_KNOWN_PACKS_ID => {
                        let serverbound_known_packs_packet = pckt::serverbound_known_packs(packet);
                        conn_tx
                            .lock()
                            .await
                            .write(&serverbound_known_packs_packet)
                            .await?;
                    }
                    mc::PCKT_FINISH_CONFIGURATION_ID => {
                        let ack_finish_configuration_packet =
                            pckt::login_ack_finish_configuration();
                        conn_tx
                            .lock()
                            .await
                            .write(&ack_finish_configuration_packet)
                            .await?;
                    }
                    mc::PCKT_LOGIN_PLAY_ID => state = State::Game,
                    _ => { /* crap we don't care or need */ }
                }
            }
            State::Game => {
                let (packet_id, packet) = conn_rx.read().await?;

                // println!(
                //     "> @GAME {packet_id:02X} | {:?}",
                //     &packet[..packet.len().min(32)]
                // );

                match packet_id {
                    mc::PCKT_SYNCHRONIZE_PLAYER_POSITION_ID => {
                        let confirm_teleportation_packet =
                            pckt::confirmation_teleportation(packet)?;
                        conn_tx
                            .lock()
                            .await
                            .write(&confirm_teleportation_packet)
                            .await?;
                    }
                    mc::PCKT_CHUNK_DATA_AND_UPDATE_LIGHT_ID => {
                        let player_loaded_packet = pckt::player_loaded();
                        conn_tx.lock().await.write(&player_loaded_packet).await?;

                        return Ok(());
                    }
                    _ => { /* crap we don't care or need */ }
                }
            }
        }
    }
}

/*
 * needed to handle incoming traffic to prevent the server of thinking
 * we time out'd or something
 * */
pub async fn game_handler(
    conn_rx: &mut ClankerIo<OwnedReadHalf>,
    conn_tx: &mut Arc<Mutex<ClankerIo<OwnedWriteHalf>>>,
) -> anyhow::Result<()> {
    loop {
        let (packet_id, packet) = conn_rx.read().await?;

        // println!("> @GAME PACKET 0x{packet_id:02X}");

        match packet_id {
            mc::PCKT_SYNCHRONIZE_PLAYER_POSITION_ID => {
                let confirm_teleportation_packet = pckt::confirmation_teleportation(packet)?;
                conn_tx
                    .lock()
                    .await
                    .write(&confirm_teleportation_packet)
                    .await?;
            }
            mc::PCKT_CLIENTBOUND_KEEP_ALIVE_ID => {
                let keep_alive_packet = pckt::keep_alive(packet);
                conn_tx.lock().await.write(&keep_alive_packet).await?;
            }
            mc::PCKT_DISCONNECT_ID => {
                return Err(anyhow!("disconnected by server :("));
            }
            _ => {}
        }
    }
}

pub async fn spam_messages(
    conn_tx: &mut Arc<Mutex<ClankerIo<OwnedWriteHalf>>>,
    message_interval: u64,
    clanker_messages: Arc<service::ClankerMessages>,
) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(message_interval)).await;

        let message_packet = pckt::chat_message(&clanker_messages.message());
        {
            conn_tx.lock().await.write(&message_packet).await?;
        }
    }
}
