//! Undercroft: a top-down dungeon escape with a modern retro look.
//!
//! Descend eight procedurally generated floors, find each floor's key, buy
//! gear from shopkeepers, pick a perk between floors, and destroy the Lich
//! guarding the way out.

mod art;
mod audio;
mod combat;
mod debug;
mod dungeon;
mod enemy;
mod floor;
mod game;
mod hud;
mod input;
mod items;
mod lighting;
mod palette;
mod physics;
mod player;
mod shop;
mod skills;

use art::TILE;
use bevy::{
    asset::RenderAssetUsages,
    camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::Msaa,
    },
    window::{WindowResized, WindowResolution},
};
use game::*;
use hud::text;
use input::Controls;
use player::Player;
use rand::Rng;

/// The high-resolution layer holds only the upscaled canvas and the UI.
const HIGH_RES: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
struct OuterCamera;

#[derive(Component)]
struct TitleUi;
#[derive(Component)]
struct PauseUi;
#[derive(Component)]
struct EndUi;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Undercroft".into(),
                        resolution: WindowResolution::new(1280, 720),
                        // In the browser, fill the `#game` canvas's parent
                        // (the whole page; see `web/`). `fit_canvas` below
                        // keeps the pixel art at an integer scale.
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<GameState>()
        .insert_resource(ClearColor(palette::VOID))
        .init_resource::<Shake>()
        .init_resource::<Hero>()
        .insert_resource(RunSeed(rand::rng().random()))
        .add_message::<Notify>()
        .add_message::<Banner>()
        .add_plugins((
            input::InputPlugin,
            audio::GameAudioPlugin,
            lighting::LightingPlugin,
            floor::FloorPlugin,
            player::PlayerPlugin,
            enemy::EnemyPlugin,
            combat::CombatPlugin,
            hud::HudPlugin,
            shop::ShopPlugin,
            skills::SkillsPlugin,
            debug::DebugPlugin,
        ))
        .configure_sets(
            Update,
            (Step::Move, Step::Ai, Step::Hits, Step::Damage, Step::Juice, Step::Camera)
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(PreStartup, art::build_atlas)
        .add_systems(Startup, setup_cameras)
        .add_systems(Update, (fit_canvas, animate_sprites, y_sort))
        .add_systems(Update, (camera_follow, decay_shake).in_set(Step::Camera))
        .add_systems(OnEnter(GameState::Title), show_title)
        .add_systems(OnExit(GameState::Title), despawn_all::<TitleUi>)
        .add_systems(Update, title_input.run_if(in_state(GameState::Title)))
        .add_systems(Update, pause_input.run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::Paused), show_pause)
        .add_systems(OnExit(GameState::Paused), despawn_all::<PauseUi>)
        .add_systems(Update, paused_input.run_if(in_state(GameState::Paused)))
        .add_systems(OnEnter(GameState::GameOver), show_game_over)
        .add_systems(OnEnter(GameState::Victory), show_victory)
        .add_systems(OnExit(GameState::GameOver), despawn_all::<EndUi>)
        .add_systems(OnExit(GameState::Victory), despawn_all::<EndUi>)
        .add_systems(
            Update,
            end_input.run_if(in_state(GameState::GameOver).or_else(in_state(GameState::Victory))),
        )
        .run();
}

// ---------------------------------------------------------------------------
// Cameras: the world renders to a small canvas that is scaled up by an
// integer factor, so every pixel stays crisp.
// ---------------------------------------------------------------------------

fn setup_cameras(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: CANVAS_W,
        height: CANVAS_H,
        depth_or_array_layers: 1,
    };
    let mut canvas = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    canvas.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let canvas = images.add(canvas);

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(palette::VOID),
            ..default()
        },
        RenderTarget::Image(canvas.clone().into()),
        Msaa::Off,
        GameCamera,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    commands.spawn((Sprite::from_image(canvas), HIGH_RES));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        OuterCamera,
        IsDefaultUiCamera,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0 / 3.0,
            ..OrthographicProjection::default_2d()
        }),
        HIGH_RES,
    ));
}

fn fit_canvas(
    mut resized: MessageReader<WindowResized>,
    mut projection: Query<&mut Projection, With<OuterCamera>>,
    mut ui_scale: ResMut<UiScale>,
) {
    for event in resized.read() {
        let scale = (event.width / CANVAS_W as f32).min(event.height / CANVAS_H as f32).floor().max(1.0);
        for mut p in &mut projection {
            if let Projection::Orthographic(o) = &mut *p {
                o.scale = 1.0 / scale;
            }
        }
        // The HUD was laid out for the 1280x720 window, where the canvas is
        // scaled by 3; keep it in step with the canvas.
        ui_scale.0 = scale / 3.0;
    }
}

