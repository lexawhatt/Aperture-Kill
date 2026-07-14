use glam::Vec2;

pub(super) fn menu_hit(pos: Vec2, count: usize) -> Option<usize> {
    let item_height = 42.0;
    let x = 280.0;
    let y = 150.0;
    let width = 340.0;

    if pos.x < x || pos.x > x + width || pos.y < y {
        return None;
    }

    let index = ((pos.y - y) / item_height).floor() as usize;
    (index < count).then_some(index)
}
