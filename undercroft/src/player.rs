//! The hero: movement, sprinting, blocking, sword, bow, the class's special
//! ability, potions, interaction, pickups.

use crate::{
    art::{Atlas, SpriteId},
    audio::{PlaySfx, SfxKind},
    classes::{Class, Dir},
    combat::{
        BaseColor, DamageEnemy, Faction, ON_TILE, SwordSwing, boss_alive, spawn_particles,
        spawn_projectile,
    },
    dungeon::{ChestLoot, MAX_FLOOR},
    enemy::{Boss, Enemy},
    game::*,
    input::Controls,
    items::{BOWS, MagicItem, Perk, ShopItem},
    physics::{move_box, tile_center, tile_of},
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct Player {
    pub sword_cd: f32,
    pub bow_cd: f32,
    pub sprinting: bool,
    /// Shield raised (L held). Hits from the front are absorbed for energy.
    pub blocking: bool,
    /// After a block, further front hits are ignored for a moment so a
    /// monster leaning on the shield does not drain all the energy at once.
    pub guard_timer: f32,
    /// Seconds until the class's special ability is ready again.
    pub special_cd: f32,
    /// Knight: seconds left in a shield bash charge.
    pub bash_left: f32,
    pub bash_hit: Vec<Entity>,
    /// Rogue: the next sword hit does triple damage.
    pub ambush: bool,
    pub hint_timer: f32,
    pub dust_timer: f32,
    pub regen_acc: f32,
}

/// The shield sprite parented to the hero.
#[derive(Component)]
pub struct HeroShield;

/// How fast a shield bash carries the hero, and for how long.
const BASH_SPEED: f32 = 280.0;
const BASH_TIME: f32 = 0.18;

#[derive(Clone, Copy, Debug)]
pub enum PickupKind {
    Coin(u32),
    Potion,
    Arrows(u32),
    Key,
}

#[derive(Component)]
pub struct Pickup {
    pub kind: PickupKind,
    pub vel: Vec2,
    pub phase: f32,
    pub age: f32,
}

#[derive(Component)]
pub struct Chest {
    pub loot: ChestLoot,
    pub open: bool,
}

#[derive(Component)]
pub struct Shopkeeper {
    pub stock: Vec<ShopItem>,
}

/// Which shopkeeper the open shop menu belongs to.
#[derive(Resource)]
pub struct ShopContext {
    pub shopkeeper: Entity,
    pub selected: usize,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                player_move,
                update_shield,
                player_actions,
                special_ability,
                player_stairs,
                regen,
                sync_hero_maxes,
            )
                .chain()
                .in_set(Step::Move),
        )
        .add_systems(Update, (pickups, bash_hits).in_set(Step::Hits));
    }
}

pub fn spawn_player(commands: &mut Commands, atlas: &Atlas, hero: &Hero, pos: Vec2) {
    let mut shield = atlas.sprite(SpriteId::Shield, 0);
    shield.color = hero.class.color();
    let sprite = hero.class.sprite(Dir::Down);
    let mut player = commands.spawn((
        Player {
            sword_cd: 0.0,
            bow_cd: 0.0,
            sprinting: false,
            blocking: false,
            guard_timer: 0.0,
            special_cd: 0.0,
            bash_left: 0.0,
            bash_hit: Vec::new(),
            ambush: false,
            hint_timer: 0.0,
            dust_timer: 0.0,
            regen_acc: 0.0,
        },
        RunEntity,
        Health {
            hp: hero.max_hp(),
            max: hero.max_hp(),
        },
        Energy {
            cur: hero.max_energy(),
            max: hero.max_energy(),
            since_use: 10.0,
        },
        Velocity::default(),
        Facing(Vec2::NEG_Y),
        Hitbox(Vec2::new(5.0, 5.0)),
        Animation::new(sprite, 0.14),
        YSort,
        BaseColor(Color::WHITE),
        Light {
            radius: hero.light_radius(),
            intensity: 1.0,
            color: Color::srgb(1.0, 0.93, 0.8),
            flicker: 0.03,
        },
        atlas.sprite(sprite, 0),
        Transform::from_translation(pos.extend(layer::ACTOR)),
    ));
    player.with_children(|p| {
        p.spawn((HeroShield, shield, Transform::from_xyz(5.0, -2.0, 0.1)));
    });
}

