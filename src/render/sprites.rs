#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SpriteId {
    MenuV1,
    PiercerHud,
    DeathSkull1,
    DeathSkull2,
    DeathShutdownFlash,
    PlayerBase,
    EnemyWeak,
    PortalBlue,
    PortalOrange,
    DoorPanel,
    Hazard,
    Checkpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpriteUsage {
    Ui,
    Fullscreen,
    World,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpriteSource {
    RawRgba(&'static [u8]),
    PlannedSheet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpriteSize {
    pub(super) width: u32,
    pub(super) height: u32,
}

impl SpriteSize {
    pub(super) const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub(super) const fn square(size: u32) -> Self {
        Self::new(size, size)
    }

    pub(super) fn byte_len(self) -> usize {
        (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpriteRect {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl SpriteRect {
    const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn fits(self, size: SpriteSize) -> bool {
        self.x.saturating_add(self.width) <= size.width
            && self.y.saturating_add(self.height) <= size.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpriteFrame {
    pub(super) name: &'static str,
    pub(super) rect: SpriteRect,
    pub(super) duration_ms: u16,
}

impl SpriteFrame {
    const fn new(
        name: &'static str,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        duration_ms: u16,
    ) -> Self {
        Self {
            name,
            rect: SpriteRect::new(x, y, width, height),
            duration_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SpritePivot {
    pub(super) x: f32,
    pub(super) y: f32,
}

impl SpritePivot {
    const CENTER: Self = Self { x: 0.5, y: 0.5 };
    const TOP_LEFT: Self = Self { x: 0.0, y: 0.0 };
    const BOTTOM_CENTER: Self = Self { x: 0.5, y: 1.0 };
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SpriteAsset {
    id: SpriteId,
    label: &'static str,
    source_path: &'static str,
    source: SpriteSource,
    usage: SpriteUsage,
    size: SpriteSize,
    frames: &'static [SpriteFrame],
    pivot: SpritePivot,
}

impl SpriteAsset {
    const fn raw_rgba(
        id: SpriteId,
        label: &'static str,
        source_path: &'static str,
        bytes: &'static [u8],
        usage: SpriteUsage,
        size: SpriteSize,
        frames: &'static [SpriteFrame],
        pivot: SpritePivot,
    ) -> Self {
        Self {
            id,
            label,
            source_path,
            source: SpriteSource::RawRgba(bytes),
            usage,
            size,
            frames,
            pivot,
        }
    }

    const fn planned_sheet(
        id: SpriteId,
        label: &'static str,
        source_path: &'static str,
        usage: SpriteUsage,
        size: SpriteSize,
        frames: &'static [SpriteFrame],
        pivot: SpritePivot,
    ) -> Self {
        Self {
            id,
            label,
            source_path,
            source: SpriteSource::PlannedSheet,
            usage,
            size,
            frames,
            pivot,
        }
    }

    pub(super) fn id(self) -> SpriteId {
        self.id
    }

    pub(super) fn label(self) -> &'static str {
        self.label
    }

    pub(super) fn size(self) -> SpriteSize {
        self.size
    }

    pub(super) fn frames(self) -> &'static [SpriteFrame] {
        self.frames
    }

    pub(super) fn raw_bytes(self) -> Option<&'static [u8]> {
        match self.source {
            SpriteSource::RawRgba(bytes) => Some(bytes),
            SpriteSource::PlannedSheet => None,
        }
    }

    fn manifest_score(self) -> usize {
        let source_score = match self.source {
            SpriteSource::RawRgba(bytes) => bytes.len(),
            SpriteSource::PlannedSheet => 0,
        };
        let usage_score = match self.usage {
            SpriteUsage::Ui => 11,
            SpriteUsage::Fullscreen => 17,
            SpriteUsage::World => 23,
        };
        let frame_score = self.frames.iter().fold(0usize, |acc, frame| {
            acc ^ frame.name.len()
                ^ frame.rect.x as usize
                ^ frame.rect.y as usize
                ^ frame.rect.width as usize
                ^ frame.rect.height as usize
                ^ frame.duration_ms as usize
        });

        self.label.len()
            ^ self.source_path.len()
            ^ source_score
            ^ usage_score
            ^ self.size.width as usize
            ^ self.size.height as usize
            ^ ((self.pivot.x * 1000.0) as usize)
            ^ ((self.pivot.y * 1000.0) as usize)
            ^ frame_score
    }
}

const MENU_V1_FRAMES: [SpriteFrame; 1] =
    [SpriteFrame::new("menu.machine.single", 0, 0, 760, 760, 0)];
const PIERCER_HUD_FRAMES: [SpriteFrame; 1] =
    [SpriteFrame::new("hud.piercer.single", 0, 0, 550, 268, 0)];
const DEATH_SKULL_1_FRAMES: [SpriteFrame; 1] =
    [SpriteFrame::new("death.skull.open", 0, 0, 1637, 1636, 120)];
const DEATH_SKULL_2_FRAMES: [SpriteFrame; 1] = [SpriteFrame::new(
    "death.skull.closed",
    0,
    0,
    1637,
    1636,
    120,
)];
const DEATH_SHUTDOWN_FLASH_FRAMES: [SpriteFrame; 1] =
    [SpriteFrame::new("death.shutdown.flash", 0, 0, 480, 270, 0)];

const PLAYER_BASE_FRAMES: [SpriteFrame; 14] = [
    SpriteFrame::new("idle.0", 0, 0, 96, 96, 120),
    SpriteFrame::new("idle.1", 96, 0, 96, 96, 120),
    SpriteFrame::new("run.0", 0, 96, 96, 96, 75),
    SpriteFrame::new("run.1", 96, 96, 96, 96, 75),
    SpriteFrame::new("run.2", 192, 96, 96, 96, 75),
    SpriteFrame::new("run.3", 288, 96, 96, 96, 75),
    SpriteFrame::new("jump", 0, 192, 96, 96, 0),
    SpriteFrame::new("fall", 96, 192, 96, 96, 0),
    SpriteFrame::new("dash.0", 0, 288, 128, 96, 45),
    SpriteFrame::new("dash.1", 128, 288, 128, 96, 45),
    SpriteFrame::new("slide.0", 0, 384, 128, 80, 70),
    SpriteFrame::new("slide.1", 128, 384, 128, 80, 70),
    SpriteFrame::new("slam.0", 0, 464, 128, 128, 80),
    SpriteFrame::new("slam.1", 128, 464, 128, 128, 80),
];

const ENEMY_WEAK_FRAMES: [SpriteFrame; 11] = [
    SpriteFrame::new("idle.0", 0, 0, 96, 96, 140),
    SpriteFrame::new("idle.1", 96, 0, 96, 96, 140),
    SpriteFrame::new("walk.0", 0, 96, 96, 96, 95),
    SpriteFrame::new("walk.1", 96, 96, 96, 96, 95),
    SpriteFrame::new("walk.2", 192, 96, 96, 96, 95),
    SpriteFrame::new("walk.3", 288, 96, 96, 96, 95),
    SpriteFrame::new("attack.0", 0, 192, 128, 96, 80),
    SpriteFrame::new("attack.1", 128, 192, 128, 96, 80),
    SpriteFrame::new("death.0", 0, 288, 128, 96, 90),
    SpriteFrame::new("death.1", 128, 288, 128, 96, 90),
    SpriteFrame::new("death.2", 256, 288, 128, 96, 120),
];

const PORTAL_FRAMES: [SpriteFrame; 4] = [
    SpriteFrame::new("open.0", 0, 0, 64, 160, 60),
    SpriteFrame::new("open.1", 64, 0, 64, 160, 60),
    SpriteFrame::new("idle.0", 128, 0, 64, 160, 90),
    SpriteFrame::new("idle.1", 192, 0, 64, 160, 90),
];

const WORLD_PROP_FRAMES: [SpriteFrame; 1] = [SpriteFrame::new("single", 0, 0, 128, 128, 0)];

pub(super) const MENU_V1: SpriteAsset = SpriteAsset::raw_rgba(
    SpriteId::MenuV1,
    "menu_v1",
    "assets/images/menu_v1.rgba",
    include_bytes!("../../assets/images/menu_v1.rgba"),
    SpriteUsage::Ui,
    SpriteSize::square(760),
    &MENU_V1_FRAMES,
    SpritePivot::CENTER,
);

pub(super) const PIERCER_HUD: SpriteAsset = SpriteAsset::raw_rgba(
    SpriteId::PiercerHud,
    "piercer_hud",
    "assets/images/hud/PiercerHUDNew.rgba",
    include_bytes!("../../assets/images/hud/PiercerHUDNew.rgba"),
    SpriteUsage::Ui,
    SpriteSize::new(550, 268),
    &PIERCER_HUD_FRAMES,
    SpritePivot::TOP_LEFT,
);

pub(super) const DEATH_SKULL_1: SpriteAsset = SpriteAsset::raw_rgba(
    SpriteId::DeathSkull1,
    "death_skull_1",
    "assets/images/death/DeathScreenSkull1.rgba",
    include_bytes!("../../assets/images/death/DeathScreenSkull1.rgba"),
    SpriteUsage::Ui,
    SpriteSize::new(1637, 1636),
    &DEATH_SKULL_1_FRAMES,
    SpritePivot::CENTER,
);

pub(super) const DEATH_SKULL_2: SpriteAsset = SpriteAsset::raw_rgba(
    SpriteId::DeathSkull2,
    "death_skull_2",
    "assets/images/death/DeathScreenSkull2.rgba",
    include_bytes!("../../assets/images/death/DeathScreenSkull2.rgba"),
    SpriteUsage::Ui,
    SpriteSize::new(1637, 1636),
    &DEATH_SKULL_2_FRAMES,
    SpritePivot::CENTER,
);

pub(super) const DEATH_SHUTDOWN_FLASH: SpriteAsset = SpriteAsset::raw_rgba(
    SpriteId::DeathShutdownFlash,
    "death_shutdown_flash",
    "assets/images/death/ISeeYou.rgba",
    include_bytes!("../../assets/images/death/ISeeYou.rgba"),
    SpriteUsage::Fullscreen,
    SpriteSize::new(480, 270),
    &DEATH_SHUTDOWN_FLASH_FRAMES,
    SpritePivot::TOP_LEFT,
);

const PLAYER_BASE: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::PlayerBase,
    "player_base",
    "assets/images/sprites/player_base.rgba",
    SpriteUsage::World,
    SpriteSize::new(512, 640),
    &PLAYER_BASE_FRAMES,
    SpritePivot::BOTTOM_CENTER,
);

const ENEMY_WEAK: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::EnemyWeak,
    "enemy_weak",
    "assets/images/sprites/enemy_weak.rgba",
    SpriteUsage::World,
    SpriteSize::new(512, 384),
    &ENEMY_WEAK_FRAMES,
    SpritePivot::BOTTOM_CENTER,
);

const PORTAL_BLUE: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::PortalBlue,
    "portal_blue",
    "assets/images/sprites/portal_blue.rgba",
    SpriteUsage::World,
    SpriteSize::new(256, 160),
    &PORTAL_FRAMES,
    SpritePivot::CENTER,
);

const PORTAL_ORANGE: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::PortalOrange,
    "portal_orange",
    "assets/images/sprites/portal_orange.rgba",
    SpriteUsage::World,
    SpriteSize::new(256, 160),
    &PORTAL_FRAMES,
    SpritePivot::CENTER,
);

const DOOR_PANEL: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::DoorPanel,
    "door_panel",
    "assets/images/sprites/door_panel.rgba",
    SpriteUsage::World,
    SpriteSize::new(128, 128),
    &WORLD_PROP_FRAMES,
    SpritePivot::CENTER,
);

const HAZARD: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::Hazard,
    "hazard",
    "assets/images/sprites/hazard.rgba",
    SpriteUsage::World,
    SpriteSize::new(128, 128),
    &WORLD_PROP_FRAMES,
    SpritePivot::CENTER,
);

const CHECKPOINT: SpriteAsset = SpriteAsset::planned_sheet(
    SpriteId::Checkpoint,
    "checkpoint",
    "assets/images/sprites/checkpoint.rgba",
    SpriteUsage::World,
    SpriteSize::new(128, 128),
    &WORLD_PROP_FRAMES,
    SpritePivot::CENTER,
);

pub(super) const SPRITE_ASSETS: &[SpriteAsset] = &[
    MENU_V1,
    PIERCER_HUD,
    DEATH_SKULL_1,
    DEATH_SKULL_2,
    DEATH_SHUTDOWN_FLASH,
    PLAYER_BASE,
    ENEMY_WEAK,
    PORTAL_BLUE,
    PORTAL_ORANGE,
    DOOR_PANEL,
    HAZARD,
    CHECKPOINT,
];

pub(super) fn gpu_ready_assets() -> impl Iterator<Item = SpriteAsset> {
    SPRITE_ASSETS
        .iter()
        .copied()
        .filter(|asset| asset.raw_bytes().is_some())
}

pub(super) fn validate_manifest() -> Result<(), String> {
    for asset in SPRITE_ASSETS {
        if asset.frames().is_empty() {
            return Err(format!("sprite '{}' has no frames", asset.label()));
        }

        if let Some(bytes) = asset.raw_bytes() {
            let expected = asset.size().byte_len();
            if bytes.len() != expected {
                return Err(format!(
                    "sprite '{}' has {} bytes, expected {}",
                    asset.label(),
                    bytes.len(),
                    expected
                ));
            }
        }

        for frame in asset.frames() {
            if frame.rect.width == 0 || frame.rect.height == 0 {
                return Err(format!(
                    "sprite '{}' has an empty frame '{}'",
                    asset.label(),
                    frame.name
                ));
            }
            if !frame.rect.fits(asset.size()) {
                return Err(format!(
                    "sprite '{}' frame '{}' is outside the sheet",
                    asset.label(),
                    frame.name
                ));
            }
        }
    }

    Ok(())
}

pub(super) fn manifest_score() -> usize {
    SPRITE_ASSETS
        .iter()
        .fold(0usize, |score, asset| score ^ asset.manifest_score())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_manifest_is_valid() {
        validate_manifest().expect("sprite manifest should be internally consistent");
    }

    #[test]
    fn current_raw_sprites_are_gpu_ready() {
        let raw_count = gpu_ready_assets().count();

        assert_eq!(raw_count, 5);
    }
}
