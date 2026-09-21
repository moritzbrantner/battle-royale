#![forbid(unsafe_code)]

use battle_royale_core::{
    BattleMatchId, CanonicalMatchSnapshot, CanonicalPlayerSnapshot, EliminationCause, MatchCommand,
    MatchPhase, MatchSnapshot, PlayerSnapshot, StormSnapshot, MAX_PLAYERS_PER_MATCH,
};
use std::error::Error;
use std::fmt;

pub const PROTOCOL_VERSION: u16 = 1;
const COMMAND_BYTES: usize = 5;
const COMMAND_KIND_MOVEMENT: u8 = 1;
const CANONICAL_MAGIC: [u8; 4] = *b"BRCA";
const PLAYER_MAGIC: [u8; 4] = *b"BRPL";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolError {
    message: String,
}

impl ProtocolError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProtocolError {}

#[must_use]
pub fn encode_command(command: MatchCommand) -> [u8; COMMAND_BYTES] {
    let mut bytes = [0_u8; COMMAND_BYTES];
    bytes[..2].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    match command {
        MatchCommand::SetMovement { x, z } => {
            bytes[2] = COMMAND_KIND_MOVEMENT;
            bytes[3] = x.to_le_bytes()[0];
            bytes[4] = z.to_le_bytes()[0];
        }
    }
    bytes
}

pub fn decode_command(payload: &[u8]) -> Result<MatchCommand, ProtocolError> {
    if payload.len() != COMMAND_BYTES {
        return Err(ProtocolError::new("invalid command length"));
    }
    let version = u16::from_le_bytes([payload[0], payload[1]]);
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::new("unsupported command protocol version"));
    }
    if payload[2] != COMMAND_KIND_MOVEMENT {
        return Err(ProtocolError::new("unsupported command kind"));
    }
    let x = i8::from_le_bytes([payload[3]]);
    let z = i8::from_le_bytes([payload[4]]);
    if !(-1..=1).contains(&x) || !(-1..=1).contains(&z) {
        return Err(ProtocolError::new(
            "movement components must be between -1 and 1",
        ));
    }
    Ok(MatchCommand::SetMovement { x, z })
}

#[must_use]
pub fn encode_canonical_snapshot(snapshot: &CanonicalMatchSnapshot) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(128 + snapshot.players.len() * 64);
    bytes.extend_from_slice(&CANONICAL_MAGIC);
    write_common_header(
        &mut bytes,
        snapshot.schema_version,
        snapshot.ruleset_revision,
        snapshot.match_id,
        snapshot.tick,
        snapshot.phase,
        snapshot.phase_started_tick,
        snapshot.lobby_ready_tick,
        snapshot.storm,
        snapshot.winner,
    );
    write_u16(
        &mut bytes,
        u16::try_from(snapshot.players.len()).unwrap_or(u16::MAX),
    );
    for player in &snapshot.players {
        write_canonical_player(&mut bytes, player);
    }
    bytes
}

pub fn decode_canonical_snapshot(payload: &[u8]) -> Result<CanonicalMatchSnapshot, ProtocolError> {
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(CANONICAL_MAGIC)?;
    let header = read_common_header(&mut cursor)?;
    let count = usize::from(cursor.read_u16()?);
    validate_player_count(count)?;
    let mut players = Vec::with_capacity(count);
    for _ in 0..count {
        players.push(read_canonical_player(&mut cursor)?);
    }
    cursor.finish()?;
    Ok(CanonicalMatchSnapshot {
        schema_version: header.schema_version,
        ruleset_revision: header.ruleset_revision,
        match_id: header.match_id,
        tick: header.tick,
        phase: header.phase,
        phase_started_tick: header.phase_started_tick,
        lobby_ready_tick: header.lobby_ready_tick,
        storm: header.storm,
        winner: header.winner,
        players,
    })
}

#[must_use]
pub fn encode_snapshot(snapshot: &MatchSnapshot) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(128 + snapshot.players.len() * 48);
    bytes.extend_from_slice(&PLAYER_MAGIC);
    write_common_header(
        &mut bytes,
        snapshot.schema_version,
        snapshot.ruleset_revision,
        snapshot.match_id,
        snapshot.tick,
        snapshot.phase,
        snapshot.phase_started_tick,
        snapshot.lobby_ready_tick,
        snapshot.storm,
        snapshot.winner,
    );
    write_u32(&mut bytes, snapshot.acknowledged_sequence);
    write_u16(
        &mut bytes,
        u16::try_from(snapshot.players.len()).unwrap_or(u16::MAX),
    );
    for player in &snapshot.players {
        write_player(&mut bytes, player);
    }
    bytes
}