pub fn spawn_pickup(
    commands: &mut Commands,
    atlas: &Atlas,
    pos: Vec2,
    kind: PickupKind,
    vel: Vec2,
) {
    let mut rng = rand::rng();
    let (id, frame_time) = match kind {
        PickupKind::Coin(_) => (SpriteId::Coin, 0.22),
        PickupKind::Potion => (SpriteId::Potion, 1.0),
        PickupKind::Arrows(_) => (SpriteId::Arrows, 1.0),
        PickupKind::Key => (SpriteId::Key, 1.0),
    };
    let mut anim = Animation::new(id, frame_time);
    anim.timer = rng.random_range(0.0..frame_time);
    let mut entity = commands.spawn((
        FloorEntity,
        Pickup {
            kind,
            vel,
            phase: rng.random_range(0.0..std::f32::consts::TAU),
            age: 0.0,
        },
        anim,
        atlas.sprite(id, 0),
        Transform::from_translation(pos.extend(layer::ITEM)),
    ));
    if matches!(kind, PickupKind::Key) {
        entity.insert(Light {
            radius: 30.0,
            intensity: 0.7,
            color: crate::palette::YELLOW,
            flicker: 0.1,
        });
    }
}

#[allow(clippy::type_complexity)]
fn player_move(
    mut commands: Commands,
    time: Res<Time>,
    controls: Res<Controls>,
    hero: Res<Hero>,
    floor: Res<CurrentFloor>,
    mut q: Query<(
        &mut Transform,
        &mut Player,
        &mut Facing,
        &mut Animation,
        &mut Sprite,
        &Hitbox,
        &mut Energy,
        Option<&Knockback>,
    )>,
) {
    let dt = time.delta_secs();
    let Ok((
        mut transform,
        mut player,
        mut facing,
        mut anim,
        mut sprite,
        hitbox,
        mut energy,
        knockback,
    )) = q.single_mut()
    else {
        return;
    };
    player.sword_cd -= dt;
    player.bow_cd -= dt;
    player.hint_timer -= dt;
    player.dust_timer -= dt;
    player.guard_timer -= dt;
    player.special_cd = (player.special_cd - dt).max(0.0);

    let pos = transform.translation.truncate();

    // A shield bash carries the hero forward; nothing else moves them.
    if player.bash_left > 0.0 {
        player.bash_left -= dt;
        let (new_pos, blocked) =
            move_box(&floor.dungeon, pos, facing.0 * BASH_SPEED * dt, hitbox.0);
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
        if blocked.any() {
            player.bash_left = 0.0;
        }
        if player.dust_timer <= 0.0 {
            player.dust_timer = 0.03;
            spawn_afterimage(&mut commands, &sprite, new_pos, crate::palette::WHITE);
        }
        player.blocking = true;
        player.sprinting = false;
        anim.playing = true;
        anim.frame_time = 0.07;
        set_hero_pose(&hero, facing.0, &mut anim, &mut sprite);
        return;
    }

    let dir = controls.move_dir;
    let moving = dir != Vec2::ZERO;
    player.blocking = controls.block;
    player.sprinting = controls.sprint && moving && energy.cur > 0.0 && !player.blocking;
    let mut speed = hero.move_speed();
    if player.sprinting {
        speed *= hero.sprint_multiplier();
        energy.cur = (energy.cur - hero.sprint_drain() * dt).max(0.0);
        energy.since_use = 0.0;
        if player.dust_timer <= 0.0 {
            player.dust_timer = 0.05;
            let dust = pos - dir * 5.0 + Vec2::new(0.0, -5.0);
            spawn_particles(&mut commands, dust, crate::palette::GREY, 2, 15.0, 0.35);
            // A fading trail of ghosts sells the speed.
            spawn_afterimage(&mut commands, &sprite, pos, Color::srgb(0.55, 0.8, 1.0));
        }
    } else if player.blocking {
        // Shuffling behind a raised shield.
        speed *= 0.45;
    }

    if moving {
        // Blocking keeps the shield pointed where it was; you can back away
        // from a monster without turning to face it.
        if !player.blocking {
            facing.0 = dir;
        }
        if knockback.is_none() {
            let (new_pos, _) = move_box(&floor.dungeon, pos, dir * speed * dt, hitbox.0);
            transform.translation.x = new_pos.x;
            transform.translation.y = new_pos.y;
        }
        anim.playing = true;
        anim.frame_time = if player.sprinting { 0.07 } else { 0.14 };
    } else {
        anim.playing = false;
        anim.frame = 0;
    }

    set_hero_pose(&hero, facing.0, &mut anim, &mut sprite);
}

