//! Heads-up display: bars, inventory, minimap, messages and banners.

use crate::{
    art::{Atlas, SpriteId},
    dungeon::{MAX_FLOOR, RoomKind, Spawn, Tile},
    game::*,
    items::{ARMORS, BOWS, SWORDS},
    palette,
    physics::tile_of,
    player::{Chest, Player},
};
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// Which tiles the hero has seen on this floor.
#[derive(Resource)]
pub struct Explored {
    pub w: i32,
    pub h: i32,
    pub seen: Vec<bool>,
}

impl Explored {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            seen: vec![false; (w * h) as usize],
        }
    }

    fn mark(&mut self, p: IVec2) {
        if p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h {
            self.seen[(p.y * self.w + p.x) as usize] = true;
        }
    }

    fn is_seen(&self, p: IVec2) -> bool {
        p.x >= 0
            && p.y >= 0
            && p.x < self.w
            && p.y < self.h
            && self.seen[(p.y * self.w + p.x) as usize]
    }
}

#[derive(Resource)]
struct Minimap {
    image: Handle<Image>,
    w: i32,
    h: i32,
    timer: f32,
}

#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct HpText;
#[derive(Component)]
struct EnergyFill;
#[derive(Component)]
struct XpFill;
#[derive(Component)]
struct LevelText;
#[derive(Component)]
struct CoinText;
#[derive(Component)]
struct PotionText;
#[derive(Component)]
struct ArrowText;
#[derive(Component)]
struct KeyText;
#[derive(Component)]
struct GearText;
#[derive(Component)]
struct SpecialText;
#[derive(Component)]
struct FloorText;
#[derive(Component)]
struct MinimapNode;
#[derive(Component)]
struct MessageText(f32);
#[derive(Component)]
struct BannerText(f32);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build_hud).add_systems(
            Update,
            (
                hud_visibility,
                update_hud,
                update_special,
                update_minimap,
                messages,
                banners,
            ),
        );
    }
}

pub fn text(s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

pub fn icon(atlas: &Atlas, id: SpriteId, size: f32) -> impl Bundle {
    (
        ImageNode {
            image: atlas.image.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: atlas.layout.clone(),
                index: id.index(0),
            }),
            ..default()
        },
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
    )
}

fn bar(width: f32, height: f32, color: Color, marker: impl Component) -> impl Bundle {
    (
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor::all(palette::OUTLINE),
        BackgroundColor(palette::NIGHT),
        children![(
            marker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(color),
        )],
    )
}

fn build_hud(mut commands: Commands, atlas: Res<Atlas>, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = ImageSampler::nearest();
    let minimap = images.add(image);
    commands.insert_resource(Minimap {
        image: minimap.clone(),
        w: 0,
        h: 0,
        timer: 0.0,
    });

    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // Top-left: vitals and inventory.
            root.spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },))
                .with_children(|col| {
                    col.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|row| {
                            row.spawn((icon(&atlas, SpriteId::Heart, 24.0),));
                            row.spawn(bar(200.0, 18.0, palette::RED, HpFill));
                            row.spawn((HpText, text("20 / 20", 16.0, palette::WHITE)));
                        });
                    col.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|row| {
                            row.spawn((icon(&atlas, SpriteId::Crystal, 24.0),));
                            row.spawn(bar(200.0, 14.0, palette::TEAL, EnergyFill));
                            row.spawn(text("Shift: sprint   L: block", 13.0, palette::GREY));
                        });
                    col.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|row| {
                            row.spawn((LevelText, text("Lv 1", 15.0, palette::YELLOW)));
                            row.spawn(bar(160.0, 8.0, palette::YELLOW, XpFill));
                        });
                    col.spawn((SpecialText, text("", 14.0, palette::CYAN)));
                    col.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },))
                        .with_children(|row| {
                            row.spawn((icon(&atlas, SpriteId::Coin, 24.0),));
                            row.spawn((CoinText, text("0", 16.0, palette::YELLOW)));
                            row.spawn((icon(&atlas, SpriteId::Potion, 24.0),));
                            row.spawn((PotionText, text("1", 16.0, palette::WHITE)));
                            row.spawn((icon(&atlas, SpriteId::Arrows, 24.0),));
                            row.spawn((ArrowText, text("0", 16.0, palette::WHITE)));
                            row.spawn((icon(&atlas, SpriteId::Key, 24.0),));
                            row.spawn((KeyText, text("0", 16.0, palette::WHITE)));
                        });
                    col.spawn((GearText, text("", 13.0, palette::LIGHT_GREY)));
                });

            // Top-right: floor and minimap.
            root.spawn((Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(4.0),
                ..default()
            },))
                .with_children(|col| {
                    col.spawn((FloorText, text("Floor 1 / 8", 16.0, palette::WHITE)));
                    col.spawn((
                        MinimapNode,
                        ImageNode {
                            image: minimap.clone(),
                            ..default()
                        },
                        Node {
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(palette::OUTLINE),
                        BackgroundColor(palette::VOID.with_alpha(0.6)),
                    ));
                });

            // Bottom-centre: message line.
            root.spawn((Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(18.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((
                        MessageText(0.0),
                        text("", 17.0, palette::WHITE),
                        TextLayout::justify(Justify::Center),
                    ));
                });

            // Centre: banner.
            root.spawn((Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((BannerText(0.0), text("", 44.0, palette::YELLOW)));
                });
        });
}

