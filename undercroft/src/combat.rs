//! Hits, damage, death, drops, projectiles, traps and visual juice.

use crate::{
    art::{Atlas, SpriteId, TILE},
    audio::{PlaySfx, SfxKind},
    dungeon::{MAX_FLOOR, Tile},
    enemy::{Boss, Enemy},
    game::*,
    items::{MagicItem, Perk},
    physics::{move_box, tile_center, tile_of},
    player::{PickupKind, Player, spawn_pickup},
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Hero,
    Monster,
}

#[derive(Component)]
pub struct Projectile {
    pub damage: i32,
    pub owner: Faction,
    pub pierce: bool,
    pub hit: Vec<Entity>,
    pub element: Option<Element>,
}

/// Damage over time from fire. Spreads to other monsters on contact.
#[derive(Component)]
pub struct Burning {
    pub left: f32,
    pub tick: f32,
    /// Only fires lit directly (arrows, fireballs) jump to neighbours, so two
    /// monsters cannot keep re-igniting each other forever.
    pub spreads: bool,
}

/// Damage over time from poison.
#[derive(Component)]
pub struct Poisoned {
    pub left: f32,
    pub tick: f32,
}

/// Cannot move or act until it thaws.
#[derive(Component)]
pub struct Frozen {
    pub left: f32,
}

const BURN_TIME: f32 = 3.0;
const BURN_TICK: f32 = 0.5;
const POISON_TIME: f32 = 5.0;
const POISON_TICK: f32 = 1.0;
const FREEZE_TIME: f32 = 2.0;

/// Puts a status effect on `entity`. Re-applying refreshes the duration.
pub fn apply_element(commands: &mut Commands, entity: Entity, element: Element) {
    let mut e = commands.entity(entity);
    match element {
        Element::Fire => {
            e.insert(Burning {
                left: BURN_TIME,
                tick: BURN_TICK,
                spreads: true,
            });
        }
        Element::Poison => {
            e.insert(Poisoned {
                left: POISON_TIME,
                tick: POISON_TICK,
            });
        }
        Element::Frost => {
            e.insert(Frozen { left: FREEZE_TIME });
        }
    }
}

#[derive(Component)]
pub struct SwordSwing {
    pub damage: i32,
    pub radius: f32,
    pub hit: Vec<Entity>,
}

#[derive(Component)]
pub struct Spike {
    pub tile: IVec2,
    pub timer: f32,
    pub up: bool,
}

#[derive(Component)]
pub struct CrackedWallSprite(pub IVec2);

#[derive(Component)]
pub struct BossDoorSprite;

#[derive(Component)]
pub struct DamageNumber;

/// The sprite's resting colour; flashes and blinks are layered on top.
#[derive(Component)]
pub struct BaseColor(pub Color);

#[derive(Message)]
pub struct DamageEnemy {
    pub target: Entity,
    pub amount: i32,
    pub from: Vec2,
    pub knockback: f32,
    pub arrow: bool,
    pub element: Option<Element>,
}

#[derive(Message)]
pub struct DamagePlayer {
    pub amount: i32,
    pub from: Vec2,
    /// Damage over time: ignores and does not grant invulnerability, no knockback.
    pub dot: bool,
    /// A raised shield facing the attacker stops it. Spikes come from below.
    pub blockable: bool,
    pub element: Option<Element>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEnemy>()
            .add_message::<DamagePlayer>()
            .add_systems(
                Update,
                (
                    sword_hits,
                    projectiles,
                    contact_damage,
                    spikes,
                    status_effects,
                )
                    .in_set(Step::Hits),
            )
            .add_systems(
                Update,
                (apply_enemy_damage, apply_player_damage).in_set(Step::Damage),
            )
            .add_systems(
                Update,
                (
                    tick_hit_flash,
                    tick_invulnerable,
                    apply_knockback,
                    tick_lifetime,
                    move_particles,
                    float_damage_numbers,
                    apply_sprite_color,
                )
                    .in_set(Step::Juice),
            );
    }
}

