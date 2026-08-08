// The v4 package/runtime surface is intentionally broader than the first consumers.
#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::Path;

mod binio;
mod chunk;
mod error;
mod manifest;
mod quant;
mod runtime;
mod security;
mod sha256;
mod text_format;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use chunk::{
    ChunkObjectCounts, ChunkSoAView, CollisionProxyCache, DoorSoABlock, EnemySpawnSoABlock,
    RectSoABlock, TextPointSoABlock, TriggerSoABlock, WorldChunk, WorldPortalSoABlock,
};
pub use error::{LevelPackageError, PackageResult};
pub use manifest::LevelManifest;
#[allow(unused_imports)]
pub use runtime::{
    AssetHandle, AuthoringWorld, CheckpointIndex, CheckpointRuntime, CheckpointSnapshot,
    ChunkResidentSet, GroupIndex, LevelAssetId, LevelAssetResolver, MusicCue, ObjectHandle,
    PortalIndex, RenderVisibleSet, RuntimeChunkSets, RuntimeChunkSummary, SemanticRegion,
    SemanticRegionIndex, SimulationInterestSet, TriggerIndex, TriggerRuntimeEntry,
    UserPortalRuntime, WorldAabb, WorldPortalRuntime, WorldSpatialIndex,
};
pub use text_format::{CheckpointAnchor, WorldChunkEntry, WorldIndex, WorldPortalAnchor};

use binio::{decode_chunk, encode_chunk};
use chunk::{compile_package, package_to_level};
use manifest::{format_manifest, parse_manifest, stable_level_id};
use quant::{
    chunk_bounds_for_key, chunk_bounds_units, chunk_key_for_world, chunk_key_for_world_runtime,
    chunk_origin_for_key, dequant_local_point, dequant_rotation, dequant_u16_units,
    dequant_unit_vec, quant_local_aabb, quant_local_point, quant_rotation, quant_size,
    quant_u16_fixed, quant_u16_units, quant_unit,
};
use security::{
    read_package_bytes, read_package_string, validate_package_path, write_package_bytes,
    write_package_string,
};
use sha256::sha256_hex;
use text_format::{
    format_checkpoints, format_debug_level, format_groups, format_layers, format_portals,
    format_source_level, format_triggers, format_world_index, parse_world_index,
};

use super::LevelSpec;
pub(super) use super::{parse_key_values, quote};

pub const PACKAGE_SCHEMA: u8 = 4;
pub const COORD_SCALE: i32 = 16;
pub const CHUNK_SIZE_UNITS: i32 = 2048;
pub const CHUNK_SIZE_Q: i32 = CHUNK_SIZE_UNITS * COORD_SCALE;
pub const DEFAULT_RENDER_GUARD_UNITS: f32 = 256.0;
pub const DEFAULT_PLAYER_INTEREST_MARGIN_UNITS: f32 = 768.0;
pub const DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS: f32 = 512.0;

const MANIFEST_PATH: &str = "manifest.toml";
const WORLD_INDEX_PATH: &str = "world.index";
const WORLD_LAYERS_PATH: &str = "world.layers";
const WORLD_GROUPS_PATH: &str = "world.groups";
const WORLD_TRIGGERS_PATH: &str = "world.triggers";
const WORLD_CHECKPOINTS_PATH: &str = "world.checkpoints";
const WORLD_PORTALS_PATH: &str = "world.portals";
const WORLD_SOURCE_PATH: &str = "world.source.level";
const WORLD_DEBUG_PATH: &str = "world.debug.level";
const CHUNK_DIR: &str = "world.chunks";
const CHUNK_MAGIC: &[u8; 8] = b"WCHUNK4\0";
const DEFAULT_VIEW_LAYER: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LevelPackageKind {
    LegacyV3,
    PackageV4,
}

#[derive(Clone, Debug)]
pub struct LevelPackage {
    pub manifest: LevelManifest,
    pub index: WorldIndex,
    pub chunks: Vec<WorldChunk>,
}

pub(super) fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub(super) struct LevelPackageReader;

pub(super) struct LevelPackageWriter;

