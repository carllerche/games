//! Developer helpers, all off unless an environment variable enables them.
//!
//! `UNDERCROFT_SCREENSHOT_DIR=<dir>`: start a run on autopilot, wander the
//! first floor, open the shop and the perk screen, save a series of PNGs
//! into `<dir>`, then quit. Handy for checking the look without a person at
//! the keyboard.
//!
//! `UNDERCROFT_SEED=<n>` fixes the dungeon seed. `UNDERCROFT_AUTOBOW=fire|poison|frost|plain`
//! additionally hands the hero a bow and quivers and has them shoot two
//! knights spawned in front of them, to exercise the status effects.

use crate::{
    game::*,
    input::Controls,
    physics::tile_center,
    player::{Player, ShopContext, Shopkeeper},
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::path::PathBuf;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(dir) = std::env::var("UNDERCROFT_SCREENSHOT_DIR") {
            app.insert_resource(ScreenshotRun {
                dir: PathBuf::from(dir),
                shots: vec![
                    (0.8, "title.png"),
                    (3.0, "floor.png"),
                    (5.5, "explore.png"),
                    (8.0, "shop.png"),
                    (10.0, "perks.png"),
                    (12.0, "gameover.png"),
                ],
                elapsed: 0.0,
                phase: 0,
                spawned_targets: false,
            })
            .add_systems(PreUpdate, autopilot.after(crate::input::read_input))
            .add_systems(Update, (take_screenshots, diagnostics));
        }
    }
}