// ---------------------------------------------------------------------------
// Spawning helpers
// ---------------------------------------------------------------------------

pub fn spawn_particles(
    commands: &mut Commands,
    pos: Vec2,
    color: Color,
    count: usize,
    speed: f32,
    life: f32,
) {
    let mut rng = rand::rng();
    for _ in 0..count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let v = Vec2::from_angle(angle) * speed * rng.random_range(0.3..1.0);
        let size = if rng.random_bool(0.3) { 2.0 } else { 1.0 };
        commands.spawn((
            FloorEntity,
            Particle {
                vel: v,
                drag: 0.92,
                gravity: 0.0,
            },
            Lifetime(life * rng.random_range(0.6..1.2)),
            Sprite::from_color(color, Vec2::splat(size)),
            Transform::from_translation(pos.extend(layer::PARTICLE)),
        ));
    }
}

pub fn spawn_damage_number(
    commands: &mut Commands,
    atlas: &Atlas,
    pos: Vec2,
    amount: i32,
    color: Color,
) {
    let digits: Vec<u32> = amount
        .max(0)
        .to_string()
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();
    let width = digits.len() as f32 * 4.0;
    let mut rng = rand::rng();
    let offset = Vec2::new(rng.random_range(-3.0..3.0), 8.0);
    commands
        .spawn((
            FloorEntity,
            DamageNumber,
            Lifetime(0.7),
            Transform::from_translation((pos + offset).extend(layer::OVERLAY)),
            Visibility::default(),
        ))
        .with_children(|p| {
            for (i, d) in digits.iter().enumerate() {
                let mut sprite = atlas.sprite(SpriteId::digit(*d), 0);
                sprite.color = color;
                p.spawn((
                    sprite,
                    Transform::from_xyz(i as f32 * 4.0 - width / 2.0 + 2.0, 0.0, 0.0),
                ));
            }
        });
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_projectile(
    commands: &mut Commands,
    atlas: &Atlas,
    id: SpriteId,
    pos: Vec2,
    dir: Vec2,
    speed: f32,
    damage: i32,
    owner: Faction,
    pierce: bool,
    element: Option<Element>,
) {
    let mut sprite = atlas.sprite(id, 0);
    let mut entity = commands.spawn((
        FloorEntity,
        Projectile {
            damage,
            owner,
            pierce,
            hit: Vec::new(),
            element,
        },
        Velocity(dir * speed),
        Lifetime(3.0),
        Transform::from_translation(pos.extend(layer::PROJECTILE))
            .with_rotation(Quat::from_rotation_z(dir.to_angle())),
    ));
    if id == SpriteId::Bolt {
        entity.insert((
            Animation::new(SpriteId::Bolt, 0.1),
            Light {
                radius: 28.0,
                intensity: 0.8,
                color: crate::palette::PURPLE,
                flicker: 0.2,
            },
        ));
    } else if id == SpriteId::Fireball {
        entity.insert((
            Animation::new(SpriteId::Fireball, 0.08),
            Light {
                radius: 26.0,
                intensity: 0.8,
                color: crate::palette::ORANGE,
                flicker: 0.3,
            },
        ));
    } else if id == SpriteId::Arrow && owner == Faction::Monster {
        sprite.color = crate::palette::BONE_SHADOW;
    } else if id == SpriteId::Arrow
        && let Some(element) = element
    {
        // Enchanted arrows glow their colour.
        let glow = Color::WHITE.mix(&element.color(), 0.6).to_linear();
        sprite.color = Color::from(glow);
        entity.insert(Light {
            radius: 20.0,
            intensity: 0.7,
            color: element.color(),
            flicker: 0.3,
        });
    }
    entity.insert(sprite);
}

// ---------------------------------------------------------------------------
// Hit detection
// ---------------------------------------------------------------------------

fn sword_hits(
    mut commands: Commands,
    atlas: Res<Atlas>,
    mut floor: ResMut<CurrentFloor>,
    mut swings: Query<(&GlobalTransform, &mut SwordSwing)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    cracked: Query<(Entity, &CrackedWallSprite)>,
    mut damage: MessageWriter<DamageEnemy>,
    mut sfx: MessageWriter<PlaySfx>,
    mut shake: ResMut<Shake>,
    mut notify: MessageWriter<Notify>,
) {
    for (gt, mut swing) in &mut swings {
        let center = gt.translation().truncate();
        for (entity, transform, enemy) in &enemies {
            if swing.hit.contains(&entity) {
                continue;
            }
            if transform.translation.truncate().distance(center) < swing.radius + enemy.radius {
                swing.hit.push(entity);
                damage.write(DamageEnemy {
                    target: entity,
                    amount: swing.damage,
                    from: center,
                    knockback: 150.0,
                    arrow: false,
                    element: None,
                });
            }
        }
        let t = tile_of(center);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = t + IVec2::new(dx, dy);
                if floor.dungeon.get(p) == Tile::Cracked
                    && tile_center(p).distance(center) < swing.radius + 9.0
                {
                    floor.dungeon.set(p, Tile::Rubble);
                    for (e, c) in &cracked {
                        if c.0 == p {
                            commands.entity(e).despawn();
                        }
                    }
                    commands.spawn((
                        FloorEntity,
                        atlas.sprite(SpriteId::Rubble, 0),
                        Transform::from_translation(tile_center(p).extend(layer::FLOOR + 0.1)),
                    ));
                    spawn_particles(
                        &mut commands,
                        tile_center(p),
                        crate::palette::GREY,
                        16,
                        70.0,
                        0.5,
                    );
                    sfx.write(PlaySfx(SfxKind::Break));
                    shake.add(0.35);
                    notify.write(Notify("The wall crumbles, revealing a secret room!".into()));
                }
            }
        }
    }
}