fn camera_follow(
    time: Res<Time>,
    floor: Res<CurrentFloor>,
    shake: Res<Shake>,
    player: Query<&Transform, (With<Player>, Without<GameCamera>)>,
    mut camera: Query<&mut Transform, With<GameCamera>>,
) {
    let (Ok(player), Ok(mut cam)) = (player.single(), camera.single_mut()) else { return };
    let map = Vec2::new(floor.dungeon.w as f32, floor.dungeon.h as f32) * TILE;
    let half = Vec2::new(CANVAS_W as f32, CANVAS_H as f32) / 2.0;
    let mut target = player.translation.truncate();
    target.x = if map.x > half.x * 2.0 { target.x.clamp(half.x, map.x - half.x) } else { map.x / 2.0 };
    target.y = if map.y > half.y * 2.0 { target.y.clamp(half.y, map.y - half.y) } else { map.y / 2.0 };
    let current = cam.translation.truncate();
    let t = 1.0 - (-9.0 * time.delta_secs()).exp();
    let mut pos = if current.distance(target) > 260.0 { target } else { current.lerp(target, t) };
    let mut rng = rand::rng();
    let amount = shake.trauma * shake.trauma * 5.0;
    pos += Vec2::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0)) * amount;
    cam.translation = pos.round().extend(cam.translation.z);
}

fn decay_shake(time: Res<Time>, mut shake: ResMut<Shake>) {
    shake.trauma = (shake.trauma - time.delta_secs() * 1.8).max(0.0);
}

fn animate_sprites(time: Res<Time>, mut q: Query<(&mut Animation, &mut Sprite)>) {
    for (mut anim, mut sprite) in &mut q {
        if anim.playing {
            anim.timer += time.delta_secs();
            if anim.timer >= anim.frame_time {
                anim.timer -= anim.frame_time;
                anim.frame ^= 1;
            }
        }
        let index = anim.id.index(anim.frame);
        if let Some(atlas) = sprite.texture_atlas.as_mut()
            && atlas.index != index
        {
            atlas.index = index;
        }
    }
}

fn y_sort(floor: Option<Res<CurrentFloor>>, mut q: Query<&mut Transform, With<YSort>>) {
    let Some(floor) = floor else { return };
    let height = floor.dungeon.h as f32 * TILE;
    for mut t in &mut q {
        t.translation.z = layer::ACTOR + (1.0 - t.translation.y / height).clamp(0.0, 1.0) * 8.0;
    }
}

fn despawn_all<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------

fn overlay(marker: impl Component, dim: f32) -> impl Bundle {
    (
        marker,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(dim)),
    )
}

fn show_title(mut commands: Commands) {
    commands.spawn(overlay(TitleUi, 0.0)).with_children(|root| {
        root.spawn(text("UNDERCROFT", 72.0, palette::CARPET_TRIM));
        root.spawn(text("Escape the dungeon. Eight floors down, one way out.", 20.0, palette::LIGHT_GREY));
        root.spawn(Node {
            height: Val::Px(24.0),
            ..default()
        });
        for line in [
            "WASD / Arrows  move",
            "Shift  sprint (drains energy)",
            "J / Space  sword          K  bow",
            "Q  drink potion            E  interact / trade",
            "Esc  pause                 Gamepad supported",
        ] {
            root.spawn(text(line, 17.0, palette::GREY));
        }
        root.spawn(Node {
            height: Val::Px(24.0),
            ..default()
        });
        root.spawn(text("Press ENTER to descend", 24.0, palette::YELLOW));
    });
}