pub fn decode_snapshot(payload: &[u8]) -> Result<MatchSnapshot, ProtocolError> {
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(PLAYER_MAGIC)?;
    let header = read_common_header(&mut cursor)?;
    let acknowledged_sequence = cursor.read_u32()?;
    let count = usize::from(cursor.read_u16()?);
    validate_player_count(count)?;
    let mut players = Vec::with_capacity(count);
    for _ in 0..count {
        players.push(read_player(&mut cursor)?);
    }
    cursor.finish()?;
    Ok(MatchSnapshot {
        schema_version: header.schema_version,
        ruleset_revision: header.ruleset_revision,
        match_id: header.match_id,
        tick: header.tick,
        phase: header.phase,
        phase_started_tick: header.phase_started_tick,
        lobby_ready_tick: header.lobby_ready_tick,
        storm: header.storm,
        winner: header.winner,
        acknowledged_sequence,
        players,
    })
}

#[derive(Clone, Copy)]
struct CommonHeader {
    schema_version: u16,
    ruleset_revision: u64,
    match_id: BattleMatchId,
    tick: u64,
    phase: MatchPhase,
    phase_started_tick: u64,
    lobby_ready_tick: Option<u64>,
    storm: StormSnapshot,
    winner: Option<u32>,
}

fn write_common_header(
    bytes: &mut Vec<u8>,
    schema_version: u16,
    ruleset_revision: u64,
    match_id: BattleMatchId,
    tick: u64,
    phase: MatchPhase,
    phase_started_tick: u64,
    lobby_ready_tick: Option<u64>,
    storm: StormSnapshot,
    winner: Option<u32>,
) {
    write_u16(bytes, PROTOCOL_VERSION);
    write_u16(bytes, schema_version);
    write_u64(bytes, ruleset_revision);
    write_u64(bytes, match_id.get());
    write_u64(bytes, tick);
    bytes.push(encode_phase(phase));
    write_u64(bytes, phase_started_tick);
    write_option_u64(bytes, lobby_ready_tick);
    write_i32(bytes, storm.center[0]);
    write_i32(bytes, storm.center[1]);
    write_i32(bytes, storm.radius);
    bytes.push(u8::from(storm.damage_active));
    write_option_u32(bytes, winner);
}

fn read_common_header(cursor: &mut Cursor<'_>) -> Result<CommonHeader, ProtocolError> {
    let protocol_version = cursor.read_u16()?;
    if protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::new("unsupported snapshot protocol version"));
    }
    Ok(CommonHeader {
        schema_version: cursor.read_u16()?,
        ruleset_revision: cursor.read_u64()?,
        match_id: BattleMatchId::new(cursor.read_u64()?),
        tick: cursor.read_u64()?,
        phase: decode_phase(cursor.read_u8()?)?,
        phase_started_tick: cursor.read_u64()?,
        lobby_ready_tick: cursor.read_option_u64()?,
        storm: StormSnapshot {
            center: [cursor.read_i32()?, cursor.read_i32()?],
            radius: cursor.read_i32()?,
            damage_active: cursor.read_bool()?,
        },
        winner: cursor.read_option_u32()?,
    })
}

fn write_canonical_player(bytes: &mut Vec<u8>, player: &CanonicalPlayerSnapshot) {
    write_u32(bytes, player.player_id);
    write_vec3(bytes, player.position);
    write_vec3(bytes, player.velocity);
    bytes.push(player.movement_x.to_le_bytes()[0]);
    bytes.push(player.movement_z.to_le_bytes()[0]);
    write_u32(bytes, player.last_sequence);
    write_u16(bytes, player.spawn_slot);
    write_u16(bytes, player.health);
    bytes.push(u8::from(player.alive));
    write_option_u64(bytes, player.elimination_tick);
    bytes.push(encode_cause(player.elimination_cause));
}

fn read_canonical_player(cursor: &mut Cursor<'_>) -> Result<CanonicalPlayerSnapshot, ProtocolError> {
    Ok(CanonicalPlayerSnapshot {
        player_id: cursor.read_u32()?,
        position: cursor.read_vec3()?,
        velocity: cursor.read_vec3()?,
        movement_x: i8::from_le_bytes([cursor.read_u8()?]),
        movement_z: i8::from_le_bytes([cursor.read_u8()?]),
        last_sequence: cursor.read_u32()?,
        spawn_slot: cursor.read_u16()?,
        health: cursor.read_u16()?,
        alive: cursor.read_bool()?,
        elimination_tick: cursor.read_option_u64()?,
        elimination_cause: decode_cause(cursor.read_u8()?)?,
    })
}

fn write_player(bytes: &mut Vec<u8>, player: &PlayerSnapshot) {
    write_u32(bytes, player.player_id);
    write_vec3(bytes, player.position);
    write_vec3(bytes, player.velocity);
    write_u16(bytes, player.health);
    bytes.push(u8::from(player.alive));
    write_option_u64(bytes, player.elimination_tick);
    bytes.push(encode_cause(player.elimination_cause));
}

fn read_player(cursor: &mut Cursor<'_>) -> Result<PlayerSnapshot, ProtocolError> {
    Ok(PlayerSnapshot {
        player_id: cursor.read_u32()?,
        position: cursor.read_vec3()?,
        velocity: cursor.read_vec3()?,
        health: cursor.read_u16()?,
        alive: cursor.read_bool()?,
        elimination_tick: cursor.read_option_u64()?,
        elimination_cause: decode_cause(cursor.read_u8()?)?,
    })
}

