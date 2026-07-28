use crate::game::level::WorldPortal;
use crate::game::portal::Portal;

#[derive(Clone, Copy)]
pub(in crate::game) struct PortalLink {
    pub source: Portal,
    pub destination: Portal,
}

pub(in crate::game) fn portal_links(
    player_portals: [Option<Portal>; 2],
    world_portals: &[WorldPortal],
) -> Vec<PortalLink> {
    let mut links = Vec::new();

    if let [Some(source), Some(destination)] = player_portals {
        links.push(PortalLink {
            source,
            destination,
        });
        links.push(PortalLink {
            source: destination,
            destination: source,
        });
    }

    links.extend(
        world_portals
            .iter()
            .enumerate()
            .filter_map(|(source_index, source)| {
                let destination_index = WorldPortal::receiver_index(world_portals, source_index)?;
                let destination = world_portals.get(destination_index)?;

                Some(PortalLink {
                    source: source.portal,
                    destination: destination.portal,
                })
            }),
    );

    links
}

pub(in crate::game) fn collision_portals(links: &[PortalLink]) -> Vec<Portal> {
    links.iter().map(|link| link.source).collect()
}
