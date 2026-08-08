use std::io;

use super::{
    CHUNK_MAGIC, DoorSoABlock, EnemySpawnSoABlock, RectSoABlock, TextPointSoABlock,
    TriggerSoABlock, WorldChunk, WorldPortalSoABlock, invalid_data,
};

const BLOCK_STATIC_RECTS: u8 = 1;
const BLOCK_HAZARD_RECTS: u8 = 2;
const BLOCK_DOORS: u8 = 3;
const BLOCK_CHECKPOINTS: u8 = 4;
const BLOCK_TRIGGERS: u8 = 5;
const BLOCK_ENEMY_SPAWNS: u8 = 6;
const BLOCK_TEXT_POINTS: u8 = 7;
const BLOCK_WORLD_PORTALS: u8 = 8;

pub(super) fn encode_chunk(chunk: &WorldChunk) -> Vec<u8> {
    let mut writer = BinWriter::new();

    writer.bytes.extend_from_slice(CHUNK_MAGIC);
    writer.u32(chunk.chunk_id);
    writer.i32(chunk.origin_q[0]);
    writer.i32(chunk.origin_q[1]);
    for value in chunk.bounds_local {
        writer.i16(value);
    }
    writer.u16(8);
    write_rect_block(&mut writer, BLOCK_STATIC_RECTS, &chunk.static_rects);
    write_rect_block(&mut writer, BLOCK_HAZARD_RECTS, &chunk.hazard_rects);
    write_door_block(&mut writer, &chunk.doors);
    write_rect_block(&mut writer, BLOCK_CHECKPOINTS, &chunk.checkpoints);
    write_trigger_block(&mut writer, &chunk.triggers);
    write_enemy_spawn_block(&mut writer, &chunk.enemy_spawns);
    write_text_point_block(&mut writer, &chunk.text_points);
    write_world_portal_block(&mut writer, &chunk.world_portals);
    writer.bytes
}

pub(super) fn decode_chunk(bytes: &[u8]) -> io::Result<WorldChunk> {
    let mut reader = BinReader::new(bytes);
    let magic = reader.bytes(CHUNK_MAGIC.len())?;
    if magic != CHUNK_MAGIC {
        return Err(invalid_data("invalid .wchunk magic"));
    }

    let chunk_id = reader.u32()?;
    let origin_q = [reader.i32()?, reader.i32()?];
    let bounds_local = [reader.i16()?, reader.i16()?, reader.i16()?, reader.i16()?];
    let block_count = reader.u16()?;
    let mut chunk = WorldChunk {
        chunk_id,
        origin_q,
        bounds_local,
        ..WorldChunk::default()
    };

    for _ in 0..block_count {
        let tag = reader.u8()?;
        match tag {
            BLOCK_STATIC_RECTS => chunk.static_rects = read_rect_block(&mut reader)?,
            BLOCK_HAZARD_RECTS => chunk.hazard_rects = read_rect_block(&mut reader)?,
            BLOCK_DOORS => chunk.doors = read_door_block(&mut reader)?,
            BLOCK_CHECKPOINTS => chunk.checkpoints = read_rect_block(&mut reader)?,
            BLOCK_TRIGGERS => chunk.triggers = read_trigger_block(&mut reader)?,
            BLOCK_ENEMY_SPAWNS => chunk.enemy_spawns = read_enemy_spawn_block(&mut reader)?,
            BLOCK_TEXT_POINTS => chunk.text_points = read_text_point_block(&mut reader)?,
            BLOCK_WORLD_PORTALS => chunk.world_portals = read_world_portal_block(&mut reader)?,
            other => return Err(invalid_data(format!("unknown .wchunk block tag {other}"))),
        }
    }
    if !reader.is_finished() {
        return Err(invalid_data("trailing bytes after .wchunk blocks"));
    }

    Ok(chunk)
}

fn write_rect_block(writer: &mut BinWriter, tag: u8, block: &RectSoABlock) {
    writer.u8(tag);
    writer.u32(block.len() as u32);
    write_rect_payload(writer, block);
}

fn read_rect_block(reader: &mut BinReader<'_>) -> io::Result<RectSoABlock> {
    let count = reader.u32()? as usize;

    read_rect_payload(reader, count)
}

fn write_door_block(writer: &mut BinWriter, block: &DoorSoABlock) {
    writer.u8(BLOCK_DOORS);
    writer.u32(block.len() as u32);
    write_rect_payload(writer, &block.rects);
    writer.u16_slice(&block.radius);
    writer.u16_slice(&block.speed);
    writer.u8_slice(&block.automatic);
}

