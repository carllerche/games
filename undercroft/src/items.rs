//! Gear tiers, consumables, magic items, perks and shop stock.

use crate::{
    art::SpriteId,
    game::{Element, Hero},
    palette,
};
use bevy::prelude::*;
use rand::{Rng, seq::SliceRandom};

pub struct SwordTier {
    pub name: &'static str,
    pub damage: i32,
    pub level: u32,
    pub price: u32,
    pub tint: Color,
}

pub const SWORDS: [SwordTier; 5] = [
    SwordTier {
        name: "Rusty Sword",
        damage: 2,
        level: 1,
        price: 0,
        tint: palette::BROWN,
    },
    SwordTier {
        name: "Iron Sword",
        damage: 3,
        level: 2,
        price: 35,
        tint: palette::LIGHT_GREY,
    },
    SwordTier {
        name: "Steel Sword",
        damage: 5,
        level: 4,
        price: 85,
        tint: palette::WHITE,
    },
    SwordTier {
        name: "Enchanted Blade",
        damage: 7,
        level: 6,
        price: 160,
        tint: palette::CYAN,
    },
    SwordTier {
        name: "Runeblade",
        damage: 10,
        level: 8,
        price: 280,
        tint: palette::PURPLE,
    },
];

pub struct BowTier {
    pub name: &'static str,
    pub damage: i32,
    pub level: u32,
    pub price: u32,
    pub arrow_speed: f32,
}

pub const BOWS: [BowTier; 3] = [
    BowTier {
        name: "Short Bow",
        damage: 2,
        level: 1,
        price: 40,
        arrow_speed: 200.0,
    },
    BowTier {
        name: "Longbow",
        damage: 4,
        level: 3,
        price: 100,
        arrow_speed: 260.0,
    },
    BowTier {
        name: "Elven Bow",
        damage: 6,
        level: 5,
        price: 180,
        arrow_speed: 320.0,
    },
];

pub struct ArmorTier {
    pub name: &'static str,
    pub reduction: i32,
    pub level: u32,
    pub price: u32,
    pub tint: Color,
}

