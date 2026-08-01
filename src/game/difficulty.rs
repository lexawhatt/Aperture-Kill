#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Harmless,
    Lenient,
    Standard,
    Violent,
    Brutal,
    UltrakillMustDie,
}

impl Difficulty {
    pub const ALL: [Self; Self::COUNT] = [
        Self::Harmless,
        Self::Lenient,
        Self::Standard,
        Self::Violent,
        Self::Brutal,
        Self::UltrakillMustDie,
    ];
    pub const COUNT: usize = 6;

    pub fn index(self) -> usize {
        match self {
            Self::Harmless => 0,
            Self::Lenient => 1,
            Self::Standard => 2,
            Self::Violent => 3,
            Self::Brutal => 4,
            Self::UltrakillMustDie => 5,
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Harmless => "HARMLESS",
            Self::Lenient => "LENIENT",
            Self::Standard => "STANDARD",
            Self::Violent => "VIOLENT",
            Self::Brutal => "BRUTAL",
            Self::UltrakillMustDie => "ULTRAKILL MUST DIE",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::Harmless | Self::Lenient => "ACCESSIBLE",
            Self::Standard | Self::Violent => "HARD",
            Self::Brutal | Self::UltrakillMustDie => "VERY HARD",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Harmless => "SLOW ENEMIES, 200 HP, LOW STRESS.",
            Self::Lenient => "SLOWER ENEMY TIMING, STRICT DAMAGE.",
            Self::Standard => "BASELINE COMBAT RULES.",
            Self::Violent => "25% FASTER ENEMY PRESSURE.",
            Self::Brutal => "50% FASTER ENEMIES AND BRUTAL BEHAVIOR.",
            Self::UltrakillMustDie => "UNDER CONSTRUCTION.",
        }
    }

    pub fn available(self) -> bool {
        !matches!(self, Self::UltrakillMustDie)
    }

    pub fn player_max_health(self) -> f32 {
        match self {
            Self::Harmless => 200.0,
            Self::Lenient
            | Self::Standard
            | Self::Violent
            | Self::Brutal
            | Self::UltrakillMustDie => 100.0,
        }
    }

    pub fn enemy_speed_multiplier(self) -> f32 {
        match self {
            Self::Harmless => 0.5,
            Self::Lenient => 0.75,
            Self::Standard => 1.0,
            Self::Violent => 1.25,
            Self::Brutal | Self::UltrakillMustDie => 1.5,
        }
    }

    pub fn enemy_damage_multiplier(self) -> f32 {
        1.0
    }
}
