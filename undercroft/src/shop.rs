//! The shop overlay: browse a shopkeeper's stock and buy with coins,
//! gated by character level.

use crate::{
    art::Atlas,
    audio::{PlaySfx, SfxKind},
    game::*,
    hud::text,
    input::Controls,
    items::ShopItem,
    palette,
    player::{ShopContext, Shopkeeper},
};
use bevy::prelude::*;

#[derive(Component)]
struct ShopUi;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Shop), open_shop)
            .add_systems(Update, shop_input.run_if(in_state(GameState::Shop)))
            .add_systems(OnExit(GameState::Shop), close_shop);
    }
}

fn open_shop(
    mut commands: Commands,
    atlas: Res<Atlas>,
    hero: Res<Hero>,
    ctx: Res<ShopContext>,
    keepers: Query<&Shopkeeper>,
) {
    if let Ok(keeper) = keepers.get(ctx.shopkeeper) {
        build_ui(&mut commands, &atlas, &hero, &keeper.stock, ctx.selected);
    }
}

fn close_shop(mut commands: Commands, ui: Query<Entity, With<ShopUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
}

fn build_ui(
    commands: &mut Commands,
    atlas: &Atlas,
    hero: &Hero,
    stock: &[ShopItem],
    selected: usize,
) {
    commands
        .spawn((
            ShopUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.55)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(18.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(3.0)),
                    ..default()
                },
                BorderColor::all(palette::CARPET_TRIM),
                BackgroundColor(palette::NIGHT),
            ))
            .with_children(|panel| {
                panel.spawn(text("SHOPKEEPER", 28.0, palette::CARPET_TRIM));
                panel.spawn(text(
                    format!(
                        "\"Take a look, traveller.\"     Coins: {}     Level {}",
                        hero.coins, hero.level
                    ),
                    15.0,
                    palette::LIGHT_GREY,
                ));
                panel.spawn(Node {
                    height: Val::Px(8.0),
                    ..default()
                });
                for (i, item) in stock.iter().enumerate() {
                    let price = item.price(hero.floor);
                    let locked = hero.level < item.level();
                    let affordable = hero.coins >= price;
                    let is_selected = i == selected;
                    let (sprite, tint) = item.sprite();
                    let name_color = if locked {
                        palette::DARK_GREY
                    } else if is_selected {
                        palette::YELLOW
                    } else {
                        palette::WHITE
                    };
                    let price_color = if locked {
                        palette::DARK_GREY
                    } else if affordable {
                        palette::YELLOW
                    } else {
                        palette::RED
                    };
                    let label = if locked {
                        format!("{}   (requires level {})", item.name(), item.level())
                    } else {
                        item.name()
                    };
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(10.0),
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor::all(if is_selected {
                                palette::YELLOW
                            } else {
                                Color::NONE
                            }),
                            BackgroundColor(if is_selected {
                                palette::DARKER_GREY
                            } else {
                                Color::NONE
                            }),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                ImageNode {
                                    image: atlas.image.clone(),
                                    texture_atlas: Some(TextureAtlas {
                                        layout: atlas.layout.clone(),
                                        index: sprite.index(0),
                                    }),
                                    color: if locked { palette::DARK_GREY } else { tint },
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    ..default()
                                },
                            ));
                            row.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    ..default()
                                },
                                text(label, 17.0, name_color),
                            ));
                            row.spawn(text(format!("{price} c"), 17.0, price_color));
                        });
                }
                panel.spawn(Node {
                    height: Val::Px(8.0),
                    ..default()
                });
                let description = stock
                    .get(selected)
                    .map(|item| item.description(hero))
                    .unwrap_or_default();
                panel.spawn(text(description, 15.0, palette::LIGHT_GREY));
                panel.spawn(text(
                    "Up/Down: browse    Enter: buy    Esc: leave",
                    13.0,
                    palette::GREY,
                ));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn shop_input(
    mut commands: Commands,
    controls: Res<Controls>,
    atlas: Res<Atlas>,
    mut hero: ResMut<Hero>,
    mut ctx: ResMut<ShopContext>,
    mut keepers: Query<&mut Shopkeeper>,
    ui: Query<Entity, With<ShopUi>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut notify: MessageWriter<Notify>,
    mut next: ResMut<NextState<GameState>>,
) {
    if controls.cancel || controls.pause {
        next.set(GameState::Playing);
        return;
    }
    let Ok(mut keeper) = keepers.get_mut(ctx.shopkeeper) else {
        next.set(GameState::Playing);
        return;
    };
    let n = keeper.stock.len();
    let mut changed = false;
    if controls.up && n > 0 {
        ctx.selected = (ctx.selected + n - 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.down && n > 0 {
        ctx.selected = (ctx.selected + 1) % n;
        changed = true;
        sfx.write(PlaySfx(SfxKind::Menu));
    }
    if controls.confirm && n > 0 {
        let item = keeper.stock[ctx.selected];
        let price = item.price(hero.floor);
        if hero.level < item.level() {
            sfx.write(PlaySfx(SfxKind::Error));
            notify.write(Notify(format!(
                "You need to be level {} for that.",
                item.level()
            )));
        } else if hero.coins < price {
            sfx.write(PlaySfx(SfxKind::Error));
            notify.write(Notify("Not enough coins.".into()));
        } else {
            hero.coins -= price;
            let line = item.apply(&mut hero);
            notify.write(Notify(line));
            sfx.write(PlaySfx(SfxKind::Buy));
            if !matches!(item, ShopItem::Potion | ShopItem::Arrows) {
                keeper.stock.remove(ctx.selected);
                // Gear purchases retire lower tiers still on the shelf.
                keeper.stock.retain(|s| match (s, item) {
                    (ShopItem::Sword(a), ShopItem::Sword(b)) => a > &b,
                    (ShopItem::Bow(a), ShopItem::Bow(b)) => a > &b,
                    (ShopItem::Armor(a), ShopItem::Armor(b)) => a > &b,
                    _ => true,
                });
                if ctx.selected >= keeper.stock.len() && !keeper.stock.is_empty() {
                    ctx.selected = keeper.stock.len() - 1;
                }
            }
        }
        changed = true;
    }
    if changed {
        for e in &ui {
            commands.entity(e).despawn();
        }
        build_ui(&mut commands, &atlas, &hero, &keeper.stock, ctx.selected);
    }
}