fn projectiles(
    mut commands: Commands,
    time: Res<Time>,
    floor: Res<CurrentFloor>,
    mut projectiles: Query<(Entity, &mut Transform, &Velocity, &mut Projectile)>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<Projectile>>,
    player: Query<&Transform, (With<Player>, Without<Projectile>)>,
    mut damage_enemy: MessageWriter<DamageEnemy>,
    mut damage_player: MessageWriter<DamagePlayer>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, vel, mut projectile) in &mut projectiles {
        transform.translation += (vel.0 * dt).extend(0.0);
        let pos = transform.translation.truncate();
        if floor.dungeon.solid(tile_of(pos)) || !floor.dungeon.in_bounds(tile_of(pos)) {
            spawn_particles(&mut commands, pos, crate::palette::LIGHT_GREY, 4, 40.0, 0.3);
            sfx.write(PlaySfx(SfxKind::ArrowHit));
            commands.entity(entity).despawn();
            continue;
        }
        match projectile.owner {
            Faction::Hero => {
                for (e, t, enemy) in &enemies {
                    if projectile.hit.contains(&e) {
                        continue;
                    }
                    if t.translation.truncate().distance(pos) < enemy.radius + 5.0 {
                        projectile.hit.push(e);
                        damage_enemy.write(DamageEnemy {
                            target: e,
                            amount: projectile.damage,
                            from: pos - vel.0.normalize_or_zero() * 8.0,
                            knockback: 90.0,
                            arrow: true,
                            element: projectile.element,
                        });
                        if !projectile.pierce {
                            commands.entity(entity).despawn();
                            break;
                        }
                    }
                }
            }
            Faction::Monster => {
                if let Ok(t) = player.single()
                    && t.translation.truncate().distance(pos) < 7.0
                {
                    damage_player.write(DamagePlayer {
                        amount: projectile.damage,
                        from: pos - vel.0.normalize_or_zero() * 8.0,
                        dot: false,
                        blockable: true,
                        element: projectile.element,
                    });
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn contact_damage(
    hero: Res<Hero>,
    player: Query<(&Transform, &Player)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut damage_player: MessageWriter<DamagePlayer>,
    mut damage_enemy: MessageWriter<DamageEnemy>,
) {
    let Ok((pt, player)) = player.single() else {
        return;
    };
    let ppos = pt.translation.truncate();
    for (entity, t, enemy) in &enemies {
        if enemy.contact_damage == 0 || !enemy.vulnerable {
            continue;
        }
        let epos = t.translation.truncate();
        if epos.distance(ppos) < enemy.radius + 5.0 {
            if player.sprinting && hero.has_perk(Perk::SprintStrike) {
                damage_enemy.write(DamageEnemy {
                    target: entity,
                    amount: 2 + hero.sword_damage() / 2,
                    from: ppos,
                    knockback: 220.0,
                    arrow: false,
                    element: None,
                });
            } else {
                damage_player.write(DamagePlayer {
                    amount: enemy.contact_damage,
                    from: epos,
                    dot: false,
                    blockable: true,
                    element: None,
                });
            }
        }
    }
}

fn spikes(
    time: Res<Time>,
    mut spikes: Query<(&mut Spike, &mut Animation)>,
    player: Query<&Transform, With<Player>>,
    mut damage_player: MessageWriter<DamagePlayer>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let ppos = player.single().map(|t| t.translation.truncate()).ok();
    for (mut spike, mut anim) in &mut spikes {
        spike.timer += time.delta_secs();
        let phase = spike.timer % 3.0;
        let up = phase < 1.1;
        let center = tile_center(spike.tile);
        if up
            && !spike.up
            && let Some(p) = ppos
            && p.distance(center) < 140.0
        {
            sfx.write(PlaySfx(SfxKind::Spike));
        }
        spike.up = up;
        anim.frame = if up { 1 } else { 0 };
        if up
            && let Some(p) = ppos
            && (p - center).abs().max_element() < 7.0
        {
            damage_player.write(DamagePlayer {
                amount: 3,
                from: center + Vec2::new(0.0, -4.0),
                dot: false,
                blockable: false,
                element: None,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Status effects
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn status_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut burning: Query<(
        Entity,
        &Transform,
        &mut Burning,
        Option<&Enemy>,
        Option<&Player>,
    )>,
    mut poisoned: Query<(Entity, &Transform, &mut Poisoned, Option<&Enemy>), Without<Burning>>,
    mut poisoned_burning: Query<(Entity, &Transform, &mut Poisoned, Option<&Enemy>), With<Burning>>,
    mut frozen: Query<(Entity, &Transform, &mut Frozen)>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<Burning>>,
    mut damage_enemy: MessageWriter<DamageEnemy>,
    mut damage_player: MessageWriter<DamagePlayer>,
) {
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (entity, transform, mut burn, enemy, player) in &mut burning {
        let pos = transform.translation.truncate();
        burn.left -= dt;
        burn.tick -= dt;
        if rng.random_bool((dt * 25.0).min(1.0) as f64) {
            let p = pos + Vec2::new(rng.random_range(-4.0..4.0), rng.random_range(-2.0..6.0));
            let color = if rng.random_bool(0.5) {
                crate::palette::ORANGE
            } else {
                crate::palette::YELLOW
            };
            commands.spawn((
                FloorEntity,
                Particle {
                    vel: Vec2::new(rng.random_range(-8.0..8.0), rng.random_range(20.0..45.0)),
                    drag: 0.97,
                    gravity: 0.0,
                },
                Lifetime(0.35),
                Sprite::from_color(
                    color,
                    Vec2::splat(if rng.random_bool(0.3) { 2.0 } else { 1.0 }),
                ),
                Transform::from_translation(p.extend(layer::PARTICLE)),
            ));
        }
        if burn.tick <= 0.0 {
            burn.tick = BURN_TICK;
            if enemy.is_some() {
                damage_enemy.write(DamageEnemy {
                    target: entity,
                    amount: 1,
                    from: pos,
                    knockback: 0.0,
                    arrow: false,
                    element: None,
                });
                // Fire spreads to monsters standing close by.
                if burn.spreads {
                    for (other, ot, oe) in &enemies {
                        if other != entity
                            && ot.translation.truncate().distance(pos) < oe.radius + 10.0
                        {
                            commands.entity(other).insert(Burning {
                                left: BURN_TIME,
                                tick: BURN_TICK,
                                spreads: false,
                            });
                        }
                    }
                }
            } else if player.is_some() {
                damage_player.write(DamagePlayer {
                    amount: 1,
                    from: pos,
                    dot: true,
                    blockable: false,
                    element: None,
                });
            }
        }
        if burn.left <= 0.0 {
            commands.entity(entity).remove::<Burning>();
        }
    }

    let mut poison_tick =
        |entity: Entity, transform: &Transform, poison: &mut Poisoned, enemy: Option<&Enemy>| {
            let pos = transform.translation.truncate();
            poison.left -= dt;
            poison.tick -= dt;
            if rng.random_bool((dt * 8.0).min(1.0) as f64) {
                let p = pos + Vec2::new(rng.random_range(-5.0..5.0), rng.random_range(-4.0..6.0));
                commands.spawn((
                    FloorEntity,
                    Particle {
                        vel: Vec2::new(0.0, rng.random_range(8.0..18.0)),
                        drag: 0.98,
                        gravity: 0.0,
                    },
                    Lifetime(0.6),
                    Sprite::from_color(crate::palette::LIME, Vec2::splat(1.0)),
                    Transform::from_translation(p.extend(layer::PARTICLE)),
                ));
            }
            if poison.tick <= 0.0 {
                poison.tick = POISON_TICK;
                if enemy.is_some() {
                    damage_enemy.write(DamageEnemy {
                        target: entity,
                        amount: 1,
                        from: pos,
                        knockback: 0.0,
                        arrow: false,
                        element: None,
                    });
                } else {
                    damage_player.write(DamagePlayer {
                        amount: 1,
                        from: pos,
                        dot: true,
                        blockable: false,
                        element: None,
                    });
                }
            }
            if poison.left <= 0.0 {
                commands.entity(entity).remove::<Poisoned>();
            }
        };
    for (entity, transform, mut poison, enemy) in &mut poisoned {
        poison_tick(entity, transform, &mut poison, enemy);
    }
    for (entity, transform, mut poison, enemy) in &mut poisoned_burning {
        poison_tick(entity, transform, &mut poison, enemy);
    }

    for (entity, transform, mut ice) in &mut frozen {
        ice.left -= dt;
        if rng.random_bool((dt * 6.0).min(1.0) as f64) {
            let pos = transform.translation.truncate()
                + Vec2::new(rng.random_range(-6.0..6.0), rng.random_range(-6.0..6.0));
            commands.spawn((
                FloorEntity,
                Particle {
                    vel: Vec2::new(0.0, rng.random_range(-6.0..6.0)),
                    drag: 0.98,
                    gravity: 0.0,
                },
                Lifetime(0.5),
                Sprite::from_color(crate::palette::CYAN, Vec2::splat(1.0)),
                Transform::from_translation(pos.extend(layer::PARTICLE)),
            ));
        }
        if ice.left <= 0.0 {
            commands.entity(entity).remove::<Frozen>();
            spawn_particles(
                &mut commands,
                transform.translation.truncate(),
                crate::palette::CYAN,
                8,
                40.0,
                0.4,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Applying damage
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn apply_enemy_damage(
    mut commands: Commands,
    atlas: Res<Atlas>,
    mut reader: MessageReader<DamageEnemy>,
    mut enemies: Query<(&Transform, &mut Health, &mut Enemy, Option<&Boss>)>,
    mut player: Query<&mut Health, (With<Player>, Without<Enemy>)>,
    mut hero: ResMut<Hero>,
    mut floor: ResMut<CurrentFloor>,
    doors: Query<Entity, With<BossDoorSprite>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut shake: ResMut<Shake>,
    mut notify: MessageWriter<Notify>,
    mut banner: MessageWriter<Banner>,
) {
    let mut rng = rand::rng();
    for msg in reader.read() {
        let Ok((transform, mut health, mut enemy, boss)) = enemies.get_mut(msg.target) else {
            continue;
        };
        if health.hp <= 0 {
            continue;
        }
        let pos = transform.translation.truncate();
        if !enemy.vulnerable {
            spawn_particles(&mut commands, pos, crate::palette::LIGHT_GREY, 3, 30.0, 0.3);
            continue;
        }
        let dot = msg.knockback == 0.0;
        let amount = (msg.amount - if msg.arrow { enemy.armor } else { 0 }).max(1);
        health.hp -= amount;
        enemy.aggro = true;
        let dir = (pos - msg.from).normalize_or_zero();
        let mut e = commands.entity(msg.target);
        if !dot {
            e.insert(HitFlash(0.1));
            if boss.is_none() {
                e.insert(Knockback(dir * msg.knockback));
            }
            spawn_particles(&mut commands, pos, enemy.blood, 5, 60.0, 0.35);
            sfx.write(PlaySfx(SfxKind::Hit));
            shake.add(0.12);
        }
        let number_color = match msg.element {
            Some(el) => el.color(),
            None if dot => crate::palette::LIGHT_GREY,
            None => crate::palette::WHITE,
        };
        spawn_damage_number(&mut commands, &atlas, pos, amount, number_color);
        if let Some(element) = msg.element
            && health.hp > 0
            && boss.is_none_or(|_| element != Element::Frost)
        {
            // Bosses shrug off freezing but still burn and sicken.
            apply_element(&mut commands, msg.target, element);
            if element == Element::Frost {
                sfx.write(PlaySfx(SfxKind::Spike));
            }
        }

        if health.hp > 0 {
            continue;
        }

        // Death.
        commands.entity(msg.target).despawn();
        spawn_particles(&mut commands, pos, enemy.blood, 16, 90.0, 0.6);
        spawn_particles(&mut commands, pos, crate::palette::WHITE, 6, 50.0, 0.4);
        sfx.write(PlaySfx(SfxKind::EnemyDie));
        shake.add(if boss.is_some() { 0.8 } else { 0.2 });
        hero.kills += 1;
        let levels = hero.gain_xp(enemy.xp);
        if levels > 0 {
            sfx.write(PlaySfx(SfxKind::LevelUp));
            banner.write(Banner(format!("LEVEL {}", hero.level)));
            notify.write(Notify(format!(
                "Level {}! Max health and energy increased.",
                hero.level
            )));
            if let Ok(mut h) = player.single_mut() {
                h.hp = (h.hp + 5).min(hero.max_hp());
            }
        }
        if hero.has_perk(Perk::Vampiric)
            && let Ok(mut h) = player.single_mut()
        {
            h.hp = (h.hp + 1).min(h.max);
        }

        let coins = rng.random_range(enemy.coins.0..=enemy.coins.1);
        for i in 0..coins {
            let value = if boss.is_some() { 5 } else { 1 };
            let angle =
                i as f32 / coins.max(1) as f32 * std::f32::consts::TAU + rng.random_range(0.0..1.0);
            let vel = Vec2::from_angle(angle) * rng.random_range(40.0..90.0);
            spawn_pickup(&mut commands, &atlas, pos, PickupKind::Coin(value), vel);
        }
        if rng.random_bool(0.07) {
            spawn_pickup(
                &mut commands,
                &atlas,
                pos,
                PickupKind::Potion,
                Vec2::new(0.0, 30.0),
            );
        }
        if hero.bow.is_some() && rng.random_bool(0.15) {
            spawn_pickup(
                &mut commands,
                &atlas,
                pos,
                PickupKind::Arrows(3),
                Vec2::new(20.0, 20.0),
            );
        }

        if enemy.kind == crate::dungeon::MonsterKind::Lich {
            sfx.write(PlaySfx(SfxKind::Victory));
            banner.write(Banner("THE LICH FALLS".into()));
            notify.write(Notify(
                "The seal is broken. Take the stairs to escape!".into(),
            ));
            for p in floor.dungeon.boss_doors.clone() {
                floor.dungeon.set(p, Tile::Floor);
            }
            floor.boss_sealed = false;
            for d in &doors {
                commands.entity(d).despawn();
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_player_damage(
    mut commands: Commands,
    atlas: Res<Atlas>,
    mut reader: MessageReader<DamagePlayer>,
    mut player: Query<(
        Entity,
        &Transform,
        &Facing,
        &mut Player,
        &mut Energy,
        &mut Health,
        Option<&Invulnerable>,
    )>,
    mut hero: ResMut<Hero>,
    mut sfx: MessageWriter<PlaySfx>,
    mut shake: ResMut<Shake>,
    mut notify: MessageWriter<Notify>,
    mut banner: MessageWriter<Banner>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok((entity, transform, facing, mut player, mut energy, mut health, invulnerable)) =
        player.single_mut()
    else {
        reader.clear();
        return;
    };
    if health.hp <= 0 {
        reader.clear();
        return;
    }
    let pos = transform.translation.truncate();
    let messages: Vec<&DamagePlayer> = reader.read().collect();
    // Damage over time always ticks; of the direct hits, only the strongest lands.
    let mut total = 0;
    for msg in messages.iter().filter(|m| m.dot) {
        total += msg.amount;
        spawn_damage_number(
            &mut commands,
            &atlas,
            pos,
            msg.amount,
            crate::palette::ORANGE,
        );
    }
    // A charging Knight cannot be hurt.
    let charging = player.bash_left > 0.0;
    if let Some(msg) = messages.iter().filter(|m| !m.dot).max_by_key(|m| m.amount)
        && invulnerable.is_none()
        && !charging
    {
        let to_attacker = (msg.from - pos).normalize_or_zero();
        let guarded = msg.blockable && player.blocking && to_attacker.dot(facing.0) > 0.3;
        if guarded && player.guard_timer > 0.0 {
            // Still braced from the last block.
            return;
        }
        if guarded && energy.cur >= hero.block_cost() {
            energy.cur -= hero.block_cost();
            energy.since_use = 0.0;
            player.guard_timer = 0.35;
            let shield = pos + facing.0 * 8.0;
            spawn_particles(&mut commands, shield, crate::palette::WHITE, 5, 70.0, 0.3);
            spawn_particles(&mut commands, shield, crate::palette::YELLOW, 3, 50.0, 0.25);
            commands
                .entity(entity)
                .insert(Knockback(-to_attacker * 70.0));
            sfx.write(PlaySfx(SfxKind::Block));
            shake.add(0.15);
            return;
        }
        if guarded {
            notify.write(Notify("Too tired to hold the shield up!".into()));
        }
        let amount = (msg.amount - hero.armor_reduction()).max(1);
        total += amount;
        let dir = (pos - msg.from).normalize_or_zero();
        commands.entity(entity).insert((
            Invulnerable(hero.invuln_time()),
            HitFlash(0.1),
            Knockback(dir * 170.0),
        ));
        spawn_damage_number(&mut commands, &atlas, pos, amount, crate::palette::RED);
        spawn_particles(&mut commands, pos, crate::palette::RED, 6, 60.0, 0.4);
        sfx.write(PlaySfx(SfxKind::Hurt));
        shake.add(0.45);
        if let Some(element) = msg.element {
            apply_element(&mut commands, entity, element);
            if element == Element::Fire {
                notify.write(Notify("You're on fire!".into()));
            }
        }
    }
    if total == 0 {
        return;
    }
    health.hp -= total;

    if health.hp <= 0 {
        if hero.has_magic(MagicItem::PhoenixFeather) && !hero.phoenix_used {
            hero.phoenix_used = true;
            health.hp = health.max / 2;
            spawn_particles(&mut commands, pos, crate::palette::ORANGE, 30, 120.0, 0.8);
            sfx.write(PlaySfx(SfxKind::LevelUp));
            banner.write(Banner("REBORN".into()));
            notify.write(Notify(
                "The Phoenix Feather burns away and you rise again!".into(),
            ));
            commands.entity(entity).insert(Invulnerable(2.0));
        } else {
            sfx.write(PlaySfx(SfxKind::Death));
            next.set(GameState::GameOver);
        }
    }
}

// ---------------------------------------------------------------------------
// Juice
// ---------------------------------------------------------------------------

fn tick_hit_flash(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut HitFlash)>) {
    for (e, mut flash) in &mut q {
        flash.0 -= time.delta_secs();
        if flash.0 <= 0.0 {
            commands.entity(e).remove::<HitFlash>();
        }
    }
}

fn tick_invulnerable(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Invulnerable)>,
) {
    for (e, mut inv) in &mut q {
        inv.0 -= time.delta_secs();
        if inv.0 <= 0.0 {
            commands.entity(e).remove::<Invulnerable>();
        }
    }
}

fn apply_knockback(
    mut commands: Commands,
    time: Res<Time>,
    floor: Res<CurrentFloor>,
    mut q: Query<(Entity, &mut Transform, &mut Knockback, &Hitbox)>,
) {
    let dt = time.delta_secs();
    for (e, mut transform, mut kb, hitbox) in &mut q {
        let pos = transform.translation.truncate();
        let (new_pos, _) = move_box(&floor.dungeon, pos, kb.0 * dt, hitbox.0);
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
        kb.0 *= (1.0 - 10.0 * dt).max(0.0);
        if kb.0.length() < 8.0 {
            commands.entity(e).remove::<Knockback>();
        }
    }
}

fn tick_lifetime(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Lifetime)>) {
    for (e, mut life) in &mut q {
        life.0 -= time.delta_secs();
        if life.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn move_particles(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut Particle, &Lifetime, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut particle, life, mut sprite) in &mut q {
        transform.translation += (particle.vel * dt).extend(0.0);
        let drag = particle.drag.powf(dt * 60.0);
        particle.vel *= drag;
        let g = particle.gravity;
        particle.vel.y -= g * dt;
        let alpha = (life.0 * 3.0).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(alpha);
    }
}

fn float_damage_numbers(
    time: Res<Time>,
    mut numbers: Query<(&mut Transform, &Lifetime, &Children), With<DamageNumber>>,
    mut sprites: Query<&mut Sprite>,
) {
    for (mut transform, life, children) in &mut numbers {
        transform.translation.y += 14.0 * time.delta_secs();
        let alpha = (life.0 * 2.5).clamp(0.0, 1.0);
        for child in children.iter() {
            if let Ok(mut sprite) = sprites.get_mut(child) {
                sprite.color = sprite.color.with_alpha(alpha);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_sprite_color(
    mut q: Query<(
        &mut Sprite,
        &BaseColor,
        Option<&HitFlash>,
        Option<&Invulnerable>,
        Option<&Burning>,
        Option<&Poisoned>,
        Option<&Frozen>,
    )>,
) {
    for (mut sprite, base, flash, invulnerable, burning, poisoned, frozen) in &mut q {
        let mut color = base.0;
        if frozen.is_some() {
            color = color.mix(&crate::palette::CYAN, 0.55);
        } else if burning.is_some() {
            color = color.mix(&crate::palette::DARK_ORANGE, 0.45);
        } else if poisoned.is_some() {
            color = color.mix(&crate::palette::LIME, 0.4);
        }
        if let Some(inv) = invulnerable
            && ((inv.0 * 18.0) as i32) % 2 == 0
        {
            color = color.with_alpha(0.35);
        }
        if flash.is_some() {
            // Sprite tints must stay within 0..1: values above that blank the
            // whole frame on Metal. A hot pink-white read as a hit.
            color = Color::srgba(1.0, 0.55, 0.55, color.alpha());
        }
        sprite.color = color;
    }
}

/// True when the boss of the final floor is still alive.
pub fn boss_alive(floor: &CurrentFloor, bosses: &Query<(), With<Boss>>) -> bool {
    floor.dungeon.floor == MAX_FLOOR && !bosses.is_empty()
}

/// Half a tile: how close to a tile centre counts as standing on it.
pub const ON_TILE: f32 = TILE * 0.45;