fn read_door_block(reader: &mut BinReader<'_>) -> io::Result<DoorSoABlock> {
    let count = reader.u32()? as usize;
    let rects = read_rect_payload(reader, count)?;

    Ok(DoorSoABlock {
        rects,
        radius: reader.u16_vec(count)?.into_boxed_slice(),
        speed: reader.u16_vec(count)?.into_boxed_slice(),
        automatic: reader.u8_vec(count)?.into_boxed_slice(),
    })
}

fn write_trigger_block(writer: &mut BinWriter, block: &TriggerSoABlock) {
    writer.u8(BLOCK_TRIGGERS);
    writer.u32(block.len() as u32);
    write_rect_payload(writer, &block.rects);
    writer.u8_slice(&block.kind);
    writer.u16_slice(&block.enemy_id);
}

fn read_trigger_block(reader: &mut BinReader<'_>) -> io::Result<TriggerSoABlock> {
    let count = reader.u32()? as usize;
    let rects = read_rect_payload(reader, count)?;

    Ok(TriggerSoABlock {
        rects,
        kind: reader.u8_vec(count)?.into_boxed_slice(),
        enemy_id: reader.u16_vec(count)?.into_boxed_slice(),
    })
}

fn write_enemy_spawn_block(writer: &mut BinWriter, block: &EnemySpawnSoABlock) {
    writer.u8(BLOCK_ENEMY_SPAWNS);
    writer.u32(block.x.len() as u32);
    writer.u16_slice(&block.meta);
    writer.i16_slice(&block.x);
    writer.i16_slice(&block.y);
    writer.u8_slice(&block.kind);
    writer.u16_slice(&block.spawn_id);
    writer.u16_slice(&block.spawn_wave);
    writer.i16_slice(&block.editor_layer);
}

fn read_enemy_spawn_block(reader: &mut BinReader<'_>) -> io::Result<EnemySpawnSoABlock> {
    let count = reader.u32()? as usize;

    Ok(EnemySpawnSoABlock {
        meta: reader.u16_vec(count)?.into_boxed_slice(),
        x: reader.i16_vec(count)?.into_boxed_slice(),
        y: reader.i16_vec(count)?.into_boxed_slice(),
        kind: reader.u8_vec(count)?.into_boxed_slice(),
        spawn_id: reader.u16_vec(count)?.into_boxed_slice(),
        spawn_wave: reader.u16_vec(count)?.into_boxed_slice(),
        editor_layer: reader.i16_vec(count)?.into_boxed_slice(),
    })
}

fn write_text_point_block(writer: &mut BinWriter, block: &TextPointSoABlock) {
    writer.u8(BLOCK_TEXT_POINTS);
    writer.u32(block.x.len() as u32);
    writer.u16_slice(&block.meta);
    writer.i16_slice(&block.x);
    writer.i16_slice(&block.y);
    writer.i16_slice(&block.editor_layer);
    for value in &block.text {
        writer.string(value);
    }
}

fn read_text_point_block(reader: &mut BinReader<'_>) -> io::Result<TextPointSoABlock> {
    let count = reader.u32()? as usize;

    Ok(TextPointSoABlock {
        meta: reader.u16_vec(count)?.into_boxed_slice(),
        x: reader.i16_vec(count)?.into_boxed_slice(),
        y: reader.i16_vec(count)?.into_boxed_slice(),
        editor_layer: reader.i16_vec(count)?.into_boxed_slice(),
        text: (0..count)
            .map(|_| reader.string())
            .collect::<io::Result<Vec<_>>>()?
            .into_boxed_slice(),
    })
}

fn write_world_portal_block(writer: &mut BinWriter, block: &WorldPortalSoABlock) {
    writer.u8(BLOCK_WORLD_PORTALS);
    writer.u32(block.x.len() as u32);
    writer.u16_slice(&block.meta);
    writer.i16_slice(&block.x);
    writer.i16_slice(&block.y);
    writer.i16_slice(&block.normal_x);
    writer.i16_slice(&block.normal_y);
    writer.i16_slice(&block.tangent_x);
    writer.i16_slice(&block.tangent_y);
    writer.u16_slice(&block.width);
    writer.u16_slice(&block.portal_id);
    writer.u16_slice(&block.receiver_id);
    writer.i16_slice(&block.priority);
    writer.u16_slice(&block.scale);
    writer.u8_slice(&block.flags);
    writer.u16_slice(&block.seamless_depth);
    writer.u16_slice(&block.seamless_angle);
    writer.i16_slice(&block.editor_layer);
}

