//! Hero classes: who you descend as, and the character select screen shown
//! before every run. Each class has a passive trait and a special ability
//! on F / the right bumper with its own cooldown.

use crate::{
    art::{Atlas, SpriteId},
    audio::{PlaySfx, SfxKind},
    game::*,
    hud::{icon, text},
    input::Controls,
    palette,
};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Class {
    #[default]
    Knight,
    Ranger,
    Mage,
    Rogue,
}

/// Which way a hero sprite faces.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    Side,
}

impl Class {
    pub const ALL: [Class; 4] = [Class::Knight, Class::Ranger, Class::Mage, Class::Rogue];

    pub fn name(self) -> &'static str {
        match self {
            Class::Knight => "Knight",
            Class::Ranger => "Ranger",
            Class::Mage => "Mage",
            Class::Rogue => "Rogue",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Class::Knight => "Sturdy. Holds the line.",
            Class::Ranger => "Never without a bow.",
            Class::Mage => "Frail, but the dark fears them.",
            Class::Rogue => "Fast, and hard to pin down.",
        }
    }

    pub fn ability_name(self) -> &'static str {
        match self {
            Class::Knight => "Shield Bash",
            Class::Ranger => "Arrow Storm",
            Class::Mage => "Frost Nova",
            Class::Rogue => "Shadow Step",
        }
    }

    pub fn ability_description(self) -> &'static str {
        match self {
            Class::Knight => "Charge forward behind the shield, smashing everything in the way.",
            Class::Ranger => "Loose a ring of twelve arrows without spending any.",
            Class::Mage => "Freeze every monster nearby and chill them for 3 damage.",
            Class::Rogue => {
                "Blink ahead, untouchable for a moment. The next sword hit does triple damage."
            }
        }
    }

    pub fn passive(self) -> &'static str {
        match self {
            Class::Knight => "+6 max health. Blocking costs half the energy.",
            Class::Ranger => "Starts with a Short Bow and 20 arrows.",
            Class::Mage => "Sees far in the dark. Starts with 2 potions but -4 max health.",
            Class::Rogue => "Moves 15% faster.",
        }
    }

    /// Seconds before the special ability can be used again.
    pub fn cooldown(self) -> f32 {
        match self {
            Class::Knight => 4.0,
            Class::Ranger => 6.0,
            Class::Mage => 7.0,
            Class::Rogue => 5.0,
        }
    }

    pub fn bonus_hp(self) -> i32 {
        match self {
            Class::Knight => 6,
            Class::Mage => -4,
            _ => 0,
        }
    }

    pub fn speed_multiplier(self) -> f32 {
        if self == Class::Rogue { 1.15 } else { 1.0 }
    }

    pub fn bonus_light(self) -> f32 {
        if self == Class::Mage { 40.0 } else { 0.0 }
    }

    /// Energy a blocked hit costs.
    pub fn block_cost(self) -> f32 {
        if self == Class::Knight { 6.0 } else { 12.0 }
    }

    /// Colour the shield is tinted, and the accent used on menus.
    pub fn color(self) -> Color {
        match self {
            Class::Knight => palette::WHITE,
            Class::Ranger => palette::LIME,
            Class::Mage => palette::PURPLE,
            Class::Rogue => palette::GREY,
        }
    }

    pub fn sprite(self, dir: Dir) -> SpriteId {
        match (self, dir) {
            (Class::Knight, Dir::Down) => SpriteId::HeroDown,
            (Class::Knight, Dir::Up) => SpriteId::HeroUp,
            (Class::Knight, Dir::Side) => SpriteId::HeroSide,
            (Class::Ranger, Dir::Down) => SpriteId::RangerDown,
            (Class::Ranger, Dir::Up) => SpriteId::RangerUp,
            (Class::Ranger, Dir::Side) => SpriteId::RangerSide,
            (Class::Mage, Dir::Down) => SpriteId::MageDown,
            (Class::Mage, Dir::Up) => SpriteId::MageUp,
            (Class::Mage, Dir::Side) => SpriteId::MageSide,
            (Class::Rogue, Dir::Down) => SpriteId::RogueDown,
            (Class::Rogue, Dir::Up) => SpriteId::RogueUp,
            (Class::Rogue, Dir::Side) => SpriteId::RogueSide,
        }
    }
}

