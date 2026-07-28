mod both;
mod dynamic;
mod special;
mod r#static;

pub(super) use both::{PortalLink, collision_portals, portal_links};
pub(super) use dynamic::{enemy_targets, separate_enemies, teleport_enemy_through_portals};
