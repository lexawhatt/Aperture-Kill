use glam::Vec2;

pub fn projected_extent(half_size: Vec2, axis: Vec2) -> f32 {
    half_size.x * axis.x.abs() + half_size.y * axis.y.abs()
}