fn hud_visibility(state: Res<State<GameState>>, mut root: Query<&mut Visibility, With<HudRoot>>) {
    if !state.is_changed() {
        return;
    }
    let visible = !matches!(
        state.get(),
        GameState::Title | GameState::ClassSelect | GameState::GameOver | GameState::Victory
    );
    for mut v in &mut root {
        *v = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    hero: Res<Hero>,
    player: Query<(&Health, &Energy), With<Player>>,
    mut fills: ParamSet<(
        Query<&mut Node, With<HpFill>>,
        Query<&mut Node, With<EnergyFill>>,
        Query<&mut Node, With<XpFill>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HpText>>,
        Query<&mut Text, With<LevelText>>,
        Query<&mut Text, With<CoinText>>,
        Query<&mut Text, With<PotionText>>,
        Query<&mut Text, With<ArrowText>>,
        Query<&mut Text, With<KeyText>>,
        Query<&mut Text, With<GearText>>,
        Query<&mut Text, With<FloorText>>,
    )>,
) {
    let Ok((health, energy)) = player.single() else {
        return;
    };
    let set = |q: &mut Query<&mut Node, With<HpFill>>, pct: f32| {
        for mut n in q.iter_mut() {
            n.width = Val::Percent(pct.clamp(0.0, 100.0));
        }
    };
    set(
        &mut fills.p0(),
        health.hp as f32 / health.max.max(1) as f32 * 100.0,
    );
    for mut n in fills.p1().iter_mut() {
        n.width = Val::Percent((energy.cur / energy.max.max(1.0) * 100.0).clamp(0.0, 100.0));
    }
    for mut n in fills.p2().iter_mut() {
        n.width =
            Val::Percent((hero.xp as f32 / hero.xp_to_next() as f32 * 100.0).clamp(0.0, 100.0));
    }
    let write = |q: &mut Query<&mut Text, With<HpText>>, s: String| {
        for mut t in q.iter_mut() {
            t.0 = s.clone();
        }
    };
    write(
        &mut texts.p0(),
        format!("{} / {}", health.hp.max(0), health.max),
    );
    for mut t in texts.p1().iter_mut() {
        t.0 = format!("Lv {}", hero.level);
    }
    for mut t in texts.p2().iter_mut() {
        t.0 = hero.coins.to_string();
    }
    for mut t in texts.p3().iter_mut() {
        t.0 = hero.potions.to_string();
    }
    for mut t in texts.p4().iter_mut() {
        t.0 = hero.arrows.to_string();
    }
    for mut t in texts.p5().iter_mut() {
        t.0 = hero.keys.to_string();
    }
    for mut t in texts.p6().iter_mut() {
        let bow = match (hero.bow, hero.arrow_type) {
            (None, _) => "no bow".to_string(),
            (Some(b), None) => BOWS[b].name.to_string(),
            (Some(b), Some(e)) => format!("{} ({} arrows)", BOWS[b].name, e.name()),
        };
        let armor = hero.armor.map(|a| ARMORS[a].name).unwrap_or("no armor");
        let magic = if hero.magic.is_empty() {
            String::new()
        } else {
            format!(
                "  |  {}",
                hero.magic
                    .iter()
                    .map(|m| m.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let perks = if hero.perks.is_empty() {
            String::new()
        } else {
            format!(
                "\nPerks: {}",
                hero.perks
                    .iter()
                    .map(|p| p.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        t.0 = format!(
            "{}  |  {}  |  {}  |  {}{}{}",
            hero.class.name(),
            SWORDS[hero.sword].name,
            bow,
            armor,
            magic,
            perks
        );
    }
    for mut t in texts.p7().iter_mut() {
        t.0 = format!("Floor {} / {}", hero.floor, MAX_FLOOR);
    }
}

fn update_special(
    hero: Res<Hero>,
    player: Query<&Player>,
    mut q: Query<(&mut Text, &mut TextColor), With<SpecialText>>,
) {
    let Ok(player) = player.single() else { return };
    for (mut t, mut color) in &mut q {
        let ability = hero.class.ability_name();
        if player.special_cd > 0.0 {
            t.0 = format!("F: {ability}  {:.1}s", player.special_cd);
            color.0 = palette::GREY;
        } else {
            t.0 = format!("F: {ability}  READY");
            color.0 = palette::CYAN;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_minimap(
    time: Res<Time>,
    floor: Option<Res<CurrentFloor>>,
    mut explored: Option<ResMut<Explored>>,
    mut minimap: ResMut<Minimap>,
    mut images: ResMut<Assets<Image>>,
    player: Query<&Transform, With<Player>>,
    chests: Query<(&Transform, &Chest)>,
    mut node: Query<&mut Node, With<MinimapNode>>,
) {
    let (Some(floor), Some(explored)) = (floor, explored.as_mut()) else {
        return;
    };
    let Ok(pt) = player.single() else { return };
    let d = &floor.dungeon;
    let ptile = tile_of(pt.translation.truncate());

    // Reveal the room the hero stands in, plus a small radius anywhere.
    if let Some(i) = d.room_at(ptile) {
        let r = &d.rooms[i];
        for y in r.y - 1..=r.y + r.h {
            for x in r.x - 1..=r.x + r.w {
                explored.mark(IVec2::new(x, y));
            }
        }
    }
    for dy in -3..=3 {
        for dx in -3..=3 {
            explored.mark(ptile + IVec2::new(dx, dy));
        }
    }

    minimap.timer -= time.delta_secs();
    if minimap.timer > 0.0 {
        return;
    }
    minimap.timer = 0.1;

    let scale = 3.0;
    if minimap.w != d.w || minimap.h != d.h {
        minimap.w = d.w;
        minimap.h = d.h;
        if let Some(mut image) = images.get_mut(&minimap.image) {
            image.resize(Extent3d {
                width: d.w as u32,
                height: d.h as u32,
                depth_or_array_layers: 1,
            });
        }
        for mut n in &mut node {
            n.width = Val::Px(d.w as f32 * scale + 4.0);
            n.height = Val::Px(d.h as f32 * scale + 4.0);
        }
    }
    let Some(mut image) = images.get_mut(&minimap.image) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let chest_tiles: Vec<(IVec2, bool)> = chests
        .iter()
        .map(|(t, c)| (tile_of(t.translation.truncate()), c.open))
        .collect();
    for y in 0..d.h {
        for x in 0..d.w {
            let p = IVec2::new(x, y);
            let row = (d.h - 1 - y) as usize;
            let i = (row * d.w as usize + x as usize) * 4;
            let color: [u8; 4] = if !explored.is_seen(p) {
                [0, 0, 0, 0]
            } else if p == ptile {
                [255, 255, 255, 255]
            } else if p == d.exit {
                [99, 199, 77, 255]
            } else if let Some((_, open)) = chest_tiles.iter().find(|(t, _)| *t == p) {
                if *open {
                    [120, 90, 60, 255]
                } else {
                    [254, 174, 52, 255]
                }
            } else if d
                .spawns
                .iter()
                .any(|(q, s)| *q == p && *s == Spawn::Shopkeeper)
            {
                [254, 231, 97, 255]
            } else {
                match d.get(p) {
                    Tile::Wall | Tile::Cracked => [58, 68, 102, 255],
                    Tile::Carpet => [140, 47, 59, 255],
                    Tile::Spikes => [160, 60, 60, 255],
                    _ => {
                        if d.room_at(p).map(|i| d.rooms[i].kind) == Some(RoomKind::Secret) {
                            [168, 107, 217, 255]
                        } else {
                            [139, 155, 180, 255]
                        }
                    }
                }
            };
            data[i..i + 4].copy_from_slice(&color);
        }
    }
}

fn messages(
    time: Res<Time>,
    mut reader: MessageReader<Notify>,
    mut q: Query<(&mut MessageText, &mut Text, &mut TextColor)>,
) {
    let Ok((mut msg, mut text, mut color)) = q.single_mut() else {
        reader.clear();
        return;
    };
    for Notify(s) in reader.read() {
        text.0 = s.clone();
        msg.0 = 4.0;
    }
    msg.0 -= time.delta_secs();
    color.0 = palette::WHITE.with_alpha(msg.0.clamp(0.0, 1.0));
}

fn banners(
    time: Res<Time>,
    mut reader: MessageReader<Banner>,
    mut q: Query<(&mut BannerText, &mut Text, &mut TextColor)>,
) {
    let Ok((mut banner, mut text, mut color)) = q.single_mut() else {
        reader.clear();
        return;
    };
    for Banner(s) in reader.read() {
        text.0 = s.clone();
        banner.0 = 2.2;
    }
    banner.0 -= time.delta_secs();
    color.0 = palette::YELLOW.with_alpha(banner.0.clamp(0.0, 1.0));
}
