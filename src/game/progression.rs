use crate::game::levels::LevelSpec;

pub const CUSTOM_LEVELS_CHAPTER_INDEX: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuLevelInfo {
    pub code: &'static str,
    pub title: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerInfo {
    pub title: &'static str,
    pub levels: &'static [MenuLevelInfo],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChapterInfo {
    pub label: &'static str,
    pub section: &'static str,
    pub layers: &'static [LayerInfo],
}

const PRELUDE_LEVELS: [MenuLevelInfo; 5] = [
    MenuLevelInfo {
        code: "0-1",
        title: "INTO THE FIRE",
    },
    MenuLevelInfo {
        code: "0-2",
        title: "THE MEATGRINDER",
    },
    MenuLevelInfo {
        code: "0-3",
        title: "DOUBLE DOWN",
    },
    MenuLevelInfo {
        code: "0-4",
        title: "A ONE-MACHINE ARMY",
    },
    MenuLevelInfo {
        code: "0-5",
        title: "CERBERUS",
    },
];

const LIMBO_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "1-1",
        title: "HEART OF THE SUNRISE",
    },
    MenuLevelInfo {
        code: "1-2",
        title: "THE BURNING WORLD",
    },
    MenuLevelInfo {
        code: "1-3",
        title: "HALLS OF SACRED REMAINS",
    },
    MenuLevelInfo {
        code: "1-4",
        title: "CLAIR DE LUNE",
    },
];

const LUST_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "2-1",
        title: "BRIDGEBURNER",
    },
    MenuLevelInfo {
        code: "2-2",
        title: "DEATH AT 20,000 VOLTS",
    },
    MenuLevelInfo {
        code: "2-3",
        title: "SHEER HEART ATTACK",
    },
    MenuLevelInfo {
        code: "2-4",
        title: "COURT OF THE CORPSE KING",
    },
];

const GLUTTONY_LEVELS: [MenuLevelInfo; 2] = [
    MenuLevelInfo {
        code: "3-1",
        title: "BELLY OF THE BEAST",
    },
    MenuLevelInfo {
        code: "3-2",
        title: "IN THE FLESH",
    },
];

const GREED_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "4-1",
        title: "SLAVES TO POWER",
    },
    MenuLevelInfo {
        code: "4-2",
        title: "GOD DAMN THE SUN",
    },
    MenuLevelInfo {
        code: "4-3",
        title: "A SHOT IN THE DARK",
    },
    MenuLevelInfo {
        code: "4-4",
        title: "CLAIR DE SOLEIL",
    },
];

const WRATH_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "5-1",
        title: "IN THE WAKE OF POSEIDON",
    },
    MenuLevelInfo {
        code: "5-2",
        title: "WAVES OF THE STARLESS SEA",
    },
    MenuLevelInfo {
        code: "5-3",
        title: "SHIP OF FOOLS",
    },
    MenuLevelInfo {
        code: "5-4",
        title: "LEVIATHAN",
    },
];

const HERESY_LEVELS: [MenuLevelInfo; 2] = [
    MenuLevelInfo {
        code: "6-1",
        title: "CRY FOR THE WEEPER",
    },
    MenuLevelInfo {
        code: "6-2",
        title: "AESTHETICS OF HATE",
    },
];

const VIOLENCE_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "7-1",
        title: "GARDEN OF FORKING PATHS",
    },
    MenuLevelInfo {
        code: "7-2",
        title: "LIGHT UP THE NIGHT",
    },
    MenuLevelInfo {
        code: "7-3",
        title: "NO SOUND, NO MEMORY",
    },
    MenuLevelInfo {
        code: "7-4",
        title: "...LIKE ANTENNAS TO HEAVEN",
    },
];

const FRAUD_LEVELS: [MenuLevelInfo; 4] = [
    MenuLevelInfo {
        code: "8-1",
        title: "HURTBREAK WONDERLAND",
    },
    MenuLevelInfo {
        code: "8-2",
        title: "THROUGH THE MIRROR",
    },
    MenuLevelInfo {
        code: "8-3",
        title: "DISINTEGRATION LOOP",
    },
    MenuLevelInfo {
        code: "8-4",
        title: "FINAL FLIGHT",
    },
];

const TREACHERY_LEVELS: [MenuLevelInfo; 2] = [
    MenuLevelInfo {
        code: "9-1",
        title: "???",
    },
    MenuLevelInfo {
        code: "9-2",
        title: "???",
    },
];

const PRELUDE_LAYERS: [LayerInfo; 1] = [LayerInfo {
    title: "PRELUDE",
    levels: &PRELUDE_LEVELS,
}];

