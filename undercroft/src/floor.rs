//! Turns a generated `Dungeon` into entities and places the hero.

use crate::{
    art::{Atlas, SpriteId},
    combat::{CrackedWallSprite, Spike},
    dungeon::{Spawn, Tile, generate},
    enemy::{FlowField, spawn_enemy},
    game::*,
    hud::Explored,
    items::generate_stock,
    palette,
    physics::tile_center,
    player::{Chest, PickupKind, Player, Shopkeeper, spawn_pickup, spawn_player},
};
use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

pub struct FloorPlugin;

impl Plugin for FloorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), load_floor);
    }
}

#[allow(clippy::too_many_arguments)]
fn load_floor(
    mut commands: Commands,
    hero: Res<Hero>,
    seed: Res<RunSeed>,
    atlas: Res<Atlas>,
    old: Query<Entity, With<FloorEntity>>,
    mut player: Query<&mut Transform, With<Player>>,
    mut field: ResMut<FlowField>,
    mut banner: MessageWriter<Banner>,
    mut notify: MessageWriter<Notify>,
    mut next: ResMut<NextState<GameState>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let d = generate(hero.floor, seed.0);
    let mut rng = StdRng::seed_from_u64(seed.0.wrapping_add(hero.floor as u64 * 7919));

    for y in 0..d.h {
        for x in 0..d.w {
            let p = IVec2::new(x, y);
            let pos = tile_center(p);
            let tile = d.get(p);
            let id = match tile {
                Tile::Wall | Tile::Cracked => {
                    let mut visible = false;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if !d.solid(p + IVec2::new(dx, dy)) {
                                visible = true;
                            }
                        }
                    }
                    if !visible {
                        continue;
                    }
                    if tile == Tile::Cracked {
                        SpriteId::CrackedWall
                    } else if !d.solid(p + IVec2::NEG_Y) {
                        SpriteId::WallFace
                    } else {
                        SpriteId::Wall
                    }
                }
                Tile::Floor | Tile::Spikes => {
                    [SpriteId::Floor0, SpriteId::Floor1, SpriteId::Floor2, SpriteId::Floor3]
                        [((x * 7 + y * 13 + (x * y) % 5) % 4) as usize]
                }
                Tile::Carpet => SpriteId::Carpet,
                Tile::Stairs => SpriteId::Stairs,
                Tile::Rubble => SpriteId::Rubble,
            };
            let frame = ((x + y) % 2) as usize;
            let mut entity = commands.spawn((
                FloorEntity,
                atlas.sprite(id, frame),
                Transform::from_translation(pos.extend(layer::FLOOR)),
            ));
            match tile {
                Tile::Cracked => {
                    entity.insert(CrackedWallSprite(p));
                }
                Tile::Stairs => {
                    entity.insert(Light {
                        radius: 40.0,
                        intensity: 0.8,
                        color: palette::GREEN,
                        flicker: 0.1,
                    });
                }
                Tile::Spikes => {
                    let mut anim = Animation::new(SpriteId::Spikes, 1.0);
                    anim.playing = false;
                    commands.spawn((
                        FloorEntity,
                        Spike {
                            tile: p,
                            timer: rng.random_range(0.0..3.0),
                            up: false,
                        },
                        anim,
                        atlas.sprite(SpriteId::Spikes, 0),
                        Transform::from_translation(pos.extend(layer::DECOR)),
                    ));
                }
                _ => {}
            }
        }
    }

    for (p, spawn) in &d.spawns {
        let pos = tile_center(*p);
        match spawn {
            Spawn::Monster(kind) => {
                let home = d.room_at(*p).map(|i| &d.rooms[i]);
                spawn_enemy(&mut commands, &atlas, *kind, d.floor, pos, home);
            }
            Spawn::Coin => spawn_pickup(&mut commands, &atlas, pos, PickupKind::Coin(2), Vec2::ZERO),
            Spawn::Potion => spawn_pickup(&mut commands, &atlas, pos, PickupKind::Potion, Vec2::ZERO),
            Spawn::ArrowBundle => spawn_pickup(&mut commands, &atlas, pos, PickupKind::Arrows(6), Vec2::ZERO),
            Spawn::Chest(loot) => {
                commands.spawn((
                    FloorEntity,
                    Chest {
                        loot: *loot,
                        open: false,
                    },
                    atlas.sprite(SpriteId::ChestClosed, 0),
                    Transform::from_translation(pos.extend(layer::ITEM)),
                ));
            }
            Spawn::Shopkeeper => {
                let mut anim = Animation::new(SpriteId::Shopkeeper, 0.6);
                anim.timer = rng.random_range(0.0..0.6);
                commands.spawn((
                    FloorEntity,
                    Shopkeeper {
                        stock: generate_stock(&hero, d.floor, &mut rng),
                    },
                    anim,
                    YSort,
                    Light {
                        radius: 70.0,
                        intensity: 0.9,
                        color: palette::ORANGE,
                        flicker: 0.05,
                    },
                    atlas.sprite(SpriteId::Shopkeeper, 0),
                    Transform::from_translation(pos.extend(layer::ACTOR)),
                ));
            }
            Spawn::Torch => {
                let mut anim = Animation::new(SpriteId::Torch, 0.13);
                anim.timer = rng.random_range(0.0..0.13);
                commands.spawn((
                    FloorEntity,
                    anim,
                    Light {
                        radius: 64.0,
                        intensity: 0.85,
                        color: palette::ORANGE,
                        flicker: 0.18,
                    },
                    atlas.sprite(SpriteId::Torch, 0),
                    Transform::from_translation((pos + Vec2::new(0.0, 1.0)).extend(layer::DECOR)),
                ));
            }
        }
    }

    let start = tile_center(d.start);
    if let Ok(mut t) = player.single_mut() {
        t.translation = start.extend(layer::ACTOR);
    } else {
        spawn_player(&mut commands, &atlas, &hero, start);
    }

    commands.insert_resource(Explored::new(d.w, d.h));
    field.dist.clear();
    banner.write(Banner(format!("FLOOR {}", d.floor)));
    notify.write(Notify(match d.floor {
        1 => "Find the key, then the stairs down. J: sword  Shift: sprint  E: interact".into(),
        crate::dungeon::MAX_FLOOR => "The air is cold. Something ancient waits below.".into(),
        3 | 6 => "Heavy footsteps echo near the stairs...".into(),
        _ => format!("Floor {}. The exit is locked; find the key.", d.floor),
    }));
    commands.insert_resource(CurrentFloor {
        dungeon: d,
        boss_sealed: false,
    });
    next.set(GameState::Playing);
}