fn start_run(
    commands: &mut Commands,
    hero: &mut Hero,
    seed: &mut RunSeed,
    run_entities: &Query<Entity, With<RunEntity>>,
    next: &mut NextState<GameState>,
) {
    for e in run_entities {
        commands.entity(e).despawn();
    }
    *hero = Hero::default();
    seed.0 = std::env::var("UNDERCROFT_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| rand::rng().random());
    next.set(GameState::Loading);
}

fn title_input(
    mut commands: Commands,
    controls: Res<Controls>,
    keys: Res<ButtonInput<KeyCode>>,
    mut hero: ResMut<Hero>,
    mut seed: ResMut<RunSeed>,
    run_entities: Query<Entity, With<RunEntity>>,
    mut next: ResMut<NextState<GameState>>,
    mut sfx: MessageWriter<audio::PlaySfx>,
    mut exit: MessageWriter<AppExit>,
) {
    // Quitting only makes sense on desktop; in the browser it would leave a
    // frozen canvas.
    if cfg!(not(target_arch = "wasm32")) && keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    if controls.confirm {
        sfx.write(audio::PlaySfx(audio::SfxKind::Select));
        start_run(&mut commands, &mut hero, &mut seed, &run_entities, &mut next);
    }
}

fn pause_input(controls: Res<Controls>, mut next: ResMut<NextState<GameState>>) {
    if controls.pause {
        next.set(GameState::Paused);
    }
}

fn show_pause(mut commands: Commands, hero: Res<Hero>) {
    commands.spawn(overlay(PauseUi, 0.6)).with_children(|root| {
        root.spawn(text("PAUSED", 48.0, palette::YELLOW));
        root.spawn(text(
            format!("Floor {}   Level {}   {} coins   {} kills", hero.floor, hero.level, hero.coins, hero.kills),
            18.0,
            palette::LIGHT_GREY,
        ));
        if !hero.perks.is_empty() {
            let perks = hero.perks.iter().map(|p| format!("{}: {}", p.name(), p.description())).collect::<Vec<_>>();
            for p in perks {
                root.spawn(text(p, 15.0, palette::GREY));
            }
        }
        root.spawn(Node {
            height: Val::Px(16.0),
            ..default()
        });
        root.spawn(text("Esc: resume        T: abandon run", 17.0, palette::GREY));
    });
}

fn paused_input(controls: Res<Controls>, keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<GameState>>) {
    if controls.pause {
        next.set(GameState::Playing);
    }
    if keys.just_pressed(KeyCode::KeyT) {
        next.set(GameState::GameOver);
    }
}

fn show_game_over(mut commands: Commands, hero: Res<Hero>) {
    commands.spawn(overlay(EndUi, 0.75)).with_children(|root| {
        root.spawn(text("YOU DIED", 64.0, palette::RED));
        root.spawn(text(
            format!("You reached floor {} at level {}.", hero.floor, hero.level),
            20.0,
            palette::LIGHT_GREY,
        ));
        root.spawn(text(
            format!("{} monsters slain, {} coins in your pocket.", hero.kills, hero.coins),
            18.0,
            palette::GREY,
        ));
        root.spawn(Node {
            height: Val::Px(24.0),
            ..default()
        });
        root.spawn(text("Enter: descend again        Esc: title", 20.0, palette::YELLOW));
    });
}

fn show_victory(mut commands: Commands, hero: Res<Hero>, mut sfx: MessageWriter<audio::PlaySfx>) {
    sfx.write(audio::PlaySfx(audio::SfxKind::Victory));
    commands.spawn(overlay(EndUi, 0.75)).with_children(|root| {
        root.spawn(text("YOU ESCAPED", 64.0, palette::YELLOW));
        root.spawn(text("Daylight. You never thought you'd see it again.", 20.0, palette::LIGHT_GREY));
        root.spawn(text(
            format!("Level {}, {} monsters slain, {} coins to your name.", hero.level, hero.kills, hero.coins),
            18.0,
            palette::GREY,
        ));
        root.spawn(Node {
            height: Val::Px(24.0),
            ..default()
        });
        root.spawn(text("Enter: play again        Esc: title", 20.0, palette::YELLOW));
    });
}

#[allow(clippy::too_many_arguments)]
fn end_input(
    mut commands: Commands,
    controls: Res<Controls>,
    keys: Res<ButtonInput<KeyCode>>,
    mut hero: ResMut<Hero>,
    mut seed: ResMut<RunSeed>,
    run_entities: Query<Entity, With<RunEntity>>,
    floor_entities: Query<Entity, With<FloorEntity>>,
    mut next: ResMut<NextState<GameState>>,
) {
    if controls.confirm {
        start_run(&mut commands, &mut hero, &mut seed, &run_entities, &mut next);
    } else if keys.just_pressed(KeyCode::Escape) {
        for e in run_entities.iter().chain(floor_entities.iter()) {
            commands.entity(e).despawn();
        }
        commands.remove_resource::<CurrentFloor>();
        next.set(GameState::Title);
    }
}