const ACT_I_LAYERS: [LayerInfo; 3] = [
    LayerInfo {
        title: "LAYER 1: LIMBO",
        levels: &LIMBO_LEVELS,
    },
    LayerInfo {
        title: "LAYER 2: LUST",
        levels: &LUST_LEVELS,
    },
    LayerInfo {
        title: "LAYER 3: GLUTTONY",
        levels: &GLUTTONY_LEVELS,
    },
];

const ACT_II_LAYERS: [LayerInfo; 3] = [
    LayerInfo {
        title: "LAYER 4: GREED",
        levels: &GREED_LEVELS,
    },
    LayerInfo {
        title: "LAYER 5: WRATH",
        levels: &WRATH_LEVELS,
    },
    LayerInfo {
        title: "LAYER 6: HERESY",
        levels: &HERESY_LEVELS,
    },
];

const ACT_III_LAYERS: [LayerInfo; 3] = [
    LayerInfo {
        title: "LAYER 7: VIOLENCE",
        levels: &VIOLENCE_LEVELS,
    },
    LayerInfo {
        title: "LAYER 8: FRAUD",
        levels: &FRAUD_LEVELS,
    },
    LayerInfo {
        title: "LAYER 9: TREACHERY",
        levels: &TREACHERY_LEVELS,
    },
];

const EMPTY_LAYERS: [LayerInfo; 0] = [];

pub const CHAPTERS: [ChapterInfo; 9] = [
    ChapterInfo {
        label: "PRELUDE",
        section: "PRIMARY",
        layers: &PRELUDE_LAYERS,
    },
    ChapterInfo {
        label: "ACT I: INFINITE HYPERDEATH",
        section: "PRIMARY",
        layers: &ACT_I_LAYERS,
    },
    ChapterInfo {
        label: "ACT II: IMPERFECT HATRED",
        section: "PRIMARY",
        layers: &ACT_II_LAYERS,
    },
    ChapterInfo {
        label: "ACT III: GODFIST SUICIDE",
        section: "PRIMARY",
        layers: &ACT_III_LAYERS,
    },
    ChapterInfo {
        label: "ENCORES",
        section: "SECONDARY",
        layers: &EMPTY_LAYERS,
    },
    ChapterInfo {
        label: "PRIME SANCTUMS",
        section: "SECONDARY",
        layers: &EMPTY_LAYERS,
    },
    ChapterInfo {
        label: "THE CYBER GRIND",
        section: "SECONDARY",
        layers: &EMPTY_LAYERS,
    },
    ChapterInfo {
        label: "SANDBOX",
        section: "SECONDARY",
        layers: &EMPTY_LAYERS,
    },
    ChapterInfo {
        label: "CUSTOM LEVELS",
        section: "SECONDARY",
        layers: &EMPTY_LAYERS,
    },
];

pub fn find_level_by_code(levels: &[LevelSpec], code: &str) -> Option<usize> {
    levels
        .iter()
        .position(|level| level_code(&level.name).is_some_and(|candidate| candidate == code))
}

pub fn chapter_level_count(chapter_index: usize) -> usize {
    CHAPTERS
        .get(chapter_index)
        .map(|chapter| chapter.layers.iter().map(|layer| layer.levels.len()).sum())
        .unwrap_or(0)
}

pub fn chapter_level(chapter_index: usize, level_index: usize) -> Option<&'static MenuLevelInfo> {
    let mut remaining = level_index;

    for layer in CHAPTERS.get(chapter_index)?.layers {
        if remaining < layer.levels.len() {
            return layer.levels.get(remaining);
        }
        remaining -= layer.levels.len();
    }

    None
}

pub fn is_custom_chapter(chapter_index: usize) -> bool {
    chapter_index == CUSTOM_LEVELS_CHAPTER_INDEX
}

pub fn custom_level_indices(levels: &[LevelSpec]) -> Vec<usize> {
    levels
        .iter()
        .enumerate()
        .filter_map(|(index, level)| level_code(&level.name).is_none().then_some(index))
        .collect()
}

pub fn level_code(name: &str) -> Option<&str> {
    let code = name.split_whitespace().next()?;
    let mut parts = code.split('-');
    let major = parts.next()?;
    let minor = parts.next()?;

    (parts.next().is_none()
        && !major.is_empty()
        && !minor.is_empty()
        && major.chars().all(|ch| ch.is_ascii_digit())
        && minor.chars().all(|ch| ch.is_ascii_digit()))
    .then_some(code)
}
