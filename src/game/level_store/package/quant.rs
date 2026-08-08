use glam::Vec2;

use super::{
    CHUNK_SIZE_Q, CHUNK_SIZE_UNITS, COORD_SCALE, LevelPackageError, PackageResult, WorldAabb,
    error::invalid_data,
};

pub(super) fn quant(value: f32, name: &str) -> PackageResult<i32> {
    if !value.is_finite() {
        return Err(invalid_data(format!("{name} is not finite")));
    }
    let scaled = (value * COORD_SCALE as f32).round();
    if scaled < i32::MIN as f32 || scaled > i32::MAX as f32 {
        return Err(LevelPackageError::CoordinateOverflow {
            name: name.to_string(),
        });
    }

    Ok(scaled as i32)
}

pub(super) fn quant_local_point(origin_q: [i32; 2], point: Vec2) -> PackageResult<[i16; 2]> {
    let x = quant(point.x, "x")? - origin_q[0];
    let y = quant(point.y, "y")? - origin_q[1];

    Ok([
        i16::try_from(x).map_err(|_| LevelPackageError::CoordinateOutOfRange {
            name: "x".to_string(),
            storage: "chunk-local i16",
        })?,
        i16::try_from(y).map_err(|_| LevelPackageError::CoordinateOutOfRange {
            name: "y".to_string(),
            storage: "chunk-local i16",
        })?,
    ])
}

pub(super) fn quant_local_aabb(origin_q: [i32; 2], bounds: WorldAabb) -> PackageResult<[i16; 4]> {
    let min = quant_local_point(origin_q, bounds.min)?;
    let max = quant_local_point(origin_q, bounds.max)?;

    Ok([min[0], min[1], max[0], max[1]])
}

pub(super) fn quant_size(size: Vec2) -> PackageResult<[u16; 2]> {
    if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
        return Err(invalid_data("invalid rect size"));
    }

    Ok([
        quant_u16_units(size.x, "width")?,
        quant_u16_units(size.y, "height")?,
    ])
}

pub(super) fn quant_u16_units(value: f32, name: &str) -> PackageResult<u16> {
    let value = quant(value, name)?;

    u16::try_from(value).map_err(|_| LevelPackageError::CoordinateOutOfRange {
        name: name.to_string(),
        storage: "u16",
    })
}

pub(super) fn quant_u16_fixed(value: f32, scale: f32, name: &str) -> PackageResult<u16> {
    if !value.is_finite() || value < 0.0 {
        return Err(invalid_data(format!("{name} is invalid")));
    }
    let value = (value * scale).round();
    if value > u16::MAX as f32 {
        return Err(LevelPackageError::CoordinateOutOfRange {
            name: name.to_string(),
            storage: "u16 fixed-point",
        });
    }

    Ok(value as u16)
}

pub(super) fn dequant_local_point(origin_q: [i32; 2], x: i16, y: i16) -> Vec2 {
    Vec2::new(
        (origin_q[0] + i32::from(x)) as f32 / COORD_SCALE as f32,
        (origin_q[1] + i32::from(y)) as f32 / COORD_SCALE as f32,
    )
}

pub(super) fn dequant_u16_units(value: u16) -> f32 {
    value as f32 / COORD_SCALE as f32
}

pub(super) fn quant_rotation(rotation: f32) -> i16 {
    if !rotation.is_finite() {
        return 0;
    }

    let normalized =
        (rotation + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    (normalized / std::f32::consts::PI * i16::MAX as f32).round() as i16
}

pub(super) fn dequant_rotation(rotation: i16) -> f32 {
    rotation as f32 / i16::MAX as f32 * std::f32::consts::PI
}

pub(super) fn quant_unit(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

pub(super) fn dequant_unit_vec(x: i16, y: i16) -> Vec2 {
    Vec2::new(x as f32 / i16::MAX as f32, y as f32 / i16::MAX as f32).normalize_or_zero()
}

pub(super) fn chunk_key_for_world(point: Vec2) -> PackageResult<(i32, i32)> {
    let x = quant(point.x, "chunk key x")?.div_euclid(CHUNK_SIZE_Q);
    let y = quant(point.y, "chunk key y")?.div_euclid(CHUNK_SIZE_Q);

    Ok((x, y))
}

pub(super) fn chunk_key_for_world_runtime(point: Vec2) -> (i32, i32) {
    chunk_key_for_world(point).expect("runtime level coordinate must be finite and quantizable")
}

pub(super) fn chunk_origin_for_key(key: (i32, i32)) -> [i32; 2] {
    [
        key.0 * CHUNK_SIZE_Q + CHUNK_SIZE_Q / 2,
        key.1 * CHUNK_SIZE_Q + CHUNK_SIZE_Q / 2,
    ]
}

pub(super) fn chunk_bounds_for_key(key: (i32, i32)) -> WorldAabb {
    let origin = chunk_origin_for_key(key);
    let half = CHUNK_SIZE_UNITS as f32 / 2.0;
    let center = Vec2::new(
        origin[0] as f32 / COORD_SCALE as f32,
        origin[1] as f32 / COORD_SCALE as f32,
    );

    WorldAabb::from_center_size(center, Vec2::splat(half * 2.0))
}

pub(super) fn chunk_bounds_units(bounds: [i16; 4], origin_q: [i32; 2]) -> [f32; 4] {
    [
        (origin_q[0] + i32::from(bounds[0])) as f32 / COORD_SCALE as f32,
        (origin_q[1] + i32::from(bounds[1])) as f32 / COORD_SCALE as f32,
        (origin_q[0] + i32::from(bounds[2])) as f32 / COORD_SCALE as f32,
        (origin_q[1] + i32::from(bounds[3])) as f32 / COORD_SCALE as f32,
    ]
}
