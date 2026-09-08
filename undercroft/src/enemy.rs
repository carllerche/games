//! Monsters and their behaviours.

use crate::{
    art::{Atlas, SpriteId, TILE},
    audio::{PlaySfx, SfxKind},
    combat::{BaseColor, BossDoorSprite, Faction, Frozen, spawn_particles, spawn_projectile},
    dungeon::{MAX_FLOOR, MonsterKind, Room, RoomKind, Tile},
    game::*,
    palette,
    physics::{flow_direction, flow_field, line_of_sight, move_box, tile_center, tile_of},
    player::Player,
};
use bevy::prelude::*;
use rand::{Rng, seq::IndexedRandom};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AiState {
    Idle,
    Chase,
    Windup,
    Charge(Vec2),
    Rest,
}

#[derive(Component)]
pub struct Enemy {
    pub kind: MonsterKind,
    pub speed: f32,
    pub contact_damage: i32,
    pub xp: u32,
    pub coins: (u32, u32),
    pub radius: f32,
    pub armor: i32,
    pub blood: Color,
    pub vulnerable: bool,
    pub aggro: bool,
    pub state: AiState,
    pub timer: f32,
    pub cooldown: f32,
    pub wander: Vec2,
    pub summon_timer: f32,
    pub teleport_timer: f32,
    /// World-space bounds of the room this monster was spawned in. Monsters
    /// never leave their room.
    pub home: Option<Rect>,
}

#[derive(Component)]
pub struct Boss;

#[derive(Resource, Default)]
pub struct FlowField {
    pub dist: Vec<u16>,
    pub timer: f32,
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlowField>().add_systems(
            Update,
            (update_flow_field, enemy_ai, lich_magic, boss_room_seal)
                .chain()
                .in_set(Step::Ai),
        );
    }
}

struct Stats {
    hp: i32,
    speed: f32,
    contact: i32,
    xp: u32,
    coins: (u32, u32),
    sprite: SpriteId,
    scale: f32,
    radius: f32,
    armor: i32,
    blood: Color,
    anim: f32,
}