pub const ARMORS: [ArmorTier; 3] = [
    ArmorTier {
        name: "Leather Armor",
        reduction: 1,
        level: 1,
        price: 45,
        tint: palette::BROWN,
    },
    ArmorTier {
        name: "Chainmail",
        reduction: 2,
        level: 3,
        price: 120,
        tint: palette::GREY,
    },
    ArmorTier {
        name: "Plate Armor",
        reduction: 3,
        level: 6,
        price: 240,
        tint: palette::WHITE,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perk {
    SwiftFeet,
    Marathon,
    Sharpshooter,
    Vampiric,
    ThickSkin,
    TreasureHunter,
    Scholar,
    QuickDraw,
    SecondWind,
    Berserker,
    Nimble,
    Volley,
    SprintStrike,
    Fletcher,
}

impl Perk {
    pub const ALL: [Perk; 14] = [
        Perk::SwiftFeet,
        Perk::Marathon,
        Perk::Sharpshooter,
        Perk::Vampiric,
        Perk::ThickSkin,
        Perk::TreasureHunter,
        Perk::Scholar,
        Perk::QuickDraw,
        Perk::SecondWind,
        Perk::Berserker,
        Perk::Nimble,
        Perk::Volley,
        Perk::SprintStrike,
        Perk::Fletcher,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Perk::SwiftFeet => "Swift Feet",
            Perk::Marathon => "Marathon",
            Perk::Sharpshooter => "Sharpshooter",
            Perk::Vampiric => "Vampiric",
            Perk::ThickSkin => "Thick Skin",
            Perk::TreasureHunter => "Treasure Hunter",
            Perk::Scholar => "Scholar",
            Perk::QuickDraw => "Quick Draw",
            Perk::SecondWind => "Second Wind",
            Perk::Berserker => "Berserker",
            Perk::Nimble => "Nimble",
            Perk::Volley => "Volley",
            Perk::SprintStrike => "Sprint Strike",
            Perk::Fletcher => "Fletcher",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Perk::SwiftFeet => "Move 15% faster.",
            Perk::Marathon => "Energy regenerates 50% faster.",
            Perk::Sharpshooter => "Arrows pierce through enemies.",
            Perk::Vampiric => "Heal 1 HP for every kill.",
            Perk::ThickSkin => "Take 1 less damage from every hit.",
            Perk::TreasureHunter => "Coins are worth 50% more.",
            Perk::Scholar => "Gain 50% more XP.",
            Perk::QuickDraw => "Swing the sword 30% faster.",
            Perk::SecondWind => "Potions heal 14 instead of 8.",
            Perk::Berserker => "Sword damage +2.",
            Perk::Nimble => "Longer invulnerability after being hit.",
            Perk::Volley => "The bow fires two arrows at once.",
            Perk::SprintStrike => "Sprinting into enemies hurts them.",
            Perk::Fletcher => "Half of your shots don't use an arrow.",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MagicItem {
    RingOfRegeneration,
    AmuletOfLight,
    BootsOfHaste,
    PhoenixFeather,
}

impl MagicItem {
    pub const ALL: [MagicItem; 4] = [
        MagicItem::RingOfRegeneration,
        MagicItem::AmuletOfLight,
        MagicItem::BootsOfHaste,
        MagicItem::PhoenixFeather,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MagicItem::RingOfRegeneration => "Ring of Regeneration",
            MagicItem::AmuletOfLight => "Amulet of Light",
            MagicItem::BootsOfHaste => "Boots of Haste",
            MagicItem::PhoenixFeather => "Phoenix Feather",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MagicItem::RingOfRegeneration => "Slowly regenerates health.",
            MagicItem::AmuletOfLight => "You see much further in the dark.",
            MagicItem::BootsOfHaste => "Sprinting drains half the energy.",
            MagicItem::PhoenixFeather => "Revive once with half health.",
        }
    }

    pub fn price(self) -> u32 {
        match self {
            MagicItem::RingOfRegeneration => 130,
            MagicItem::AmuletOfLight => 70,
            MagicItem::BootsOfHaste => 90,
            MagicItem::PhoenixFeather => 200,
        }
    }

    pub fn level(self) -> u32 {
        match self {
            MagicItem::RingOfRegeneration => 4,
            MagicItem::AmuletOfLight => 2,
            MagicItem::BootsOfHaste => 3,
            MagicItem::PhoenixFeather => 5,
        }
    }

    pub fn sprite(self) -> SpriteId {
        match self {
            MagicItem::RingOfRegeneration => SpriteId::Ring,
            MagicItem::AmuletOfLight => SpriteId::Amulet,
            MagicItem::BootsOfHaste => SpriteId::Boots,
            MagicItem::PhoenixFeather => SpriteId::Feather,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShopItem {
    Potion,
    Arrows,
    Sword(usize),
    Bow(usize),
    Armor(usize),
    HeartContainer,
    EnergyCrystal,
    Magic(MagicItem),
    /// Enchanted arrows: every shot carries the element.
    Quiver(Element),
}

impl ShopItem {
    pub fn name(self) -> String {
        match self {
            ShopItem::Potion => "Healing Potion".into(),
            ShopItem::Arrows => "Bundle of Arrows (x10)".into(),
            ShopItem::Sword(i) => SWORDS[i].name.into(),
            ShopItem::Bow(i) => BOWS[i].name.into(),
            ShopItem::Armor(i) => ARMORS[i].name.into(),
            ShopItem::HeartContainer => "Heart Container".into(),
            ShopItem::EnergyCrystal => "Energy Crystal".into(),
            ShopItem::Magic(m) => m.name().into(),
            ShopItem::Quiver(e) => format!("{} Quiver", e.name()),
        }
    }

    pub fn description(self, hero: &Hero) -> String {
        match self {
            ShopItem::Potion => format!("Restores {} HP. Drink with Q.", hero.potion_heal()),
            ShopItem::Arrows => "Ammunition for your bow.".into(),
            ShopItem::Sword(i) => format!(
                "Sword damage {} (now {}).",
                SWORDS[i].damage,
                hero.sword_damage()
            ),
            ShopItem::Bow(i) => format!("Arrow damage {}. Fire with K.", BOWS[i].damage),
            ShopItem::Armor(i) => format!("Blocks {} damage per hit.", ARMORS[i].reduction),
            ShopItem::HeartContainer => "Max health +5.".into(),
            ShopItem::EnergyCrystal => "Max energy +25.".into(),
            ShopItem::Magic(m) => m.description().into(),
            ShopItem::Quiver(Element::Fire) => {
                "Arrows set enemies ablaze. Fire spreads between them.".into()
            }
            ShopItem::Quiver(Element::Poison) => {
                "Arrows poison enemies, who keep taking damage.".into()
            }
            ShopItem::Quiver(Element::Frost) => "Arrows freeze enemies solid for a moment.".into(),
        }
    }

    pub fn price(self, floor: u32) -> u32 {
        let base = match self {
            ShopItem::Potion => 18,
            ShopItem::Arrows => 10,
            ShopItem::Sword(i) => SWORDS[i].price,
            ShopItem::Bow(i) => BOWS[i].price,
            ShopItem::Armor(i) => ARMORS[i].price,
            ShopItem::HeartContainer => 70,
            ShopItem::EnergyCrystal => 55,
            ShopItem::Magic(m) => m.price(),
            ShopItem::Quiver(Element::Poison) => 70,
            ShopItem::Quiver(Element::Fire) => 90,
            ShopItem::Quiver(Element::Frost) => 120,
        };
        base + base * (floor - 1) / 10
    }

    pub fn level(self) -> u32 {
        match self {
            ShopItem::Sword(i) => SWORDS[i].level,
            ShopItem::Bow(i) => BOWS[i].level,
            ShopItem::Armor(i) => ARMORS[i].level,
            ShopItem::Magic(m) => m.level(),
            ShopItem::Quiver(Element::Poison) => 2,
            ShopItem::Quiver(Element::Fire) => 3,
            ShopItem::Quiver(Element::Frost) => 4,
            _ => 1,
        }
    }

    pub fn sprite(self) -> (SpriteId, Color) {
        match self {
            ShopItem::Potion => (SpriteId::Potion, Color::WHITE),
            ShopItem::Arrows => (SpriteId::Arrows, Color::WHITE),
            ShopItem::Sword(i) => (SpriteId::Sword, SWORDS[i].tint),
            ShopItem::Bow(_) => (SpriteId::Bow, Color::WHITE),
            ShopItem::Armor(i) => (SpriteId::Armor, ARMORS[i].tint),
            ShopItem::HeartContainer => (SpriteId::Heart, Color::WHITE),
            ShopItem::EnergyCrystal => (SpriteId::Crystal, Color::WHITE),
            ShopItem::Magic(m) => (m.sprite(), Color::WHITE),
            ShopItem::Quiver(e) => (SpriteId::Arrows, e.color()),
        }
    }

    /// Applies the purchase to the hero. Returns a line for the message log.
    pub fn apply(self, hero: &mut Hero) -> String {
        match self {
            ShopItem::Potion => {
                hero.potions += 1;
                "Bought a healing potion.".into()
            }
            ShopItem::Arrows => {
                hero.arrows += 10;
                "Bought 10 arrows.".into()
            }
            ShopItem::Sword(i) => {
                hero.sword = i;
                format!("Equipped the {}.", SWORDS[i].name)
            }
            ShopItem::Bow(i) => {
                hero.bow = Some(i);
                if hero.arrows == 0 {
                    hero.arrows = 10;
                }
                format!("Equipped the {}.", BOWS[i].name)
            }
            ShopItem::Armor(i) => {
                hero.armor = Some(i);
                format!("Equipped {}.", ARMORS[i].name)
            }
            ShopItem::HeartContainer => {
                hero.heart_containers += 1;
                "Max health increased!".into()
            }
            ShopItem::EnergyCrystal => {
                hero.energy_crystals += 1;
                "Max energy increased!".into()
            }
            ShopItem::Magic(m) => {
                hero.magic.push(m);
                format!("Acquired the {}.", m.name())
            }
            ShopItem::Quiver(e) => {
                hero.quivers.push(e);
                hero.arrow_type = Some(e);
                format!("{} arrows loaded. Tab switches arrow types.", e.name())
            }
        }
    }
}

/// Picks what a shopkeeper has for sale on this floor.
pub fn generate_stock(hero: &Hero, floor: u32, rng: &mut impl Rng) -> Vec<ShopItem> {
    let mut stock = vec![ShopItem::Potion, ShopItem::Arrows];
    let mut pool = Vec::new();
    // The next tier of each piece of gear, plus one further for aspiration.
    for i in hero.sword + 1..SWORDS.len().min(hero.sword + 3) {
        pool.push(ShopItem::Sword(i));
    }
    let next_bow = hero.bow.map(|b| b + 1).unwrap_or(0);
    for i in next_bow..BOWS.len().min(next_bow + 2) {
        pool.push(ShopItem::Bow(i));
    }
    let next_armor = hero.armor.map(|a| a + 1).unwrap_or(0);
    for i in next_armor..ARMORS.len().min(next_armor + 2) {
        pool.push(ShopItem::Armor(i));
    }
    pool.push(ShopItem::HeartContainer);
    pool.push(ShopItem::EnergyCrystal);
    for m in MagicItem::ALL {
        if !hero.has_magic(m) && rng.random_bool(0.5) {
            pool.push(ShopItem::Magic(m));
        }
    }
    if hero.bow.is_some() {
        for e in Element::ALL {
            if !hero.quivers.contains(&e) && rng.random_bool(0.6) {
                pool.push(ShopItem::Quiver(e));
            }
        }
    }
    pool.shuffle(rng);
    let slots = 3 + (floor as usize / 3).min(2);
    stock.extend(pool.into_iter().take(slots));
    stock
}

/// Three random perks the hero doesn't have yet.
pub fn perk_choices(hero: &Hero, rng: &mut impl Rng) -> Vec<Perk> {
    let mut pool: Vec<Perk> = Perk::ALL
        .iter()
        .copied()
        .filter(|p| !hero.has_perk(*p))
        .collect();
    pool.shuffle(rng);
    pool.truncate(3);
    pool
}
