use crate::game::enemy::Enemy;
use crate::game::level::{
    Checkpoint, Door, Hazard, LevelObjectMeta, LevelText, LevelTrigger, Solid, WorldPortal,
};

#[derive(Clone, PartialEq)]
pub(super) struct LevelSnapshot {
    pub(super) solids: Vec<Solid>,
    pub(super) doors: Vec<Door>,
    pub(super) hazards: Vec<Hazard>,
    pub(super) checkpoints: Vec<Checkpoint>,
    pub(super) enemies: Vec<Enemy>,
    pub(super) triggers: Vec<LevelTrigger>,
    pub(super) texts: Vec<LevelText>,
    pub(super) world_portals: Vec<WorldPortal>,
    pub(super) metadata: Vec<LevelObjectMeta>,
}

#[derive(Default)]
pub(super) struct EditorPan {
    pub(super) left: bool,
    pub(super) right: bool,
    pub(super) up: bool,
    pub(super) down: bool,
}

#[derive(Clone, Copy)]
pub(in crate::app) enum EditorCategory {
    Building,
    Utility,
    Enemy,
    Triggers,
    Deco,
}

impl EditorCategory {
    pub(in crate::app) const COUNT: usize = 5;

    pub(in crate::app) fn index(self) -> usize {
        match self {
            Self::Building => 0,
            Self::Utility => 1,
            Self::Enemy => 2,
            Self::Triggers => 3,
            Self::Deco => 4,
        }
    }

    pub(in crate::app) fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Building),
            1 => Some(Self::Utility),
            2 => Some(Self::Enemy),
            3 => Some(Self::Triggers),
            4 => Some(Self::Deco),
            _ => None,
        }
    }

    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Self::Building => "BUILDING",
            Self::Utility => "UTILITY",
            Self::Enemy => "ENEMY",
            Self::Triggers => "TRIGGERS",
            Self::Deco => "DECO",
        }
    }

    pub(in crate::app) fn tools(self) -> &'static [EditorTool] {
        match self {
            Self::Building => &[
                EditorTool::Portalable,
                EditorTool::Solid,
                EditorTool::Hazard,
            ],
            Self::Utility => &[
                EditorTool::Door,
                EditorTool::Checkpoint,
                EditorTool::WorldPortal,
                EditorTool::Text,
            ],
            Self::Enemy => &[EditorTool::Filth],
            Self::Triggers => &[
                EditorTool::LevelStart,
                EditorTool::LevelEnd,
                EditorTool::EnemySpawnTrigger,
            ],
            Self::Deco => &[],
        }
    }

    pub(in crate::app) fn contains_tool(self, tool: EditorTool) -> bool {
        self.tools().contains(&tool)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum EditorTool {
    Portalable,
    Solid,
    Hazard,
    Door,
    Checkpoint,
    WorldPortal,
    Text,
    Filth,
    LevelStart,
    LevelEnd,
    EnemySpawnTrigger,
}

impl EditorTool {
    pub(in crate::app) fn portalable(self) -> bool {
        matches!(self, Self::Portalable)
    }

    pub(in crate::app) fn category(self) -> EditorCategory {
        match self {
            Self::Portalable | Self::Solid | Self::Hazard => EditorCategory::Building,
            Self::Door | Self::Checkpoint | Self::WorldPortal | Self::Text => {
                EditorCategory::Utility
            }
            Self::Filth => EditorCategory::Enemy,
            Self::LevelStart | Self::LevelEnd | Self::EnemySpawnTrigger => EditorCategory::Triggers,
        }
    }

    pub(in crate::app) fn index(self) -> usize {
        match self {
            Self::Portalable => 1,
            Self::Solid => 2,
            Self::Hazard => 3,
            Self::Door => 4,
            Self::Checkpoint => 5,
            Self::WorldPortal => 6,
            Self::Text => 7,
            Self::Filth => 8,
            Self::LevelStart => 9,
            Self::LevelEnd => 10,
            Self::EnemySpawnTrigger => 11,
        }
    }

    pub(in crate::app) fn from_index(index: usize) -> Option<Self> {
        match index {
            1 => Some(Self::Portalable),
            2 => Some(Self::Solid),
            3 => Some(Self::Hazard),
            4 => Some(Self::Door),
            5 => Some(Self::Checkpoint),
            6 => Some(Self::WorldPortal),
            7 => Some(Self::Text),
            8 => Some(Self::Filth),
            9 => Some(Self::LevelStart),
            10 => Some(Self::LevelEnd),
            11 => Some(Self::EnemySpawnTrigger),
            _ => None,
        }
    }

    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Self::Portalable => "P SURF",
            Self::Solid => "SURFACE",
            Self::Hazard => "ACID",
            Self::Door => "DOOR",
            Self::Checkpoint => "CHECK",
            Self::WorldPortal => "W PORT",
            Self::Text => "TEXT",
            Self::Filth => "FILTH",
            Self::LevelStart => "START",
            Self::LevelEnd => "END",
            Self::EnemySpawnTrigger => "SPAWN",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum EditorMode {
    Build,
    Edit,
    Delete,
}

impl EditorMode {
    pub(in crate::app) const COUNT: usize = 3;

    pub(in crate::app) fn index(self) -> usize {
        match self {
            Self::Build => 0,
            Self::Edit => 1,
            Self::Delete => 2,
        }
    }

    pub(in crate::app) fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Build),
            1 => Some(Self::Edit),
            2 => Some(Self::Delete),
            _ => None,
        }
    }

    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Self::Build => "BUILD",
            Self::Edit => "EDIT",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::app) enum EditorSelectionKind {
    None,
    Solid,
    Door,
    Hazard,
    Checkpoint,
    Enemy,
    Trigger,
    Text,
    WorldPortal,
}

impl EditorSelectionKind {
    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Solid => "SOLID",
            Self::Door => "DOOR",
            Self::Hazard => "ACID",
            Self::Checkpoint => "CHECKPOINT",
            Self::Enemy => "FILTH",
            Self::Trigger => "TRIGGER",
            Self::Text => "TEXT",
            Self::WorldPortal => "WORLD PORTAL",
        }
    }
}