fn stats(kind: MonsterKind) -> Stats {
    use MonsterKind::*;
    match kind {
        Slime => Stats {
            hp: 5,
            speed: 120.0,
            contact: 2,
            xp: 4,
            coins: (1, 3),
            sprite: SpriteId::Slime,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::LIME,
            anim: 0.35,
        },
        Bat => Stats {
            hp: 3,
            speed: 85.0,
            contact: 1,
            xp: 3,
            coins: (0, 2),
            sprite: SpriteId::Bat,
            scale: 1.0,
            radius: 5.0,
            armor: 0,
            blood: palette::PURPLE,
            anim: 0.08,
        },
        Skeleton => Stats {
            hp: 8,
            speed: 52.0,
            contact: 3,
            xp: 7,
            coins: (2, 4),
            sprite: SpriteId::Skeleton,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::BONE,
            anim: 0.18,
        },
        Archer => Stats {
            hp: 6,
            speed: 48.0,
            contact: 2,
            xp: 8,
            coins: (2, 5),
            sprite: SpriteId::Archer,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::BONE,
            anim: 0.18,
        },
        Spider => Stats {
            hp: 6,
            speed: 175.0,
            contact: 3,
            xp: 8,
            coins: (1, 4),
            sprite: SpriteId::Spider,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::DARK_GREEN,
            anim: 0.1,
        },
        Ghost => Stats {
            hp: 7,
            speed: 42.0,
            contact: 3,
            xp: 10,
            coins: (3, 6),
            sprite: SpriteId::Ghost,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::CYAN,
            anim: 0.3,
        },
        Knight => Stats {
            hp: 18,
            speed: 38.0,
            contact: 5,
            xp: 16,
            coins: (5, 10),
            sprite: SpriteId::Knight,
            scale: 1.0,
            radius: 7.0,
            armor: 2,
            blood: palette::RED,
            anim: 0.25,
        },
        Ogre => Stats {
            hp: 60,
            speed: 34.0,
            contact: 6,
            xp: 60,
            coins: (14, 22),
            sprite: SpriteId::Ogre,
            scale: 2.0,
            radius: 13.0,
            armor: 1,
            blood: palette::DARK_GREEN,
            anim: 0.3,
        },
        Rat => Stats {
            hp: 3,
            speed: 78.0,
            contact: 1,
            xp: 2,
            coins: (0, 2),
            sprite: SpriteId::Rat,
            scale: 1.0,
            radius: 5.0,
            armor: 0,
            blood: palette::DARK_RED,
            anim: 0.1,
        },
        Kobold => Stats {
            hp: 5,
            speed: 46.0,
            contact: 2,
            xp: 6,
            coins: (2, 4),
            sprite: SpriteId::Kobold,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::DARK_RED,
            anim: 0.2,
        },
        Imp => Stats {
            hp: 5,
            speed: 70.0,
            contact: 2,
            xp: 9,
            coins: (2, 5),
            sprite: SpriteId::Imp,
            scale: 1.0,
            radius: 5.0,
            armor: 0,
            blood: palette::DARK_ORANGE,
            anim: 0.1,
        },
        Cultist => Stats {
            hp: 8,
            speed: 30.0,
            contact: 2,
            xp: 12,
            coins: (4, 7),
            sprite: SpriteId::Cultist,
            scale: 1.0,
            radius: 6.0,
            armor: 0,
            blood: palette::PURPLE,
            anim: 0.3,
        },
        Golem => Stats {
            hp: 32,
            speed: 26.0,
            contact: 6,
            xp: 26,
            coins: (6, 12),
            sprite: SpriteId::Golem,
            scale: 1.0,
            radius: 7.0,
            armor: 2,
            blood: palette::GREY,
            anim: 0.35,
        },
        Lich => Stats {
            hp: 170,
            speed: 0.0,
            contact: 5,
            xp: 200,
            coins: (20, 30),
            sprite: SpriteId::Lich,
            scale: 2.0,
            radius: 13.0,
            armor: 0,
            blood: palette::PURPLE,
            anim: 0.4,
        },
    }
}

pub fn spawn_enemy(
    commands: &mut Commands,
    atlas: &Atlas,
    kind: MonsterKind,
    floor: u32,
    pos: Vec2,
    home: Option<&Room>,
) {
    let s = stats(kind);
    let hp = (s.hp as f32 * (1.0 + 0.12 * (floor as f32 - 1.0))).round() as i32;
    let contact = s.contact + (floor as i32 - 1) / 3;
    let mut rng = rand::rng();
    let mut anim = Animation::new(s.sprite, s.anim);
    anim.timer = rng.random_range(0.0..s.anim);
    let mut entity = commands.spawn((
        FloorEntity,
        Enemy {
            kind,
            speed: s.speed,
            contact_damage: contact,
            xp: s.xp,
            coins: s.coins,
            radius: s.radius,
            armor: s.armor,
            blood: s.blood,
            vulnerable: true,
            aggro: false,
            state: AiState::Idle,
            timer: rng.random_range(0.0..1.0),
            cooldown: rng.random_range(0.5..1.5),
            wander: Vec2::ZERO,
            summon_timer: 6.0,
            teleport_timer: 3.0,
            home: home.map(room_bounds),
        },
        Health { hp, max: hp },
        Hitbox(Vec2::splat(s.radius * 0.8)),
        anim,
        YSort,
        BaseColor(Color::WHITE),
        atlas.sprite(s.sprite, 0),
        Transform::from_translation(pos.extend(layer::ACTOR)).with_scale(Vec3::splat(s.scale)),
    ));
    match kind {
        MonsterKind::Ghost => {
            entity.insert(Light {
                radius: 34.0,
                intensity: 0.7,
                color: palette::CYAN,
                flicker: 0.3,
            });
        }
        MonsterKind::Lich => {
            entity.insert((
                Boss,
                Light {
                    radius: 60.0,
                    intensity: 0.9,
                    color: palette::PURPLE,
                    flicker: 0.2,
                },
            ));
        }
        MonsterKind::Ogre => {
            entity.insert(Boss);
        }
        MonsterKind::Imp => {
            entity.insert(Light {
                radius: 24.0,
                intensity: 0.6,
                color: palette::DARK_ORANGE,
                flicker: 0.3,
            });
        }
        _ => {}
    }
}