impl LevelPackageReader {
    pub(super) fn detect(path: &Path) -> io::Result<LevelPackageKind> {
        if path.is_dir() {
            let manifest = path.join(MANIFEST_PATH);
            if manifest.is_file() {
                return Ok(LevelPackageKind::PackageV4);
            }

            return Err(io::Error::from(LevelPackageError::InvalidData(
                "missing v4 manifest.toml".to_string(),
            )));
        }

        let source = fs::read_to_string(path)?;
        if source.lines().next().is_some_and(|line| {
            line.trim().starts_with(&format!(
                "portals_level version={}",
                super::LEVEL_FORMAT_VERSION
            ))
        }) {
            return Ok(LevelPackageKind::LegacyV3);
        }

        Err(io::Error::from(LevelPackageError::InvalidData(
            "unknown .lvl package header".to_string(),
        )))
    }

    pub(super) fn read(path: &Path) -> io::Result<LevelSpec> {
        let package = Self::read_package(path).map_err(io::Error::from)?;
        let mut spec = package_to_level(&package).map_err(io::Error::from)?;

        spec.path = Some(path.to_path_buf());
        Ok(spec)
    }

    pub fn read_package(path: &Path) -> PackageResult<LevelPackage> {
        let manifest_text = read_package_string(path, MANIFEST_PATH)?;
        let manifest = parse_manifest(&manifest_text)?;
        if manifest.entry != WORLD_INDEX_PATH {
            return Err(LevelPackageError::UnsupportedEntry {
                entry: manifest.entry,
            });
        }

        let index_text = read_package_string(path, &manifest.entry)?;
        let mut index = parse_world_index(&index_text)?;
        if index.schema != PACKAGE_SCHEMA {
            return Err(LevelPackageError::UnsupportedSchema {
                schema: index.schema,
            });
        }
        if index.coord_scale != manifest.coord_scale
            || index.chunk_size_units != manifest.chunk_size_units
        {
            return Err(LevelPackageError::InvalidData(
                "manifest and world.index coordinate settings disagree".to_string(),
            ));
        }

        let mut chunks = Vec::with_capacity(index.chunks.len());
        for entry in &mut index.chunks {
            validate_package_path(&entry.path)?;
            let bytes = read_package_bytes(path, &entry.path)?;
            if bytes.len() as u64 != entry.bytes {
                return Err(LevelPackageError::ChunkSizeMismatch { chunk_id: entry.id });
            }
            let actual_sha = sha256_hex(&bytes);
            if actual_sha != entry.sha256 {
                return Err(LevelPackageError::ChunkChecksumMismatch { chunk_id: entry.id });
            }
            let chunk = decode_chunk(&bytes)?;
            if chunk.chunk_id != entry.id || chunk.origin_q != entry.origin_q {
                return Err(LevelPackageError::ChunkHeaderMismatch { chunk_id: entry.id });
            }
            if chunk.counts() != entry.counts {
                return Err(LevelPackageError::ChunkCountsMismatch { chunk_id: entry.id });
            }
            chunks.push(chunk);
        }

        Ok(LevelPackage {
            manifest,
            index,
            chunks,
        })
    }
}

impl LevelPackageWriter {
    pub(super) fn should_write_v4(level: &LevelSpec) -> bool {
        level
            .path
            .as_ref()
            .is_none_or(|path| path.is_dir() || path.join(MANIFEST_PATH).is_file())
    }

    pub(super) fn write_directory(path: &Path, level: &LevelSpec) -> io::Result<()> {
        let package = compile_package(level).map_err(io::Error::from)?;

        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join(CHUNK_DIR))?;
        write_package_string(path, MANIFEST_PATH, &format_manifest(&package.manifest))?;
        write_package_string(path, WORLD_INDEX_PATH, &format_world_index(&package.index))?;
        write_package_string(path, WORLD_LAYERS_PATH, &format_layers(level))?;
        write_package_string(path, WORLD_GROUPS_PATH, &format_groups(level))?;
        write_package_string(path, WORLD_TRIGGERS_PATH, &format_triggers(level))?;
        write_package_string(path, WORLD_CHECKPOINTS_PATH, &format_checkpoints(level))?;
        write_package_string(path, WORLD_PORTALS_PATH, &format_portals(level))?;
        write_package_string(path, WORLD_SOURCE_PATH, &format_source_level(level))?;
        write_package_string(path, WORLD_DEBUG_PATH, &format_debug_level(level))?;

        for (chunk, entry) in package.chunks.iter().zip(&package.index.chunks) {
            write_package_bytes(path, &entry.path, &encode_chunk(chunk))?;
        }

        Ok(())
    }
}
