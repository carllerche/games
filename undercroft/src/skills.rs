//! The perk pick between floors: three cards, choose one.

use crate::{
    audio::{PlaySfx, SfxKind},
    game::*,
    hud::text,
    input::Controls,
    items::{Perk, perk_choices},
    palette,
};
use bevy::prelude::*;

#[derive(Resource)]
struct PerkOffer {
    perks: Vec<Perk>,
    selected: usize,
}

#[derive(Component)]
struct SkillUi;

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::SkillChoice), open)
            .add_systems(Update, input.run_if(in_state(GameState::SkillChoice)))
            .add_systems(OnExit(GameState::SkillChoice), close);
    }
}

fn open(mut commands: Commands, mut hero: ResMut<Hero>, mut next: ResMut<NextState<GameState>>) {
    let perks = perk_choices(&hero, &mut rand::rng());
    if perks.is_empty() {
        hero.floor += 1;
        next.set(GameState::Loading);
        return;
    }
    build_ui(&mut commands, &hero, &perks, 0);
    commands.insert_resource(PerkOffer { perks, selected: 0 });
}

fn close(mut commands: Commands, ui: Query<Entity, With<SkillUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<PerkOffer>();
}

fn build_ui(commands: &mut Commands, hero: &Hero, perks: &[Perk], selected: usize) {
    commands
        .spawn((
            SkillUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.7)),
        ))
        .with_children(|root| {
            root.spawn(text(format!("FLOOR {} CLEARED", hero.floor), 36.0, palette::YELLOW));
            root.spawn(text("Choose a perk to take deeper", 18.0, palette::LIGHT_GREY));
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(18.0),
                ..default()
            })
            .with_children(|row| {
                for (i, perk) in perks.iter().enumerate() {
                    let is_selected = i == selected;
                    row.spawn((
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(160.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(if is_selected { palette::YELLOW } else { palette::DARK_GREY }),
                        BackgroundColor(if is_selected { palette::DARKER_GREY } else { palette::NIGHT }),
                    ))
                    .with_children(|card| {
                        card.spawn(text(perk.name(), 20.0, if is_selected { palette::YELLOW } else { palette::WHITE }));
                        card.spawn(text(perk.description(), 15.0, palette::LIGHT_GREY));
                    });
                }
            });
            root.spawn(text("Left/Right: choose    Enter: take it", 14.0, palette::GREY));
        });
}

fn input(
    mut commands: Commands,
    controls: Res<Controls>,
    mut hero: ResMut<Hero>,
    mut offer: ResMut<PerkOffer>,
    ui: Query<Entity, With<SkillUi>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut next: ResMut<NextState<GameState>>,
) {
    let n = offer.perks.len();
    let mut changed = false;
    if controls.left {
        offer.selected = (offer.selected + n - 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.right {
        offer.selected = (offer.selected + 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.confirm {
        let perk = offer.perks[offer.selected];
        hero.perks.push(perk);
        hero.floor += 1;
        sfx.write(PlaySfx(SfxKind::Perk));
        notify.write(Notify(format!("Perk taken: {}. {}", perk.name(), perk.description())));
        next.set(GameState::Loading);
        return;
    }
    if changed {
        for e in &ui {
            commands.entity(e).despawn();
        }
        build_ui(&mut commands, &hero, &offer.perks, offer.selected);
    }
}
