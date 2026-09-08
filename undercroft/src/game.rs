//! Shared state: the game state machine, the hero's persistent stats, and
//! components used across modules.

use crate::{
    art::SpriteId,
    classes::Class,
    dungeon::Dungeon,
    items::{ARMORS, BOWS, MagicItem, Perk, SWORDS},
};
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Title,
    /// Picking a hero class before a run.
    ClassSelect,
    /// Generating and spawning the next floor.
    Loading,
    Playing,
    Shop,
    SkillChoice,
    Paused,
    GameOver,
    Victory,
}

/// Z layers on the low-resolution canvas.
pub mod layer {
    pub const FLOOR: f32 = 0.0;
    pub const DECOR: f32 = 1.0;
    pub const ITEM: f32 = 2.0;
    pub const ACTOR: f32 = 10.0;
    pub const PROJECTILE: f32 = 30.0;
    pub const PARTICLE: f32 = 32.0;
    pub const DARKNESS: f32 = 50.0;
    pub const OVERLAY: f32 = 60.0;
}

/// Everything spawned for the current floor; despawned when it changes.
#[derive(Component)]
pub struct FloorEntity;

/// Everything that lives for a whole run (the hero); despawned on game over.
#[derive(Component)]
pub struct RunEntity;

#[derive(Component, Clone, Copy)]
pub struct Health {
    pub hp: i32,
    pub max: i32,
}

#[derive(Component)]
pub struct Energy {
    pub cur: f32,
    pub max: f32,
    /// Seconds since energy was last spent; regeneration waits a moment.
    pub since_use: f32,
}

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Facing(pub Vec2);

/// Half extents of the entity's collision box.
#[derive(Component, Clone, Copy)]
pub struct Hitbox(pub Vec2);

/// Two-frame sprite animation driven by `animate_sprites`.
#[derive(Component)]
pub struct Animation {
    pub id: SpriteId,
    pub frame_time: f32,
    pub timer: f32,
    pub frame: usize,
    pub playing: bool,
}

impl Animation {
    pub fn new(id: SpriteId, frame_time: f32) -> Self {
        Self {
            id,
            frame_time,
            timer: 0.0,
            frame: 0,
            playing: true,
        }
    }
}

/// Draw order follows the vertical position, so things lower on screen draw
/// on top, giving a sense of depth.
#[derive(Component)]
pub struct YSort;

#[derive(Component)]
pub struct HitFlash(pub f32);

#[derive(Component)]
pub struct Knockback(pub Vec2);

#[derive(Component)]
pub struct Invulnerable(pub f32);

#[derive(Component)]
pub struct Lifetime(pub f32);

#[derive(Component)]
pub struct Particle {
    pub vel: Vec2,
    pub drag: f32,
    pub gravity: f32,
}

/// A point light for the darkness overlay.
#[derive(Component, Clone, Copy)]
pub struct Light {
    pub radius: f32,
    pub intensity: f32,
    pub color: Color,
    pub flicker: f32,
}

#[derive(Resource, Default)]
pub struct Shake {
    pub trauma: f32,
}

impl Shake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }
}

/// The floor currently loaded.
#[derive(Resource)]
pub struct CurrentFloor {
    pub dungeon: Dungeon,
    pub boss_sealed: bool,
}

/// Seed for the whole run; each floor derives its own from it.
#[derive(Resource)]
pub struct RunSeed(pub u64);

/// Elemental damage types carried by arrows and some monster attacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Element {
    Fire,
    Poison,
    Frost,
}

impl Element {
    pub const ALL: [Element; 3] = [Element::Poison, Element::Fire, Element::Frost];

    pub fn name(self) -> &'static str {
        match self {
            Element::Fire => "Fire",
            Element::Poison => "Poison",
            Element::Frost => "Frost",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Element::Fire => crate::palette::DARK_ORANGE,
            Element::Poison => crate::palette::GREEN,
            Element::Frost => crate::palette::CYAN,
        }
    }
}

/// A short line shown at the bottom of the HUD.
#[derive(Message)]
pub struct Notify(pub String);

/// Sent by the character select screen: begin a fresh run as this class.
#[derive(Message)]
pub struct StartRun(pub Class);

/// The hero's stats and inventory. Persists across floors within a run.
#[derive(Resource, Clone)]
pub struct Hero {
    pub class: Class,
    pub floor: u32,
    pub level: u32,
    pub xp: u32,
    pub coins: u32,
    pub kills: u32,
    pub sword: usize,
    pub bow: Option<usize>,
    pub armor: Option<usize>,
    pub potions: u32,
    pub arrows: u32,
    pub keys: u32,
    pub heart_containers: u32,
    pub energy_crystals: u32,
    pub perks: Vec<Perk>,
    pub magic: Vec<MagicItem>,
    pub phoenix_used: bool,
    /// Arrow enchantments bought from shops, and the one currently loaded.
    pub quivers: Vec<Element>,
    pub arrow_type: Option<Element>,
}