/// Picks the class sprite for the direction the hero faces.
fn set_hero_pose(hero: &Hero, facing: Vec2, anim: &mut Animation, sprite: &mut Sprite) {
    let (dir, flip) = if facing.x.abs() > facing.y.abs() {
        (Dir::Side, facing.x < 0.0)
    } else if facing.y > 0.0 {
        (Dir::Up, false)
    } else {
        (Dir::Down, false)
    };
    anim.id = hero.class.sprite(dir);
    sprite.flip_x = flip;
}

/// A translucent copy of the hero's current frame that fades out in place.
fn spawn_afterimage(commands: &mut Commands, sprite: &Sprite, pos: Vec2, tint: Color) {
    let mut ghost = sprite.clone();
    ghost.color = tint.with_alpha(0.6);
    commands.spawn((
        FloorEntity,
        Particle {
            vel: Vec2::ZERO,
            drag: 1.0,
            gravity: 0.0,
        },
        Lifetime(0.22),
        ghost,
        Transform::from_translation(pos.extend(layer::ACTOR - 0.5)),
    ));
}

/// Keeps the shield on the hero's arm: at their side normally, raised in
/// front of them while blocking, and behind them when they face away.
fn update_shield(
    players: Query<(&Player, &Facing, &Children)>,
    mut shields: Query<(&mut Transform, &mut Sprite), With<HeroShield>>,
) {
    let Ok((player, facing, children)) = players.single() else {
        return;
    };
    let f = facing.0;
    let side = f.x.abs() > f.y.abs();
    let raised = player.blocking;
    // (offset, in front of the hero?, mirrored?)
    let (offset, in_front, flip) = match (side, f.y > 0.0, raised) {
        (true, _, false) => (Vec2::new(3.0 * f.x.signum(), -3.0), true, f.x < 0.0),
        (true, _, true) => (Vec2::new(8.0 * f.x.signum(), -1.0), true, f.x < 0.0),
        (false, false, false) => (Vec2::new(5.0, -2.0), true, false),
        (false, false, true) => (Vec2::new(0.0, -7.0), true, false),
        (false, true, false) => (Vec2::new(-5.0, -2.0), false, false),
        (false, true, true) => (Vec2::new(0.0, 5.0), false, false),
    };
    for child in children.iter() {
        if let Ok((mut t, mut sprite)) = shields.get_mut(child) {
            t.translation = offset.extend(if in_front { 0.1 } else { -0.1 });
            sprite.flip_x = flip;
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = SpriteId::Shield.index(raised as usize);
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn player_actions(
    mut commands: Commands,
    controls: Res<Controls>,
    atlas: Res<Atlas>,
    mut hero: ResMut<Hero>,
    mut q: Query<(&Transform, &Facing, &mut Player, &mut Health, &mut Energy)>,
    shopkeepers: Query<(Entity, &Transform), With<Shopkeeper>>,
    mut chests: Query<(&Transform, &mut Chest, &mut Sprite)>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok((transform, facing, mut player, mut health, mut energy)) = q.single_mut() else {
        return;
    };
    let pos = transform.translation.truncate();
    let mut rng = rand::rng();

    if controls.attack && player.sword_cd <= 0.0 && player.bash_left <= 0.0 {
        player.sword_cd = hero.sword_cooldown();
        let mut sprite = atlas.sprite(SpriteId::Slash, 0);
        sprite.color = crate::items::SWORDS[hero.sword].tint;
        let mut damage = hero.sword_damage();
        if player.ambush {
            // The Rogue strikes from the shadows.
            player.ambush = false;
            damage *= 3;
            sprite.color = crate::palette::PURPLE;
            spawn_particles(
                &mut commands,
                pos + facing.0 * 10.0,
                crate::palette::PURPLE,
                8,
                50.0,
                0.4,
            );
        }
        commands.spawn((
            FloorEntity,
            SwordSwing {
                damage,
                radius: 15.0,
                hit: Vec::new(),
            },
            Lifetime(0.13),
            sprite,
            Transform::from_translation((pos + facing.0 * 11.0).extend(layer::PROJECTILE))
                .with_rotation(Quat::from_rotation_z(facing.0.to_angle())),
        ));
        sfx.write(PlaySfx(SfxKind::Swing));
    }

    if controls.bow && player.bow_cd <= 0.0 {
        match hero.bow {
            None => {
                notify.write(Notify("You have no bow. Shops sell them.".into()));
                sfx.write(PlaySfx(SfxKind::Error));
            }
            Some(_) if hero.arrows == 0 => {
                notify.write(Notify("Out of arrows.".into()));
                sfx.write(PlaySfx(SfxKind::Error));
            }
            Some(_) if energy.cur < 4.0 => {
                notify.write(Notify("Too tired to draw the bow.".into()));
                sfx.write(PlaySfx(SfxKind::Error));
            }
            Some(bow) => {
                player.bow_cd = 0.45;
                if !(hero.has_perk(Perk::Fletcher) && rng.random_bool(0.5)) {
                    hero.arrows -= 1;
                }
                energy.cur -= 4.0;
                energy.since_use = 0.0;
                let tier = &BOWS[bow];
                let dirs: Vec<Vec2> = if hero.has_perk(Perk::Volley) {
                    vec![
                        Vec2::from_angle(0.12).rotate(facing.0),
                        Vec2::from_angle(-0.12).rotate(facing.0),
                    ]
                } else {
                    vec![facing.0]
                };
                for d in dirs {
                    spawn_projectile(
                        &mut commands,
                        &atlas,
                        SpriteId::Arrow,
                        pos + d * 6.0,
                        d,
                        tier.arrow_speed,
                        hero.bow_damage(),
                        Faction::Hero,
                        hero.has_perk(Perk::Sharpshooter),
                        hero.arrow_type,
                    );
                }
                sfx.write(PlaySfx(SfxKind::Bow));
            }
        }
    }

    if controls.cycle {
        if hero.quivers.is_empty() {
            notify.write(Notify(
                "You only have plain arrows. Shops sell enchanted quivers.".into(),
            ));
        } else {
            hero.cycle_arrows();
            sfx.write(PlaySfx(SfxKind::Menu));
            let name = hero.arrow_type.map(|e| e.name()).unwrap_or("Plain");
            notify.write(Notify(format!("{name} arrows loaded.")));
        }
    }

    if controls.potion {
        if hero.potions == 0 {
            notify.write(Notify("No potions left.".into()));
            sfx.write(PlaySfx(SfxKind::Error));
        } else if health.hp >= health.max {
            notify.write(Notify("You are already at full health.".into()));
            sfx.write(PlaySfx(SfxKind::Error));
        } else {
            hero.potions -= 1;
            health.hp = (health.hp + hero.potion_heal()).min(health.max);
            spawn_particles(&mut commands, pos, crate::palette::GREEN, 12, 40.0, 0.6);
            sfx.write(PlaySfx(SfxKind::Potion));
            notify.write(Notify(format!(
                "You drink a potion. {} potions left.",
                hero.potions
            )));
        }
    }

    let near_shop = shopkeepers
        .iter()
        .find(|(_, t)| t.translation.truncate().distance(pos) < 24.0)
        .map(|(e, _)| e);
    if let Some(shopkeeper) = near_shop {
        if controls.interact {
            commands.insert_resource(ShopContext {
                shopkeeper,
                selected: 0,
            });
            sfx.write(PlaySfx(SfxKind::Select));
            next.set(GameState::Shop);
            return;
        } else if player.hint_timer <= 0.0 {
            player.hint_timer = 3.0;
            notify.write(Notify("Press E to trade with the shopkeeper.".into()));
        }
    }

    if controls.interact {
        for (t, mut chest, mut sprite) in &mut chests {
            if chest.open || t.translation.truncate().distance(pos) > 22.0 {
                continue;
            }
            chest.open = true;
            if let Some(atlas_ref) = sprite.texture_atlas.as_mut() {
                atlas_ref.index = SpriteId::ChestOpen.index(0);
            }
            sfx.write(PlaySfx(SfxKind::Chest));
            let cpos = t.translation.truncate();
            spawn_particles(&mut commands, cpos, crate::palette::YELLOW, 8, 40.0, 0.5);
            match chest.loot {
                ChestLoot::Key => {
                    spawn_pickup(
                        &mut commands,
                        &atlas,
                        cpos,
                        PickupKind::Key,
                        Vec2::new(0.0, 40.0),
                    );
                    notify.write(Notify("The stairway key! Now find the stairs down.".into()));
                }
                ChestLoot::Coins(n) => {
                    let value = hero.coin_value(n);
                    hero.coins += value;
                    notify.write(Notify(format!("A stash of {value} coins!")));
                    sfx.write(PlaySfx(SfxKind::Coin));
                }
                ChestLoot::Potion => {
                    hero.potions += 1;
                    notify.write(Notify("A healing potion.".into()));
                }
                ChestLoot::Arrows(n) => {
                    hero.arrows += n;
                    notify.write(Notify(format!("{n} arrows.")));
                }
            }
            break;
        }
    }
}

/// F / right bumper: the class's special ability.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn special_ability(
    mut commands: Commands,
    controls: Res<Controls>,
    atlas: Res<Atlas>,
    hero: Res<Hero>,
    floor: Res<CurrentFloor>,
    mut q: Query<(Entity, &mut Transform, &Facing, &Hitbox, &mut Player)>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<Player>>,
    mut damage: MessageWriter<DamageEnemy>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut shake: ResMut<Shake>,
) {
    if !controls.special {
        return;
    }
    let Ok((entity, mut transform, facing, hitbox, mut player)) = q.single_mut() else {
        return;
    };
    let class = hero.class;
    if player.special_cd > 0.0 {
        notify.write(Notify(format!(
            "{} is recharging ({:.1}s).",
            class.ability_name(),
            player.special_cd
        )));
        sfx.write(PlaySfx(SfxKind::Error));
        return;
    }
    let pos = transform.translation.truncate();
    player.special_cd = class.cooldown();
    sfx.write(PlaySfx(SfxKind::Special));
    match class {
        Class::Knight => {
            player.bash_left = BASH_TIME;
            player.bash_hit.clear();
            shake.add(0.2);
            notify.write(Notify("Shield Bash!".into()));
        }
        Class::Ranger => {
            let element = if hero.bow.is_some() {
                hero.arrow_type
            } else {
                None
            };
            let (speed, dmg) = match hero.bow {
                Some(b) => (BOWS[b].arrow_speed, hero.bow_damage()),
                None => (200.0, 2),
            };
            for i in 0..12 {
                let d = Vec2::from_angle(i as f32 / 12.0 * std::f32::consts::TAU);
                spawn_projectile(
                    &mut commands,
                    &atlas,
                    SpriteId::Arrow,
                    pos + d * 6.0,
                    d,
                    speed,
                    dmg,
                    Faction::Hero,
                    hero.has_perk(Perk::Sharpshooter),
                    element,
                );
            }
            sfx.write(PlaySfx(SfxKind::Bow));
            notify.write(Notify("Arrow Storm!".into()));
        }
        Class::Mage => {
            let radius = 70.0;
            for (e, t, enemy) in &enemies {
                let epos = t.translation.truncate();
                if epos.distance(pos) < radius + enemy.radius {
                    damage.write(DamageEnemy {
                        target: e,
                        amount: 3,
                        from: pos,
                        knockback: 60.0,
                        arrow: false,
                        element: Some(Element::Frost),
                    });
                }
            }
            // A ring of frost bursting outward.
            for i in 0..24 {
                let d = Vec2::from_angle(i as f32 / 24.0 * std::f32::consts::TAU);
                commands.spawn((
                    FloorEntity,
                    Particle {
                        vel: d * 160.0,
                        drag: 0.9,
                        gravity: 0.0,
                    },
                    Lifetime(0.45),
                    Sprite::from_color(crate::palette::CYAN, Vec2::splat(2.0)),
                    Transform::from_translation(pos.extend(layer::PARTICLE)),
                ));
            }
            spawn_particles(&mut commands, pos, crate::palette::WHITE, 10, 60.0, 0.4);
            sfx.write(PlaySfx(SfxKind::Spike));
            shake.add(0.3);
            notify.write(Notify("Frost Nova! Everything nearby freezes.".into()));
        }
        Class::Rogue => {
            // Step through anything short of a wall.
            spawn_particles(&mut commands, pos, crate::palette::PURPLE, 10, 50.0, 0.4);
            let mut p = pos;
            for _ in 0..14 {
                let (next, blocked) = move_box(&floor.dungeon, p, facing.0 * 4.0, hitbox.0);
                p = next;
                if blocked.any() {
                    break;
                }
            }
            transform.translation.x = p.x;
            transform.translation.y = p.y;
            spawn_particles(&mut commands, p, crate::palette::PURPLE, 10, 50.0, 0.4);
            commands.entity(entity).insert(Invulnerable(1.2));
            player.ambush = true;
            sfx.write(PlaySfx(SfxKind::Teleport));
            notify.write(Notify(
                "Shadow Step! Your next sword strike hits three times as hard.".into(),
            ));
        }
    }
}

/// A charging Knight bowls over every monster in the way.
fn bash_hits(
    hero: Res<Hero>,
    mut player: Query<(&Transform, &mut Player)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut damage: MessageWriter<DamageEnemy>,
    mut shake: ResMut<Shake>,
) {
    let Ok((pt, mut player)) = player.single_mut() else {
        return;
    };
    if player.bash_left <= 0.0 {
        return;
    }
    let pos = pt.translation.truncate();
    for (e, t, enemy) in &enemies {
        if player.bash_hit.contains(&e)
            || t.translation.truncate().distance(pos) > enemy.radius + 10.0
        {
            continue;
        }
        player.bash_hit.push(e);
        damage.write(DamageEnemy {
            target: e,
            amount: hero.sword_damage() * 2 + 2,
            from: pos,
            knockback: 300.0,
            arrow: false,
            element: None,
        });
        shake.add(0.25);
    }
}

fn player_stairs(
    mut hero: ResMut<Hero>,
    floor: Res<CurrentFloor>,
    mut q: Query<(&Transform, &mut Player)>,
    bosses: Query<(), With<Boss>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok((transform, mut player)) = q.single_mut() else {
        return;
    };
    let pos = transform.translation.truncate();
    if tile_of(pos) != floor.dungeon.exit || pos.distance(tile_center(floor.dungeon.exit)) > ON_TILE
    {
        return;
    }
    if boss_alive(&floor, &bosses) {
        if player.hint_timer <= 0.0 {
            player.hint_timer = 2.5;
            notify.write(Notify(
                "The Lich's magic seals the way out. Destroy it!".into(),
            ));
        }
    } else if hero.keys > 0 {
        hero.keys -= 1;
        sfx.write(PlaySfx(SfxKind::Stairs));
        if hero.floor >= MAX_FLOOR {
            next.set(GameState::Victory);
        } else {
            next.set(GameState::SkillChoice);
        }
    } else if player.hint_timer <= 0.0 {
        player.hint_timer = 2.5;
        sfx.write(PlaySfx(SfxKind::Error));
        notify.write(Notify(
            "The stairway is locked. Find the key on this floor.".into(),
        ));
    }
}

fn regen(time: Res<Time>, hero: Res<Hero>, mut q: Query<(&mut Energy, &mut Health, &mut Player)>) {
    let dt = time.delta_secs();
    let Ok((mut energy, mut health, mut player)) = q.single_mut() else {
        return;
    };
    energy.since_use += dt;
    if energy.since_use > 0.7 {
        energy.cur = (energy.cur + hero.energy_regen() * dt).min(energy.max);
    }
    if hero.has_magic(MagicItem::RingOfRegeneration) {
        player.regen_acc += dt;
        if player.regen_acc >= 2.0 {
            player.regen_acc = 0.0;
            health.hp = (health.hp + 1).min(health.max);
        }
    }
}

fn sync_hero_maxes(hero: Res<Hero>, mut q: Query<(&mut Health, &mut Energy), With<Player>>) {
    if !hero.is_changed() {
        return;
    }
    for (mut health, mut energy) in &mut q {
        health.max = hero.max_hp();
        health.hp = health.hp.min(health.max);
        energy.max = hero.max_energy();
        energy.cur = energy.cur.min(energy.max);
    }
}

#[allow(clippy::too_many_arguments)]
fn pickups(
    mut commands: Commands,
    time: Res<Time>,
    floor: Res<CurrentFloor>,
    mut hero: ResMut<Hero>,
    mut q: Query<(Entity, &mut Transform, &mut Pickup)>,
    player: Query<&Transform, (With<Player>, Without<Pickup>)>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
) {
    let dt = time.delta_secs();
    let ppos = player.single().map(|t| t.translation.truncate()).ok();
    for (entity, mut transform, mut pickup) in &mut q {
        pickup.age += dt;
        let mut pos = transform.translation.truncate();
        if pickup.vel != Vec2::ZERO {
            let (new_pos, _) = move_box(&floor.dungeon, pos, pickup.vel * dt, Vec2::splat(3.0));
            pos = new_pos;
            pickup.vel *= (1.0 - 5.0 * dt).max(0.0);
            if pickup.vel.length() < 4.0 {
                pickup.vel = Vec2::ZERO;
            }
        }
        let Some(ppos) = ppos else {
            transform.translation = pos.extend(layer::ITEM);
            continue;
        };
        let dist = ppos.distance(pos);
        let magnet = matches!(pickup.kind, PickupKind::Coin(_)) && pickup.age > 0.4 && dist < 44.0;
        if magnet {
            pos += (ppos - pos).normalize_or_zero() * 140.0 * dt;
        }
        transform.translation = pos.extend(layer::ITEM);
        // Coins bob gently; everything else sits still on the floor.
        if matches!(pickup.kind, PickupKind::Coin(_)) {
            transform.translation.y += ((pickup.age * 4.0 + pickup.phase).sin() * 1.2).round();
        }

        if dist < 9.0 && pickup.age > 0.25 {
            match pickup.kind {
                PickupKind::Coin(v) => {
                    hero.coins += hero.coin_value(v);
                    sfx.write(PlaySfx(SfxKind::Coin));
                    spawn_particles(&mut commands, pos, crate::palette::YELLOW, 3, 30.0, 0.3);
                }
                PickupKind::Potion => {
                    hero.potions += 1;
                    sfx.write(PlaySfx(SfxKind::Potion));
                    notify.write(Notify("Picked up a healing potion.".into()));
                }
                PickupKind::Arrows(n) => {
                    hero.arrows += n;
                    sfx.write(PlaySfx(SfxKind::Select));
                    notify.write(Notify(format!("Picked up {n} arrows.")));
                }
                PickupKind::Key => {
                    hero.keys += 1;
                    sfx.write(PlaySfx(SfxKind::Key));
                    notify.write(Notify("You have the stairway key.".into()));
                }
            }
            commands.entity(entity).despawn();
        }
    }
}