// ---------------------------------------------------------------------------
// Character select
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct ClassChoice {
    selected: usize,
}

#[derive(Component)]
struct ClassUi;

pub struct ClassesPlugin;

impl Plugin for ClassesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::ClassSelect), open)
            .add_systems(Update, input.run_if(in_state(GameState::ClassSelect)))
            .add_systems(OnExit(GameState::ClassSelect), close);
    }
}

fn open(mut commands: Commands, atlas: Res<Atlas>, hero: Res<Hero>) {
    let selected = Class::ALL
        .iter()
        .position(|c| *c == hero.class)
        .unwrap_or(0);
    build_ui(&mut commands, &atlas, selected);
    commands.insert_resource(ClassChoice { selected });
}

fn close(mut commands: Commands, ui: Query<Entity, With<ClassUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<ClassChoice>();
}

fn build_ui(commands: &mut Commands, atlas: &Atlas, selected: usize) {
    commands
        .spawn((
            ClassUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.85)),
        ))
        .with_children(|root| {
            root.spawn(text("CHOOSE YOUR HERO", 40.0, palette::CARPET_TRIM));
            root.spawn(text(
                "Every hero carries a sword and a shield. Hold L to block.",
                16.0,
                palette::LIGHT_GREY,
            ));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(14.0),
                ..default()
            })
            .with_children(|row| {
                for (i, class) in Class::ALL.iter().enumerate() {
                    let is_selected = i == selected;
                    row.spawn((
                        Node {
                            width: Val::Px(250.0),
                            height: Val::Px(370.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(12.0)),
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(if is_selected {
                            class.color()
                        } else {
                            palette::DARK_GREY
                        }),
                        BackgroundColor(if is_selected {
                            palette::DARKER_GREY
                        } else {
                            palette::NIGHT
                        }),
                    ))
                    .with_children(|card| {
                        card.spawn(icon(atlas, class.sprite(Dir::Down), 96.0));
                        card.spawn(text(
                            class.name(),
                            24.0,
                            if is_selected {
                                class.color()
                            } else {
                                palette::WHITE
                            },
                        ));
                        card.spawn((
                            text(class.tagline(), 14.0, palette::GREY),
                            TextLayout::justify(Justify::Center),
                        ));
                        card.spawn(Node {
                            height: Val::Px(4.0),
                            ..default()
                        });
                        card.spawn(text(
                            format!("F: {}", class.ability_name()),
                            17.0,
                            palette::YELLOW,
                        ));
                        card.spawn((
                            text(class.ability_description(), 14.0, palette::LIGHT_GREY),
                            TextLayout::justify(Justify::Center),
                        ));
                        card.spawn(Node {
                            height: Val::Px(4.0),
                            ..default()
                        });
                        card.spawn((
                            text(class.passive(), 14.0, palette::GREY),
                            TextLayout::justify(Justify::Center),
                        ));
                    });
                }
            });
            root.spawn(text(
                "Left/Right: choose    Enter: descend    Esc: back",
                15.0,
                palette::GREY,
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn input(
    mut commands: Commands,
    controls: Res<Controls>,
    keys: Res<ButtonInput<KeyCode>>,
    atlas: Res<Atlas>,
    mut choice: ResMut<ClassChoice>,
    mut hero: ResMut<Hero>,
    ui: Query<Entity, With<ClassUi>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut next: ResMut<NextState<GameState>>,
    mut start: MessageWriter<StartRun>,
) {
    let n = Class::ALL.len();
    let mut changed = false;
    if controls.left {
        choice.selected = (choice.selected + n - 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.right {
        choice.selected = (choice.selected + 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.confirm {
        sfx.write(PlaySfx(SfxKind::Select));
        start.write(StartRun(Class::ALL[choice.selected]));
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        // Remember the highlighted class for next time.
        hero.class = Class::ALL[choice.selected];
        next.set(GameState::Title);
        return;
    }
    if changed {
        for e in &ui {
            commands.entity(e).despawn();
        }
        build_ui(&mut commands, &atlas, choice.selected);
    }
}