/// World-space rectangle covering a room's floor tiles.
pub fn room_bounds(room: &Room) -> Rect {
    Rect::new(
        room.x as f32 * TILE,
        room.y as f32 * TILE,
        (room.x + room.w) as f32 * TILE,
        (room.y + room.h) as f32 * TILE,
    )
}

fn update_flow_field(
    time: Res<Time>,
    floor: Res<CurrentFloor>,
    player: Query<&Transform, With<Player>>,
    mut field: ResMut<FlowField>,
) {
    field.timer -= time.delta_secs();
    if field.timer > 0.0 && !field.dist.is_empty() {
        return;
    }
    field.timer = 0.12;
    let Ok(t) = player.single() else { return };
    field.dist = flow_field(&floor.dungeon, tile_of(t.translation.truncate()), 40);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    atlas: Res<Atlas>,
    floor: Res<CurrentFloor>,
    field: Res<FlowField>,
    mut shake: ResMut<Shake>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(
        &mut Transform,
        &mut Enemy,
        &Hitbox,
        &mut Animation,
        &mut Sprite,
        &mut BaseColor,
        Option<&Knockback>,
        Option<&Frozen>,
    )>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let Ok(pt) = player.single() else { return };
    let ppos = pt.translation.truncate();
    let mut rng = rand::rng();
    let d = &floor.dungeon;

    for (mut transform, mut enemy, hitbox, mut anim, mut sprite, mut base, knockback, frozen) in
        &mut enemies
    {
        if frozen.is_some() {
            // Frozen solid: no thinking, no moving, no cooldowns.
            anim.playing = false;
            continue;
        }
        let pos = transform.translation.truncate();
        let to_player = ppos - pos;
        let dir_to_player = to_player.normalize_or_zero();
        // Monsters only care about the hero while they share a room (the
        // doorway tiles count), so nothing follows you down a corridor.
        let in_home = enemy
            .home
            .is_none_or(|home| home.inflate(TILE).contains(ppos));
        let dist = if in_home {
            to_player.length()
        } else {
            f32::INFINITY
        };
        if !in_home {
            enemy.aggro = false;
        }
        enemy.timer -= dt;
        enemy.cooldown -= dt;

        let mut velocity = Vec2::ZERO;
        let mut through_walls = false;

        match enemy.kind {
            MonsterKind::Slime => {
                match enemy.state {
                    AiState::Idle => {
                        if enemy.timer <= 0.0 {
                            let dir = if dist < 150.0 {
                                let f = flow_direction(d, &field.dist, pos);
                                if f == Vec2::ZERO { dir_to_player } else { f }
                            } else {
                                Vec2::from_angle(rng.random_range(0.0..std::f32::consts::TAU))
                            };
                            enemy.state = AiState::Charge(dir);
                            enemy.timer = 0.3;
                        }
                    }
                    AiState::Charge(dir) => {
                        velocity = dir * enemy.speed;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Idle;
                            enemy.timer = rng.random_range(0.5..0.9);
                        }
                    }
                    _ => enemy.state = AiState::Idle,
                }
                anim.playing = matches!(enemy.state, AiState::Charge(_));
                if !anim.playing {
                    anim.frame = 1;
                }
            }
            MonsterKind::Bat => {
                if dist < 170.0 || enemy.aggro {
                    enemy.aggro = true;
                    let wobble = dir_to_player.perp() * (t * 7.0 + pos.x).sin();
                    velocity = (dir_to_player + wobble * 0.8).normalize_or_zero() * enemy.speed;
                } else {
                    if enemy.timer <= 0.0 {
                        enemy.timer = rng.random_range(0.6..1.4);
                        enemy.wander =
                            Vec2::from_angle(rng.random_range(0.0..std::f32::consts::TAU));
                    }
                    velocity = enemy.wander * 35.0;
                }
            }
            MonsterKind::Skeleton => {
                if dist < 150.0 && line_of_sight(d, pos, ppos) {
                    enemy.aggro = true;
                    enemy.timer = 4.0;
                } else if enemy.timer <= 0.0 {
                    enemy.aggro = false;
                }
                if enemy.aggro {
                    let f = flow_direction(d, &field.dist, pos);
                    velocity = if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                }
            }
            MonsterKind::Archer => {
                let los = line_of_sight(d, pos, ppos);
                if dist < 180.0 && los {
                    enemy.aggro = true;
                    enemy.timer = 4.0;
                } else if enemy.timer <= 0.0 {
                    enemy.aggro = false;
                }
                if enemy.aggro {
                    if dist < 70.0 {
                        velocity = -dir_to_player * enemy.speed;
                    } else if dist > 120.0 || !los {
                        let f = flow_direction(d, &field.dist, pos);
                        velocity = if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                    }
                    if los && enemy.cooldown <= 0.0 && dist < 200.0 {
                        enemy.cooldown = 1.9;
                        spawn_projectile(
                            &mut commands,
                            &atlas,
                            SpriteId::Arrow,
                            pos + dir_to_player * 8.0,
                            dir_to_player,
                            150.0,
                            2 + (d.floor as i32 - 1) / 3,
                            Faction::Monster,
                            false,
                            None,
                        );
                        sfx.write(PlaySfx(SfxKind::Bow));
                    }
                }
            }
            MonsterKind::Spider => match enemy.state {
                AiState::Idle => {
                    anim.playing = false;
                    if dist < 70.0 && line_of_sight(d, pos, ppos) {
                        enemy.state = AiState::Windup;
                        enemy.timer = 0.25;
                    }
                }
                AiState::Windup => {
                    anim.playing = true;
                    if enemy.timer <= 0.0 {
                        enemy.state = AiState::Charge(dir_to_player);
                        enemy.timer = 0.45;
                    }
                }
                AiState::Charge(dir) => {
                    velocity = dir * enemy.speed;
                    if enemy.timer <= 0.0 {
                        enemy.state = AiState::Rest;
                        enemy.timer = 0.7;
                    }
                }
                AiState::Rest => {
                    anim.playing = false;
                    if enemy.timer <= 0.0 {
                        enemy.state = AiState::Idle;
                    }
                }
                AiState::Chase => enemy.state = AiState::Idle,
            },
            MonsterKind::Ghost => {
                through_walls = true;
                let phase = 0.5 + 0.5 * (t * 1.4 + pos.x * 0.01).sin();
                let alpha = 0.25 + 0.75 * phase;
                base.0 = Color::WHITE.with_alpha(alpha);
                enemy.vulnerable = alpha > 0.6;
                if dist < 220.0 {
                    velocity = dir_to_player * enemy.speed * (0.4 + phase);
                }
            }
            MonsterKind::Knight => {
                if dist < 140.0 && line_of_sight(d, pos, ppos) {
                    enemy.aggro = true;
                }
                match enemy.state {
                    AiState::Idle | AiState::Chase => {
                        if enemy.aggro {
                            enemy.state = AiState::Chase;
                            let f = flow_direction(d, &field.dist, pos);
                            velocity =
                                if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                            if dist < 34.0 && enemy.cooldown <= 0.0 {
                                enemy.state = AiState::Windup;
                                enemy.timer = 0.4;
                            }
                        }
                    }
                    AiState::Windup => {
                        anim.playing = false;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Charge(dir_to_player);
                            enemy.timer = 0.22;
                            sfx.write(PlaySfx(SfxKind::Swing));
                        }
                    }
                    AiState::Charge(dir) => {
                        anim.playing = true;
                        velocity = dir * 210.0;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Rest;
                            enemy.timer = 0.6;
                            enemy.cooldown = 1.2;
                        }
                    }
                    AiState::Rest => {
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Chase;
                        }
                    }
                }
            }
            MonsterKind::Ogre => {
                if dist < 200.0 {
                    enemy.aggro = true;
                }
                match enemy.state {
                    AiState::Idle | AiState::Chase => {
                        if enemy.aggro {
                            let f = flow_direction(d, &field.dist, pos);
                            velocity =
                                if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                            if enemy.cooldown <= 0.0 && line_of_sight(d, pos, ppos) {
                                enemy.state = AiState::Windup;
                                enemy.timer = 0.7;
                                sfx.write(PlaySfx(SfxKind::Roar));
                            }
                        }
                    }
                    AiState::Windup => {
                        anim.playing = false;
                        velocity = Vec2::ZERO;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Charge(dir_to_player);
                            enemy.timer = 1.1;
                        }
                    }
                    AiState::Charge(dir) => {
                        anim.playing = true;
                        velocity = dir * 190.0;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Rest;
                            enemy.timer = 0.5;
                            enemy.cooldown = 1.5;
                        }
                    }
                    AiState::Rest => {
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Chase;
                        }
                    }
                }
            }
            MonsterKind::Rat => {
                if dist < 120.0 && line_of_sight(d, pos, ppos) {
                    enemy.aggro = true;
                    enemy.timer = 2.5;
                } else if enemy.timer <= 0.0 {
                    enemy.aggro = false;
                }
                if enemy.aggro {
                    let f = flow_direction(d, &field.dist, pos);
                    let base = if f == Vec2::ZERO { dir_to_player } else { f };
                    let jitter = base.perp() * (t * 11.0 + pos.y * 0.3).sin() * 0.5;
                    velocity = (base + jitter).normalize_or_zero() * enemy.speed;
                } else {
                    if enemy.cooldown <= 0.0 {
                        enemy.cooldown = rng.random_range(0.8..2.0);
                        enemy.wander = if rng.random_bool(0.6) {
                            Vec2::from_angle(rng.random_range(0.0..std::f32::consts::TAU))
                        } else {
                            Vec2::ZERO
                        };
                    }
                    velocity = enemy.wander * 30.0;
                }
            }
            MonsterKind::Kobold => {
                let los = line_of_sight(d, pos, ppos);
                if dist < 170.0 && los {
                    enemy.aggro = true;
                    enemy.timer = 3.0;
                } else if enemy.timer <= 0.0 {
                    enemy.aggro = false;
                }
                if enemy.aggro {
                    if dist < 60.0 {
                        velocity = -dir_to_player * enemy.speed;
                    } else if dist > 110.0 || !los {
                        let f = flow_direction(d, &field.dist, pos);
                        velocity = if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                    }
                    if los && enemy.cooldown <= 0.0 && dist < 160.0 {
                        enemy.cooldown = 2.2;
                        // A lobbed stone: slow, so it can be dodged.
                        spawn_projectile(
                            &mut commands,
                            &atlas,
                            SpriteId::Rock,
                            pos + dir_to_player * 8.0,
                            dir_to_player,
                            115.0,
                            2,
                            Faction::Monster,
                            false,
                            None,
                        );
                        sfx.write(PlaySfx(SfxKind::Swing));
                    }
                }
            }
            MonsterKind::Imp => {
                if dist < 190.0 {
                    enemy.aggro = true;
                }
                if enemy.aggro {
                    // Circle the hero at a distance and spit fire.
                    let orbit = dir_to_player.perp()
                        * if (pos.x as i32 / 40) % 2 == 0 {
                            1.0
                        } else {
                            -1.0
                        };
                    let range = if dist < 60.0 {
                        -1.0
                    } else if dist > 100.0 {
                        1.0
                    } else {
                        0.0
                    };
                    velocity = (orbit + dir_to_player * range).normalize_or_zero() * enemy.speed;
                    if enemy.cooldown <= 0.0 && dist < 150.0 && line_of_sight(d, pos, ppos) {
                        enemy.cooldown = 2.4;
                        spawn_projectile(
                            &mut commands,
                            &atlas,
                            SpriteId::Fireball,
                            pos + dir_to_player * 8.0,
                            dir_to_player,
                            135.0,
                            2,
                            Faction::Monster,
                            false,
                            Some(Element::Fire),
                        );
                        sfx.write(PlaySfx(SfxKind::Bow));
                    }
                } else {
                    if enemy.timer <= 0.0 {
                        enemy.timer = rng.random_range(0.6..1.4);
                        enemy.wander =
                            Vec2::from_angle(rng.random_range(0.0..std::f32::consts::TAU));
                    }
                    velocity = enemy.wander * 30.0;
                }
            }
            MonsterKind::Cultist => {
                let los = line_of_sight(d, pos, ppos);
                if dist < 200.0 && los {
                    enemy.aggro = true;
                }
                if enemy.aggro {
                    if dist < 48.0 && enemy.timer <= 0.0 {
                        // Blink away to a random spot in the room.
                        enemy.timer = 3.0;
                        if let Some(home) = enemy.home {
                            let mut tiles = Vec::new();
                            let (lo, hi) =
                                (tile_of(home.min), tile_of(home.max - Vec2::splat(1.0)));
                            for y in lo.y..=hi.y {
                                for x in lo.x..=hi.x {
                                    let p = IVec2::new(x, y);
                                    if d.get(p) == Tile::Floor
                                        && tile_center(p).distance(ppos) > 70.0
                                    {
                                        tiles.push(p);
                                    }
                                }
                            }
                            if let Some(p) = tiles.choose(&mut rng) {
                                spawn_particles(&mut commands, pos, palette::PURPLE, 10, 50.0, 0.4);
                                let target = tile_center(*p);
                                transform.translation.x = target.x;
                                transform.translation.y = target.y;
                                spawn_particles(
                                    &mut commands,
                                    target,
                                    palette::PURPLE,
                                    10,
                                    50.0,
                                    0.4,
                                );
                                sfx.write(PlaySfx(SfxKind::Teleport));
                                enemy.cooldown = 0.8;
                            }
                        }
                    } else if !los {
                        let f = flow_direction(d, &field.dist, pos);
                        velocity = if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                    }
                    if los && enemy.cooldown <= 0.0 && dist < 190.0 {
                        enemy.cooldown = 2.6;
                        for spread in [-0.22, 0.0, 0.22] {
                            let dir = Vec2::from_angle(spread).rotate(dir_to_player);
                            spawn_projectile(
                                &mut commands,
                                &atlas,
                                SpriteId::Bolt,
                                pos + dir * 8.0,
                                dir,
                                105.0,
                                3,
                                Faction::Monster,
                                false,
                                None,
                            );
                        }
                        sfx.write(PlaySfx(SfxKind::Teleport));
                    }
                }
            }
            MonsterKind::Golem => {
                if dist < 160.0 {
                    enemy.aggro = true;
                }
                match enemy.state {
                    AiState::Idle | AiState::Chase => {
                        if enemy.aggro {
                            enemy.state = AiState::Chase;
                            let f = flow_direction(d, &field.dist, pos);
                            velocity =
                                if f == Vec2::ZERO { dir_to_player } else { f } * enemy.speed;
                            if dist < 30.0 && enemy.cooldown <= 0.0 {
                                enemy.state = AiState::Windup;
                                enemy.timer = 0.55;
                            }
                        }
                    }
                    AiState::Windup => {
                        anim.playing = false;
                        if enemy.timer <= 0.0 {
                            // Ground slam: a short lunge plus a tremor.
                            enemy.state = AiState::Charge(dir_to_player);
                            enemy.timer = 0.18;
                            shake.add(0.35);
                            spawn_particles(
                                &mut commands,
                                pos + Vec2::new(0.0, -6.0),
                                palette::GREY,
                                8,
                                50.0,
                                0.4,
                            );
                            sfx.write(PlaySfx(SfxKind::Break));
                        }
                    }
                    AiState::Charge(dir) => {
                        anim.playing = true;
                        velocity = dir * 170.0;
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Rest;
                            enemy.timer = 0.9;
                            enemy.cooldown = 1.6;
                        }
                    }
                    AiState::Rest => {
                        if enemy.timer <= 0.0 {
                            enemy.state = AiState::Chase;
                        }
                    }
                }
            }
            MonsterKind::Lich => {
                // Handled in `lich_magic`; the Lich never walks.
                enemy.aggro = true;
            }
        }

        if velocity != Vec2::ZERO && knockback.is_none() {
            if through_walls {
                let mut p = pos + velocity * dt;
                p.x = p.x.clamp(8.0, d.w as f32 * 16.0 - 8.0);
                p.y = p.y.clamp(8.0, d.h as f32 * 16.0 - 8.0);
                transform.translation.x = p.x;
                transform.translation.y = p.y;
            } else {
                let (new_pos, blocked) = move_box(d, pos, velocity * dt, hitbox.0);
                transform.translation.x = new_pos.x;
                transform.translation.y = new_pos.y;
                if blocked.any() {
                    match enemy.state {
                        AiState::Charge(_) if enemy.kind == MonsterKind::Ogre => {
                            // Slamming into a wall stuns the ogre.
                            enemy.state = AiState::Rest;
                            enemy.timer = 1.3;
                            enemy.cooldown = 2.0;
                            shake.add(0.5);
                            spawn_particles(&mut commands, new_pos, palette::GREY, 10, 60.0, 0.5);
                            sfx.write(PlaySfx(SfxKind::Break));
                        }
                        AiState::Charge(_) if enemy.kind == MonsterKind::Spider => {
                            enemy.state = AiState::Rest;
                            enemy.timer = 0.5;
                        }
                        _ if enemy.kind == MonsterKind::Bat => {
                            enemy.wander = -enemy.wander;
                        }
                        _ => {}
                    }
                }
            }
            if velocity.x.abs() > 0.1 {
                sprite.flip_x = velocity.x < 0.0;
            }
            if !matches!(
                enemy.kind,
                MonsterKind::Slime
                    | MonsterKind::Spider
                    | MonsterKind::Knight
                    | MonsterKind::Ogre
                    | MonsterKind::Golem
            ) {
                anim.playing = true;
            }
        } else if matches!(
            enemy.kind,
            MonsterKind::Skeleton
                | MonsterKind::Archer
                | MonsterKind::Rat
                | MonsterKind::Kobold
                | MonsterKind::Cultist
        ) {
            anim.playing = false;
            anim.frame = 0;
        }

        // Whatever happened, stay inside the home room.
        if let Some(home) = enemy.home {
            let r = enemy.radius;
            transform.translation.x = transform
                .translation
                .x
                .clamp(home.min.x + r, home.max.x - r);
            transform.translation.y = transform
                .translation
                .y
                .clamp(home.min.y + r, home.max.y - r);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lich_magic(
    mut commands: Commands,
    time: Res<Time>,
    atlas: Res<Atlas>,
    floor: Res<CurrentFloor>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut liches: Query<(&mut Transform, &mut Enemy), With<Boss>>,
    minions: Query<&Enemy, Without<Boss>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
) {
    let dt = time.delta_secs();
    let Ok(pt) = player.single() else { return };
    let ppos = pt.translation.truncate();
    let mut rng = rand::rng();
    let d = &floor.dungeon;
    let Some(room) = d.room_of_kind(RoomKind::Boss) else {
        return;
    };

    for (mut transform, mut enemy) in &mut liches {
        if enemy.kind != MonsterKind::Lich {
            continue;
        }
        let pos = transform.translation.truncate();
        let awake = room.contains(tile_of(ppos));
        if !awake {
            continue;
        }
        enemy.teleport_timer -= dt;
        enemy.summon_timer -= dt;

        if enemy.cooldown <= 0.0 {
            enemy.cooldown = if enemy.timer <= 0.0 { 1.4 } else { 2.2 };
            let dmg = 4;
            if rng.random_bool(0.5) {
                for i in 0..8 {
                    let dir = Vec2::from_angle(
                        i as f32 / 8.0 * std::f32::consts::TAU + time.elapsed_secs(),
                    );
                    spawn_projectile(
                        &mut commands,
                        &atlas,
                        SpriteId::Bolt,
                        pos + dir * 12.0,
                        dir,
                        95.0,
                        dmg,
                        Faction::Monster,
                        false,
                        None,
                    );
                }
            } else {
                let aim = (ppos - pos).normalize_or_zero();
                for spread in [-0.25, 0.0, 0.25] {
                    let dir = Vec2::from_angle(spread).rotate(aim);
                    spawn_projectile(
                        &mut commands,
                        &atlas,
                        SpriteId::Bolt,
                        pos + dir * 12.0,
                        dir,
                        140.0,
                        dmg,
                        Faction::Monster,
                        false,
                        None,
                    );
                }
            }
            sfx.write(PlaySfx(SfxKind::Teleport));
        }

        if enemy.teleport_timer <= 0.0 {
            enemy.teleport_timer = rng.random_range(3.0..4.5);
            let mut tiles = Vec::new();
            for y in room.y + 1..room.y + room.h - 1 {
                for x in room.x + 1..room.x + room.w - 1 {
                    let p = IVec2::new(x, y);
                    if d.get(p) == Tile::Floor && tile_center(p).distance(ppos) > 48.0 {
                        tiles.push(p);
                    }
                }
            }
            if let Some(p) = tiles.choose(&mut rng) {
                spawn_particles(&mut commands, pos, palette::PURPLE, 14, 60.0, 0.5);
                let target = tile_center(*p);
                transform.translation.x = target.x;
                transform.translation.y = target.y;
                spawn_particles(&mut commands, target, palette::PURPLE, 14, 60.0, 0.5);
                sfx.write(PlaySfx(SfxKind::Teleport));
            }
        }

        if enemy.summon_timer <= 0.0 {
            enemy.summon_timer = 11.0;
            let alive = minions
                .iter()
                .filter(|e| e.kind == MonsterKind::Skeleton)
                .count();
            if alive < 4 {
                for _ in 0..2 {
                    let offset =
                        Vec2::from_angle(rng.random_range(0.0..std::f32::consts::TAU)) * 28.0;
                    let p = pos + offset;
                    if !d.solid(tile_of(p)) {
                        spawn_enemy(
                            &mut commands,
                            &atlas,
                            MonsterKind::Skeleton,
                            MAX_FLOOR,
                            p,
                            Some(room),
                        );
                        spawn_particles(&mut commands, p, palette::BONE, 8, 40.0, 0.5);
                    }
                }
                notify.write(Notify("The Lich raises the dead!".into()));
                sfx.write(PlaySfx(SfxKind::Roar));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn boss_room_seal(
    mut commands: Commands,
    atlas: Res<Atlas>,
    mut floor: ResMut<CurrentFloor>,
    player: Query<&Transform, With<Player>>,
    bosses: Query<(), With<Boss>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut banner: MessageWriter<Banner>,
    mut shake: ResMut<Shake>,
) {
    if floor.dungeon.floor != MAX_FLOOR || floor.boss_sealed || bosses.is_empty() {
        return;
    }
    let Ok(t) = player.single() else { return };
    let tile = tile_of(t.translation.truncate());
    let Some(room) = floor.dungeon.room_of_kind(RoomKind::Boss) else {
        return;
    };
    let inside = tile.x > room.x
        && tile.x < room.x + room.w - 1
        && tile.y > room.y
        && tile.y < room.y + room.h - 1;
    if !inside {
        return;
    }
    floor.boss_sealed = true;
    for p in floor.dungeon.boss_doors.clone() {
        floor.dungeon.set(p, Tile::Wall);
        commands.spawn((
            FloorEntity,
            BossDoorSprite,
            atlas.sprite(SpriteId::WallFace, 0),
            Transform::from_translation(tile_center(p).extend(layer::DECOR)),
        ));
        spawn_particles(&mut commands, tile_center(p), palette::GREY, 8, 40.0, 0.5);
    }
    sfx.write(PlaySfx(SfxKind::Roar));
    shake.add(0.6);
    banner.write(Banner("THE LICH".into()));
    notify.write(Notify("The doors slam shut behind you!".into()));
}