fn read_world_portal_block(reader: &mut BinReader<'_>) -> io::Result<WorldPortalSoABlock> {
    let count = reader.u32()? as usize;

    Ok(WorldPortalSoABlock {
        meta: reader.u16_vec(count)?.into_boxed_slice(),
        x: reader.i16_vec(count)?.into_boxed_slice(),
        y: reader.i16_vec(count)?.into_boxed_slice(),
        normal_x: reader.i16_vec(count)?.into_boxed_slice(),
        normal_y: reader.i16_vec(count)?.into_boxed_slice(),
        tangent_x: reader.i16_vec(count)?.into_boxed_slice(),
        tangent_y: reader.i16_vec(count)?.into_boxed_slice(),
        width: reader.u16_vec(count)?.into_boxed_slice(),
        portal_id: reader.u16_vec(count)?.into_boxed_slice(),
        receiver_id: reader.u16_vec(count)?.into_boxed_slice(),
        priority: reader.i16_vec(count)?.into_boxed_slice(),
        scale: reader.u16_vec(count)?.into_boxed_slice(),
        flags: reader.u8_vec(count)?.into_boxed_slice(),
        seamless_depth: reader.u16_vec(count)?.into_boxed_slice(),
        seamless_angle: reader.u16_vec(count)?.into_boxed_slice(),
        editor_layer: reader.i16_vec(count)?.into_boxed_slice(),
    })
}

fn write_rect_payload(writer: &mut BinWriter, block: &RectSoABlock) {
    writer.u16_slice(&block.meta);
    writer.i16_slice(&block.x);
    writer.i16_slice(&block.y);
    writer.u16_slice(&block.w);
    writer.u16_slice(&block.h);
    writer.i16_slice(&block.rotation);
    writer.i16_slice(&block.editor_layer);
}

fn read_rect_payload(reader: &mut BinReader<'_>, count: usize) -> io::Result<RectSoABlock> {
    Ok(RectSoABlock {
        meta: reader.u16_vec(count)?.into_boxed_slice(),
        x: reader.i16_vec(count)?.into_boxed_slice(),
        y: reader.i16_vec(count)?.into_boxed_slice(),
        w: reader.u16_vec(count)?.into_boxed_slice(),
        h: reader.u16_vec(count)?.into_boxed_slice(),
        rotation: reader.i16_vec(count)?.into_boxed_slice(),
        editor_layer: reader.i16_vec(count)?.into_boxed_slice(),
    })
}
struct BinWriter {
    bytes: Vec<u8>,
}

impl BinWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i16(&mut self, value: i16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u8_slice(&mut self, values: &[u8]) {
        self.bytes.extend_from_slice(values);
    }

    fn u16_slice(&mut self, values: &[u16]) {
        for value in values {
            self.u16(*value);
        }
    }

    fn i16_slice(&mut self, values: &[i16]) {
        for value in values {
            self.i16(*value);
        }
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }
}

struct BinReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn bytes(&mut self, len: usize) -> io::Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| invalid_data("binary read overflow"))?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| invalid_data("unexpected end of binary data"))?;

        self.offset = end;
        Ok(slice)
    }

    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> io::Result<u16> {
        let bytes = self.bytes(2)?;

        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn i16(&mut self) -> io::Result<i16> {
        let bytes = self.bytes(2)?;

        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> io::Result<u32> {
        let bytes = self.bytes(4)?;

        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self) -> io::Result<i32> {
        let bytes = self.bytes(4)?;

        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn string(&mut self) -> io::Result<String> {
        let len = self.u32()? as usize;
        let bytes = self.bytes(len)?;

        String::from_utf8(bytes.to_vec()).map_err(|_| invalid_data("invalid utf-8 string in chunk"))
    }

    fn u8_vec(&mut self, count: usize) -> io::Result<Vec<u8>> {
        Ok(self.bytes(count)?.to_vec())
    }

    fn u16_vec(&mut self, count: usize) -> io::Result<Vec<u16>> {
        (0..count).map(|_| self.u16()).collect()
    }

    fn i16_vec(&mut self, count: usize) -> io::Result<Vec<i16>> {
        (0..count).map(|_| self.i16()).collect()
    }
}
