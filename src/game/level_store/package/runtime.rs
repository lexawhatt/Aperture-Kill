use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;

use super::{
    DEFAULT_PLAYER_INTEREST_MARGIN_UNITS, DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS,
    DEFAULT_RENDER_GUARD_UNITS, LevelPackage, LevelPackageError, PackageResult, WorldChunk,
    WorldIndex, chunk_bounds_for_key, chunk_key_for_world_runtime, chunk_origin_for_key,
    dequant_local_point, dequant_u16_units, dequant_unit_vec,
};
use crate::game::level::{Level, LevelObjectKind, Solid, WorldPortal};
use crate::game::portal::{Color, Portal};

#[derive(Clone, Debug, Default)]
pub struct GroupIndex {
    pub groups: BTreeMap<u16, Vec<ObjectHandle>>,
}

#[derive(Clone, Debug, Default)]
pub struct TriggerIndex {
    pub triggers: Vec<TriggerRuntimeEntry>,
}

#[derive(Clone, Default)]
pub struct PortalIndex {
    pub world_portals: Vec<WorldPortalRuntime>,
    pub user_portals: Vec<UserPortalRuntime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectHandle {
    pub chunk_id: u32,
    pub kind: LevelObjectKind,
    pub index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerRuntimeEntry {
    pub trigger_id: u16,
    pub source_chunk: u32,
    pub target_group: u16,
    pub kind: u8,
}

#[derive(Clone, Copy, PartialEq)]
pub struct UserPortalRuntime {
    pub source_anchor: Portal,
    pub destination_anchor: Portal,
    pub traversal_width: f32,
    pub physics_margin: f32,
    pub render_remote_scene: bool,
    pub pass_lighting: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub struct WorldPortalRuntime {
    pub source: WorldPortal,
    pub source_chunk: u32,
    pub receiver_id: u16,
    pub render: bool,
    pub lighting: bool,
    pub physics: bool,
    pub render_depth: u8,
    pub physics_margin: f32,
    pub light_depth: u8,
}

#[derive(Clone, Debug, Default)]
pub struct CheckpointIndex {
    pub checkpoints: Vec<CheckpointRuntime>,
}

#[derive(Clone, Debug, Default)]
pub struct CheckpointSnapshot {
    pub object_state_handles: Vec<u64>,
    pub persistent_flags: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckpointRuntime {
    pub spawn: Vec2,
    pub chunk_id: u32,
    pub radius: Vec2,
    pub reusable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SemanticRegionIndex {
    pub regions: Vec<SemanticRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticRegion {
    pub id: u32,
    pub kind: String,
    pub name: String,
    pub bounds: WorldAabb,
}

#[derive(Clone, Debug, Default)]
pub struct AuthoringWorld {
    pub source_level: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelAssetId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetHandle {
    pub id: LevelAssetId,
}

#[derive(Clone, Debug, Default)]
pub struct LevelAssetResolver {
    pub assets: BTreeMap<LevelAssetId, AssetHandle>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicCue {
    pub id: LevelAssetId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldAabb {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct WorldSpatialIndex {
    pub chunks: Vec<RuntimeChunkSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeChunkSummary {
    pub chunk_id: u32,
    pub origin_q: [i32; 2],
    pub bounds: WorldAabb,
}

#[derive(Clone, Debug, Default)]
pub struct RenderVisibleSet {
    pub world_rect: WorldAabb,
    pub guard_units: f32,
    pub chunks: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct SimulationInterestSet {
    pub volumes: Vec<WorldAabb>,
    pub chunks: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct ChunkResidentSet {
    pub chunks: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeChunkSets {
    pub render_visible: RenderVisibleSet,
    pub simulation_interest: SimulationInterestSet,
    pub resident: ChunkResidentSet,
}
impl WorldAabb {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size / 2.0;

        Self::new(center - half, center + half)
    }

    pub fn from_camera(camera: Vec2, zoom: f32, width: f32, height: f32) -> Self {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        let half = Vec2::new(width.max(1.0), height.max(1.0)) / (2.0 * zoom);

        Self::new(camera - half, camera + half)
    }

    pub fn expanded(self, amount: f32) -> Self {
        let amount = if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        };

        Self::new(
            self.min - Vec2::splat(amount),
            self.max + Vec2::splat(amount),
        )
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn contains_point(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn from_solid(solid: Solid) -> Self {
        let (min, max) = solid.bounds();

        Self::new(min, max)
    }
}

impl GroupIndex {
    pub fn from_chunks(chunks: &[WorldChunk]) -> Self {
        let mut index = Self::default();

        for chunk in chunks {
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Solid,
                &chunk.static_rects.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Hazard,
                &chunk.hazard_rects.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Door,
                &chunk.doors.rects.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Checkpoint,
                &chunk.checkpoints.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Trigger,
                &chunk.triggers.rects.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Enemy,
                &chunk.enemy_spawns.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::Text,
                &chunk.text_points.meta,
            );
            index.add_rect_block(
                chunk.chunk_id,
                LevelObjectKind::WorldPortal,
                &chunk.world_portals.meta,
            );
        }

        index
    }

    pub fn handles(&self, group_id: u16) -> &[ObjectHandle] {
        self.groups.get(&group_id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn contains_group(&self, group_id: u16) -> bool {
        self.groups.contains_key(&group_id)
    }

    fn add_rect_block(&mut self, chunk_id: u32, kind: LevelObjectKind, metas: &[u16]) {
        for (index, meta) in metas.iter().copied().enumerate() {
            let group_id = group_id_from_surface_meta(meta);
            if group_id == 0 {
                continue;
            }

            self.groups.entry(group_id).or_default().push(ObjectHandle {
                chunk_id,
                kind,
                index: index as u32,
            });
        }
    }
}

impl TriggerIndex {
    pub fn from_package(package: &LevelPackage) -> PackageResult<Self> {
        Self::from_world_index(&package.index)
    }

    pub fn from_world_index(index: &WorldIndex) -> PackageResult<Self> {
        let mut seen = BTreeSet::new();
        let mut triggers = Vec::with_capacity(index.triggers.len());

        for trigger in &index.triggers {
            if !seen.insert(trigger.trigger_id) {
                return Err(LevelPackageError::InvalidData(format!(
                    "duplicate trigger_id {} in world.index",
                    trigger.trigger_id
                )));
            }

            triggers.push(TriggerRuntimeEntry {
                trigger_id: trigger.trigger_id,
                source_chunk: trigger.source_chunk,
                target_group: trigger.target_group,
                kind: trigger.kind,
            });
        }

        Ok(Self { triggers })
    }

    pub fn for_target_group(
        &self,
        target_group: u16,
    ) -> impl Iterator<Item = &TriggerRuntimeEntry> {
        self.triggers
            .iter()
            .filter(move |trigger| target_group != 0 && trigger.target_group == target_group)
    }
}

impl PortalIndex {
    pub fn from_package(package: &LevelPackage) -> PackageResult<Self> {
        Self::from_package_and_user_portals(package, &[None, None])
    }

    pub fn from_package_and_user_portals(
        package: &LevelPackage,
        portals: &[Option<Portal>; 2],
    ) -> PackageResult<Self> {
        Ok(Self {
            world_portals: world_portal_runtimes_from_package(package)?,
            user_portals: user_portal_runtimes(portals),
        })
    }

    pub fn from_user_portals(portals: &[Option<Portal>; 2]) -> Self {
        Self {
            world_portals: Vec::new(),
            user_portals: user_portal_runtimes(portals),
        }
    }

    pub fn world_portal_by_id(&self, portal_id: u16) -> Option<&WorldPortalRuntime> {
        self.world_portals
            .iter()
            .find(|portal| portal.source.id == portal_id)
    }
}

impl WorldSpatialIndex {
    pub fn from_level(level: &Level) -> Self {
        let mut chunks = BTreeMap::<(i32, i32), WorldAabb>::new();
        collect_level_aabbs(level, |aabb| {
            let key = chunk_key_for_world_runtime(point_to_assign(aabb));
            chunks
                .entry(key)
                .and_modify(|existing| {
                    existing.min = existing.min.min(aabb.min);
                    existing.max = existing.max.max(aabb.max);
                })
                .or_insert(aabb);
        });

        if chunks.is_empty() {
            chunks.insert((0, 0), chunk_bounds_for_key((0, 0)));
        }

        let chunks = chunks
            .into_iter()
            .enumerate()
            .map(|(index, (key, bounds))| RuntimeChunkSummary {
                chunk_id: index as u32,
                origin_q: chunk_origin_for_key(key),
                bounds,
            })
            .collect();

        Self { chunks }
    }

    pub fn query_chunks(&self, rect: WorldAabb) -> Vec<u32> {
        self.chunks
            .iter()
            .filter(|chunk| chunk.bounds.overlaps(rect))
            .map(|chunk| chunk.chunk_id)
            .collect()
    }
}

fn group_id_from_surface_meta(meta: u16) -> u16 {
    meta >> 6
}

fn world_portal_runtimes_from_package(
    package: &LevelPackage,
) -> PackageResult<Vec<WorldPortalRuntime>> {
    package
        .index
        .portals
        .iter()
        .map(|anchor| {
            let mut source =
                world_portal_from_package(package, anchor.portal_id, anchor.source_chunk)?;
            source.receiver_id = anchor.receiver_id;

            Ok(WorldPortalRuntime {
                source,
                source_chunk: anchor.source_chunk,
                receiver_id: anchor.receiver_id,
                render: anchor.render,
                lighting: anchor.lighting,
                physics: anchor.physics,
                render_depth: u8::from(anchor.render),
                physics_margin: if anchor.physics {
                    DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS
                } else {
                    0.0
                },
                light_depth: u8::from(anchor.lighting),
            })
        })
        .collect()
}

fn world_portal_from_package(
    package: &LevelPackage,
    portal_id: u16,
    source_chunk: u32,
) -> PackageResult<WorldPortal> {
    let chunk = package
        .chunks
        .iter()
        .find(|chunk| chunk.chunk_id == source_chunk)
        .ok_or_else(|| {
            LevelPackageError::InvalidData(format!(
                "world portal {portal_id} references missing source chunk {source_chunk}"
            ))
        })?;

    let index = (0..chunk.world_portals.x.len())
        .find(|index| chunk.world_portals.portal_id[*index] == portal_id)
        .ok_or_else(|| {
            LevelPackageError::InvalidData(format!(
                "world portal {portal_id} not found in source chunk {source_chunk}"
            ))
        })?;

    Ok(world_portal_from_chunk(chunk, index))
}

fn world_portal_from_chunk(chunk: &WorldChunk, index: usize) -> WorldPortal {
    let pos = dequant_local_point(
        chunk.origin_q,
        chunk.world_portals.x[index],
        chunk.world_portals.y[index],
    );
    let normal = dequant_unit_vec(
        chunk.world_portals.normal_x[index],
        chunk.world_portals.normal_y[index],
    );
    let tangent = dequant_unit_vec(
        chunk.world_portals.tangent_x[index],
        chunk.world_portals.tangent_y[index],
    );
    let mut portal = Portal::with_tangent(
        pos.x,
        pos.y,
        normal,
        tangent,
        dequant_u16_units(chunk.world_portals.width[index]),
        Color::rgb(154, 120, 255),
    );
    let flags = chunk.world_portals.flags[index];

    portal.scale = (chunk.world_portals.scale[index] as f32 / 256.0).max(0.01);
    portal.scale_objects = flags & 0b0000_0001 != 0;

    WorldPortal {
        portal,
        id: chunk.world_portals.portal_id[index],
        receiver_id: chunk.world_portals.receiver_id[index],
        priority: chunk.world_portals.priority[index],
        seamless: flags & 0b0000_0010 != 0,
        seamless_depth: dequant_u16_units(chunk.world_portals.seamless_depth[index]),
        seamless_angle: chunk.world_portals.seamless_angle[index] as f32 / 10.0,
        seamless_rely_on_walls: flags & 0b0000_0100 != 0,
    }
}

fn user_portal_runtimes(portals: &[Option<Portal>; 2]) -> Vec<UserPortalRuntime> {
    let [Some(first), Some(second)] = *portals else {
        return Vec::new();
    };

    vec![
        user_portal_runtime(first, second),
        user_portal_runtime(second, first),
    ]
}

fn user_portal_runtime(source: Portal, destination: Portal) -> UserPortalRuntime {
    UserPortalRuntime {
        source_anchor: source,
        destination_anchor: destination,
        traversal_width: source.active_width(),
        physics_margin: DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS,
        render_remote_scene: false,
        pass_lighting: false,
    }
}

impl RenderVisibleSet {
    pub fn from_camera(
        level: &Level,
        camera: Vec2,
        zoom: f32,
        width: f32,
        height: f32,
        guard_units: f32,
    ) -> Self {
        let world_rect = WorldAabb::from_camera(camera, zoom, width, height).expanded(guard_units);
        let spatial = WorldSpatialIndex::from_level(level);
        let chunks = spatial.query_chunks(world_rect);

        Self {
            world_rect,
            guard_units,
            chunks,
        }
    }

    pub fn contains_solid(&self, solid: Solid) -> bool {
        self.world_rect.overlaps(WorldAabb::from_solid(solid))
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        self.world_rect.contains_point(point)
    }
}

impl SimulationInterestSet {
    pub fn from_player(level: &Level, player_pos: Vec2, portals: &[Option<Portal>; 2]) -> Self {
        let mut volumes = vec![WorldAabb::from_center_size(
            player_pos,
            Vec2::splat(DEFAULT_PLAYER_INTEREST_MARGIN_UNITS * 2.0),
        )];

        for portal in portals.iter().flatten() {
            volumes.push(WorldAabb::from_center_size(
                portal.pos,
                Vec2::splat(DEFAULT_PORTAL_PHYSICS_MARGIN_UNITS * 2.0),
            ));
        }

        let spatial = WorldSpatialIndex::from_level(level);
        let mut chunks = BTreeSet::new();
        for volume in &volumes {
            chunks.extend(spatial.query_chunks(*volume));
        }

        Self {
            volumes,
            chunks: chunks.into_iter().collect(),
        }
    }
}

impl ChunkResidentSet {
    pub fn union(render: &RenderVisibleSet, simulation: &SimulationInterestSet) -> Self {
        let mut chunks = BTreeSet::new();

        chunks.extend(render.chunks.iter().copied());
        chunks.extend(simulation.chunks.iter().copied());

        Self {
            chunks: chunks.into_iter().collect(),
        }
    }
}

impl RuntimeChunkSets {
    pub fn from_camera_and_player(
        level: &Level,
        camera: Vec2,
        zoom: f32,
        width: f32,
        height: f32,
        player_pos: Vec2,
        portals: &[Option<Portal>; 2],
    ) -> Self {
        let render_visible = RenderVisibleSet::from_camera(
            level,
            camera,
            zoom,
            width,
            height,
            DEFAULT_RENDER_GUARD_UNITS,
        );
        let simulation_interest = SimulationInterestSet::from_player(level, player_pos, portals);
        let resident = ChunkResidentSet::union(&render_visible, &simulation_interest);

        Self {
            render_visible,
            simulation_interest,
            resident,
        }
    }
}
fn collect_level_aabbs(level: &Level, mut f: impl FnMut(WorldAabb)) {
    for solid in &level.solids {
        f(WorldAabb::from_solid(*solid));
    }
    for hazard in &level.hazards {
        f(WorldAabb::from_solid(hazard.solid));
    }
    for door in &level.doors {
        f(WorldAabb::from_solid(door.solid));
    }
    for checkpoint in &level.checkpoints {
        f(WorldAabb::from_solid(checkpoint.solid));
    }
    for trigger in &level.triggers {
        f(WorldAabb::from_solid(trigger.solid));
    }
    for enemy in &level.enemies {
        f(WorldAabb::from_solid(enemy.spawn_solid()));
    }
    for text in &level.texts {
        f(WorldAabb::from_center_size(text.pos, Vec2::splat(1.0)));
    }
    for portal in &level.world_portals {
        f(WorldAabb::from_solid(portal.edit_solid()));
    }
}

fn point_to_assign(aabb: WorldAabb) -> Vec2 {
    (aabb.min + aabb.max) / 2.0
}
