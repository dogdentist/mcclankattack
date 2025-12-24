use anyhow::Context;

use super::mc;

pub fn handshake(remote_address: String) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    let (address, port) = remote_address
        .rsplit_once(':')
        .context("invalid minecraft address")?;

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_HANDSHAKE_ID);
    // protocol version
    mc::write_varint(&mut buffer, mc::MC_VERSION_1_21_11);
    // server address
    mc::write_string(&mut buffer, address);
    // server port
    mc::write_port(
        &mut buffer,
        port.parse::<u16>().context("invalid minecraft address")?,
    );
    // next state
    mc::write_varint(&mut buffer, mc::PCKT_HANDSHAKE_LOGIN_INTENT);

    Ok(buffer)
}

pub fn login_start(username: &str, uuid: &[u8; 16]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_LOGIN_START_ID);
    // username
    mc::write_string(&mut buffer, username);
    // uuid
    buffer.extend_from_slice(uuid);

    buffer
}

pub fn login_ack() -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_LOGIN_ACK_ID);

    buffer
}

pub fn serverbound_known_packs(clientbound_packet: Vec<u8>) -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_SERVERBOUND_KNOWN_PACKS_ID);
    // 'we have it' - pretend we have the same pack
    buffer.extend_from_slice(&clientbound_packet);

    buffer
}

pub fn login_ack_finish_configuration() -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_ACK_FINISH_CONFIGURATION_ID);

    buffer
}

pub fn confirmation_teleportation(server_packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let teleportation_id = mc::read_varint(server_packet.iter())?;

    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_CONFIRM_TELEPORTATION_ID);
    // teleportation id
    mc::write_varint(&mut buffer, teleportation_id);

    Ok(buffer)
}

pub fn player_loaded() -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_PLAYER_LOADED_ID);

    buffer
}

pub fn keep_alive(server_packet: Vec<u8>) -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_SERVERBOUND_KEEP_ALIVE_ID);
    // keep alive id
    buffer.extend_from_slice(&server_packet);

    buffer
}

pub fn chat_message(message: &str) -> Vec<u8> {
    let mut buffer = Vec::new();

    // packet id
    mc::write_varint(&mut buffer, mc::PCKT_CHAT_MESSAGE_ID);
    // message
    mc::write_string(&mut buffer, message);
    // timestamp
    mc::write_long(&mut buffer, chrono::Utc::now().timestamp_millis());
    // salt
    mc::write_long(&mut buffer, 0);
    // signature (0 - not present)
    buffer.push(0);
    // ack offset
    mc::write_varint(&mut buffer, 0);
    // ack list
    buffer.extend_from_slice(&[0u8, 0u8, 0u8]);
    // checksum byte
    buffer.push(1);

    buffer
}