impl Default for Hero {
    fn default() -> Self {
        Self {
            class: Class::Knight,
            floor: 1,
            level: 1,
            xp: 0,
            coins: 0,
            kills: 0,
            sword: 0,
            bow: None,
            armor: None,
            potions: 1,
            arrows: 0,
            keys: 0,
            heart_containers: 0,
            energy_crystals: 0,
            perks: Vec::new(),
            magic: Vec::new(),
            phoenix_used: false,
            quivers: Vec::new(),
            arrow_type: None,
        }
    }
}

pub const BASE_HP: i32 = 20;
pub const BASE_ENERGY: f32 = 100.0;

impl Hero {
    /// A fresh hero of the given class with that class's starting kit.
    pub fn new(class: Class) -> Self {
        let mut hero = Hero {
            class,
            ..Default::default()
        };
        match class {
            Class::Ranger => {
                hero.bow = Some(0);
                hero.arrows = 20;
            }
            Class::Mage => hero.potions = 2,
            Class::Knight | Class::Rogue => {}
        }
        hero
    }

    pub fn has_perk(&self, perk: Perk) -> bool {
        self.perks.contains(&perk)
    }

    pub fn has_magic(&self, item: MagicItem) -> bool {
        self.magic.contains(&item)
    }

    pub fn max_hp(&self) -> i32 {
        BASE_HP
            + self.class.bonus_hp()
            + (self.level as i32 - 1) * 3
            + self.heart_containers as i32 * 5
    }

    pub fn max_energy(&self) -> f32 {
        BASE_ENERGY + (self.level as f32 - 1.0) * 8.0 + self.energy_crystals as f32 * 25.0
    }

    pub fn xp_to_next(&self) -> u32 {
        20 + self.level * self.level * 12
    }

    /// Adds XP, returning how many levels were gained.
    pub fn gain_xp(&mut self, amount: u32) -> u32 {
        let amount = if self.has_perk(Perk::Scholar) {
            amount + amount / 2
        } else {
            amount
        };
        self.xp += amount;
        let mut gained = 0;
        while self.xp >= self.xp_to_next() {
            self.xp -= self.xp_to_next();
            self.level += 1;
            gained += 1;
        }
        gained
    }

    pub fn sword_damage(&self) -> i32 {
        SWORDS[self.sword].damage + if self.has_perk(Perk::Berserker) { 2 } else { 0 }
    }

    pub fn bow_damage(&self) -> i32 {
        self.bow.map(|b| BOWS[b].damage).unwrap_or(0)
    }

    pub fn armor_reduction(&self) -> i32 {
        self.armor.map(|a| ARMORS[a].reduction).unwrap_or(0)
            + if self.has_perk(Perk::ThickSkin) { 1 } else { 0 }
    }

    pub fn move_speed(&self) -> f32 {
        80.0 * self.class.speed_multiplier()
            * if self.has_perk(Perk::SwiftFeet) {
                1.15
            } else {
                1.0
            }
    }

    /// Energy a blocked hit costs.
    pub fn block_cost(&self) -> f32 {
        self.class.block_cost()
    }

    pub fn sprint_multiplier(&self) -> f32 {
        2.6
    }

    pub fn sprint_drain(&self) -> f32 {
        let base = 28.0;
        if self.has_magic(MagicItem::BootsOfHaste) {
            base * 0.5
        } else {
            base
        }
    }

    pub fn energy_regen(&self) -> f32 {
        14.0 * if self.has_perk(Perk::Marathon) {
            1.5
        } else {
            1.0
        }
    }

    pub fn sword_cooldown(&self) -> f32 {
        0.32 * if self.has_perk(Perk::QuickDraw) {
            0.7
        } else {
            1.0
        }
    }

    pub fn potion_heal(&self) -> i32 {
        if self.has_perk(Perk::SecondWind) {
            14
        } else {
            8
        }
    }

    pub fn invuln_time(&self) -> f32 {
        if self.has_perk(Perk::Nimble) {
            1.2
        } else {
            0.8
        }
    }

    pub fn light_radius(&self) -> f32 {
        self.class.bonus_light()
            + if self.has_magic(MagicItem::AmuletOfLight) {
                150.0
            } else {
                96.0
            }
    }

    /// Steps to the next owned arrow type (plain arrows are always available).
    pub fn cycle_arrows(&mut self) {
        let mut order: Vec<Option<Element>> = vec![None];
        order.extend(self.quivers.iter().map(|e| Some(*e)));
        let i = order
            .iter()
            .position(|e| *e == self.arrow_type)
            .unwrap_or(0);
        self.arrow_type = order[(i + 1) % order.len()];
    }

    pub fn coin_value(&self, base: u32) -> u32 {
        if self.has_perk(Perk::TreasureHunter) {
            base + base.div_ceil(2)
        } else {
            base
        }
    }
}

/// Low-resolution canvas the world is drawn to before being scaled up by an
/// integer factor. 426x240 at 3x fills a 720p window.
pub const CANVAS_W: u32 = 426;
pub const CANVAS_H: u32 = 240;

/// The camera that draws the world onto the low-resolution canvas.
#[derive(Component)]
pub struct GameCamera;

/// Big text shown briefly in the middle of the screen.
#[derive(Message)]
pub struct Banner(pub String);

/// Ordering of the gameplay systems within a frame.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Move,
    Ai,
    Hits,
    Damage,
    Juice,
    Camera,
}