#[derive(Resource)]
struct ScreenshotRun {
    dir: PathBuf,
    shots: Vec<(f32, &'static str)>,
    elapsed: f32,
    phase: u32,
    spawned_targets: bool,
}

#[allow(clippy::too_many_arguments)]
fn autopilot(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<ScreenshotRun>,
    mut controls: ResMut<Controls>,
    mut hero: ResMut<Hero>,
    atlas: Res<crate::art::Atlas>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    floor: Option<Res<CurrentFloor>>,
    mut player: Query<(Entity, &mut Transform), With<Player>>,
    shopkeepers: Query<(Entity, &Transform), (With<Shopkeeper>, Without<Player>)>,
) {
    run.elapsed += time.delta_secs();
    let t = run.elapsed;
    match state.get() {
        GameState::Title if t > 1.2 => controls.confirm = true,
        GameState::Playing => {
            // Wander in a slow circle, swinging the sword now and then.
            let mode = std::env::var("UNDERCROFT_AUTOBOW").unwrap_or_default();
            if !mode.is_empty() && t < 6.0 {
                // Face right, then stand still and shoot the targets in front.
                controls.move_dir = if t < 2.2 { Vec2::X } else { Vec2::ZERO };
                controls.sprint = false;
            } else {
                let angle = t * 1.3;
                controls.move_dir = Vec2::from_angle(angle);
                controls.sprint = (t * 2.0) as i32 % 3 == 0;
            }
            controls.attack = (mode.is_empty() || t > 6.0) && (t * 10.0) as i32 % 7 == 0;
            // Exercise enchanted arrows: a bow, plenty of ammo, fire loaded.
            if hero.bow.is_none() && !mode.is_empty() {
                hero.bow = Some(1);
                hero.arrows = 99;
                hero.quivers = vec![Element::Fire, Element::Poison, Element::Frost];
                hero.arrow_type = match mode.as_str() {
                    "fire" => Some(Element::Fire),
                    "poison" => Some(Element::Poison),
                    "frost" => Some(Element::Frost),
                    _ => None,
                };
            }
            controls.bow = !mode.is_empty() && t < 3.0 && (t * 10.0) as i32 % 5 == 0;
            // Targets for those arrows: a ring of slimes around the hero.
            if !mode.is_empty() && run.phase == 0 && t > 2.0 && hero.kills == 0 && !run.spawned_targets
                && let Ok((entity, p)) = player.single()
            {
                run.spawned_targets = true;
                // The targets must not end the run: make the hero untouchable.
                commands.entity(entity).insert(Invulnerable(60.0));
                let center = p.translation.truncate();
                for dx in [20.0, 30.0] {
                    crate::enemy::spawn_enemy(&mut commands, &atlas, crate::dungeon::MonsterKind::Knight, 1, center + Vec2::new(dx, 0.0), None);
                }
            }
            if t > 6.5 && run.phase == 0 {
                run.phase = 1;
                if let (Some(floor), Ok((_, mut p))) = (floor.as_ref(), player.single_mut())
                    && let Some((shopkeeper, st)) = shopkeepers.iter().next()
                {
                    let _ = floor;
                    p.translation.x = st.translation.x;
                    p.translation.y = st.translation.y - 18.0;
                    commands.insert_resource(ShopContext { shopkeeper, selected: 1 });
                    next.set(GameState::Shop);
                }
            }
            if t > 9.0 && run.phase == 2 {
                run.phase = 3;
                next.set(GameState::SkillChoice);
            }
            if t > 11.0 && run.phase == 4 {
                run.phase = 5;
                next.set(GameState::GameOver);
            }
        }
        GameState::Shop if t > 8.5 && run.phase == 1 => {
            run.phase = 2;
            controls.cancel = true;
        }
        GameState::SkillChoice if t > 10.5 && run.phase == 3 => {
            run.phase = 4;
            controls.confirm = true;
        }
        GameState::Loading => {
            if let (Some(floor), Ok((_, mut p))) = (floor.as_ref(), player.single_mut()) {
                p.translation = tile_center(floor.dungeon.start).extend(layer::ACTOR);
            }
        }
        _ => {}
    }
}

fn take_screenshots(mut commands: Commands, mut run: ResMut<ScreenshotRun>, mut exit: MessageWriter<AppExit>) {
    if let Some(&(at, name)) = run.shots.first()
        && run.elapsed >= at
    {
        run.shots.remove(0);
        let path = run.dir.join(name);
        info!("saving screenshot to {}", path.display());
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if run.shots.is_empty() && run.elapsed > 13.0 {
        exit.write(AppExit::Success);
    }
}


/// Logs anything that could silently break rendering: non-finite transforms
/// or sprite colours.
fn diagnostics(
    time: Res<Time>,
    mut last: Local<f32>,
    transforms: Query<(Entity, &Transform, Option<&Sprite>, Option<&Name>)>,
    camera: Query<&Transform, With<GameCamera>>,
    hero: Res<Hero>,
    burning: Query<(Entity, &crate::combat::Burning, Option<&crate::enemy::Enemy>, Option<&Player>)>,
    monsters: Query<&Health, With<crate::enemy::Enemy>>,
) {
    *last += time.delta_secs();
    if *last < 1.0 {
        return;
    }
    *last = 0.0;
    let mut bad = 0;
    for (entity, t, sprite, _) in &transforms {
        let finite = t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite();
        let color_ok = sprite.is_none_or(|s| {
            let c = s.color.to_linear();
            c.red.is_finite() && c.green.is_finite() && c.blue.is_finite() && c.alpha.is_finite()
        });
        if !finite || !color_ok {
            bad += 1;
            if bad <= 3 {
                info!("bad entity {entity:?}: {:?} color_ok={color_ok}", t.translation);
            }
        }
    }
    let cam = camera.single().map(|t| t.translation).unwrap_or_default();
    let monster_hp: i32 = monsters.iter().map(|h| h.hp).sum();
    for (e, b, enemy, player) in &burning {
        info!("  burning {e:?} left={:.2} enemy={:?} player={}", b.left, enemy.map(|x| x.kind), player.is_some());
    }
    info!("diag t={:.1} entities={} bad={bad} kills={} burning={} monster_hp={monster_hp} cam={cam:?}", time.elapsed_secs(), transforms.iter().count(), hero.kills, burning.iter().count());
}