fn validate_player_count(count: usize) -> Result<(), ProtocolError> {
    if count > MAX_PLAYERS_PER_MATCH {
        return Err(ProtocolError::new("snapshot player count exceeds match capacity"));
    }
    Ok(())
}

fn encode_phase(phase: MatchPhase) -> u8 {
    match phase {
        MatchPhase::Waiting => 0,
        MatchPhase::Drop => 1,
        MatchPhase::Combat => 2,
        MatchPhase::Finished => 3,
    }
}

fn decode_phase(value: u8) -> Result<MatchPhase, ProtocolError> {
    match value {
        0 => Ok(MatchPhase::Waiting),
        1 => Ok(MatchPhase::Drop),
        2 => Ok(MatchPhase::Combat),
        3 => Ok(MatchPhase::Finished),
        _ => Err(ProtocolError::new("invalid match phase")),
    }
}

fn encode_cause(cause: Option<EliminationCause>) -> u8 {
    match cause {
        None => 0,
        Some(EliminationCause::Storm) => 1,
        Some(EliminationCause::SessionExpired) => 2,
    }
}

fn decode_cause(value: u8) -> Result<Option<EliminationCause>, ProtocolError> {
    match value {
        0 => Ok(None),
        1 => Ok(Some(EliminationCause::Storm)),
        2 => Ok(Some(EliminationCause::SessionExpired)),
        _ => Err(ProtocolError::new("invalid elimination cause")),
    }
}

fn write_vec3(bytes: &mut Vec<u8>, value: [i32; 3]) {
    write_i32(bytes, value[0]);
    write_i32(bytes, value[1]);
    write_i32(bytes, value[2]);
}

fn write_option_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            write_u64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn write_option_u32(bytes: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            bytes.push(1);
            write_u32(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn write_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn write_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn expect_magic(&mut self, expected: [u8; 4]) -> Result<(), ProtocolError> {
        if self.take(4)? != expected {
            return Err(ProtocolError::new("invalid snapshot magic"));
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn read_bool(&mut self) -> Result<bool, ProtocolError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ProtocolError::new("invalid boolean value")),
        }
    }

    fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, ProtocolError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i32(&mut self) -> Result<i32, ProtocolError> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_vec3(&mut self) -> Result<[i32; 3], ProtocolError> {
        Ok([self.read_i32()?, self.read_i32()?, self.read_i32()?])
    }

    fn read_option_u64(&mut self) -> Result<Option<u64>, ProtocolError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u64()?)),
            _ => Err(ProtocolError::new("invalid optional u64 tag")),
        }
    }

    fn read_option_u32(&mut self) -> Result<Option<u32>, ProtocolError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.read_u32()?)),
            _ => Err(ProtocolError::new("invalid optional u32 tag")),
        }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| ProtocolError::new("snapshot offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| ProtocolError::new("truncated snapshot"))?;
        self.offset = end;
        Ok(value)
    }

    fn finish(self) -> Result<(), ProtocolError> {
        if self.offset != self.bytes.len() {
            return Err(ProtocolError::new("snapshot has trailing bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battle_royale_core::{
        BattleRoyaleMatch, LOBBY_COUNTDOWN_TICKS, MatchCommand, MatchPhase,
    };

    #[test]
    fn command_roundtrip_is_exact_and_rejects_trailing_bytes() {
        let encoded = encode_command(MatchCommand::SetMovement { x: -1, z: 1 });
        assert_eq!(
            decode_command(&encoded).unwrap(),
            MatchCommand::SetMovement { x: -1, z: 1 }
        );

        let mut trailing = encoded.to_vec();
        trailing.push(0);
        assert_eq!(
            decode_command(&trailing).unwrap_err().message(),
            "invalid command length"
        );
    }

    #[test]
    fn canonical_and_player_snapshots_roundtrip() {
        let mut simulation = BattleRoyaleMatch::new(BattleMatchId::new(31));
        simulation.add_player(1).unwrap();
        simulation.add_player(2).unwrap();
        simulation
            .apply_command(1, 4, MatchCommand::SetMovement { x: 1, z: 0 })
            .unwrap();
        for _ in 0..LOBBY_COUNTDOWN_TICKS {
            simulation.advance_tick().unwrap();
        }
        assert_eq!(simulation.phase(), MatchPhase::Drop);

        let canonical = simulation.snapshot();
        let visible = simulation.snapshot_for_player(1).unwrap();

        assert_eq!(
            decode_canonical_snapshot(&encode_canonical_snapshot(&canonical)).unwrap(),
            canonical
        );
        assert_eq!(decode_snapshot(&encode_snapshot(&visible)).unwrap(), visible);
    }

    #[test]
    fn malformed_snapshot_fails_closed() {
        let mut simulation = BattleRoyaleMatch::new(BattleMatchId::new(32));
        simulation.add_player(1).unwrap();
        let mut encoded = encode_canonical_snapshot(&simulation.snapshot());
        encoded.push(9);
        assert_eq!(
            decode_canonical_snapshot(&encoded).unwrap_err().message(),
            "snapshot has trailing bytes"
        );
    }
}
