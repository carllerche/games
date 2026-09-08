//! Screens and overlays: the title screen, the in-game HUD, the big centre
//! banners ("Level 3", "Wrong yarn!"), and the victory screen.

use crate::cat::{Cat, Slimy};
use crate::level::{LEVEL_COUNT, YarnColor};
use crate::{AppState, CurrentLevel, LoadLevel, Phase, Sfx};
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Title), spawn_title)
            .add_systems(Update, title_input.run_if(in_state(AppState::Title)))
            .add_systems(OnEnter(AppState::Playing), spawn_hud)
            .add_systems(
                Update,
                (update_hud, playing_hotkeys).run_if(in_state(AppState::Playing)),
            )
            .add_systems(OnEnter(AppState::Victory), spawn_victory)
            .add_systems(Update, victory_input.run_if(in_state(AppState::Victory)));
    }
}

const TEXT: Color = Color::srgb_u8(60, 40, 30);
const PANEL: Color = Color::srgba(1.0, 0.98, 0.9, 0.85);

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn panel() -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(px(18), px(10)),
            border_radius: BorderRadius::all(px(14)),
            ..default()
        },
        BackgroundColor(PANEL),
    )
}

// ---------------------------------------------------------------------------
// Title screen
// ---------------------------------------------------------------------------

fn spawn_title(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::Title),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: px(18),
            ..default()
        },
        BackgroundColor(Color::srgb_u8(255, 200, 120)),
        children![
            (
                Text::new("CAT POWNCER"),
                font(96.0),
                TextColor(Color::srgb_u8(120, 50, 160)),
                TextShadow::default(),
            ),
            (
                Text::new("Hop across the giant cat tree and find the PURPLE yarn ball!"),
                font(28.0),
                TextColor(TEXT),
            ),
            (
                panel(),
                children![(
                    Text::new(
                        "Arrow keys or WASD: hop     Space: swipe\n\
                         Click a tile to hop there, click a toy or dog to swipe it\n\n\
                         Swipe toys off the platforms.  Scratch dogs before they lick you!\n\
                         Grab only PURPLE yarn.  Any other colour sends you back to the start.\n\
                         22 levels to conquer."
                    ),
                    font(22.0),
                    TextColor(TEXT),
                    TextLayout::justify(Justify::Center),
                )],
            ),
            (
                Text::new("Press SPACE or click to start"),
                font(34.0),
                TextColor(Color::srgb_u8(200, 60, 60)),
                Blink,
            ),
        ],
    ));
}

#[derive(Component)]
struct Blink;

fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut next: ResMut<NextState<AppState>>,
    mut blink: Query<&mut TextColor, With<Blink>>,
) {
    for mut color in &mut blink {
        let a = 0.6 + 0.4 * (time.elapsed_secs() * 4.0).sin();
        color.0 = Color::srgba(0.8, 0.25, 0.25, a);
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) || mouse.just_pressed(MouseButton::Left) {
        next.set(AppState::Playing);
    }
}

// ---------------------------------------------------------------------------
// In-game HUD
// ---------------------------------------------------------------------------

#[derive(Component)]
struct LevelText;

#[derive(Component)]
struct BannerText;

#[derive(Component)]
struct SlimyText;

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::Playing),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(px(16)),
            ..default()
        },
        Pickable::IGNORE,
        children![
            // Top row.
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (panel(), children![(Text::new("Level 1 / 22"), font(30.0), TextColor(TEXT), LevelText)]),
                    (
                        panel(),
                        children![(
                            Text::new("Find the "),
                            font(30.0),
                            TextColor(TEXT),
                            children![
                                (TextSpan::new("PURPLE"), font(30.0), TextColor(YarnColor::Purple.color())),
                                (TextSpan::new(" yarn!"), font(30.0), TextColor(TEXT)),
                            ],
                        )],
                    ),
                ],
            ),
            // Centre banner.
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
                children![(
                    Text::new(""),
                    font(64.0),
                    TextColor(Color::srgb_u8(120, 50, 160)),
                    TextShadow::default(),
                    TextLayout::justify(Justify::Center),
                    BannerText,
                )],
            ),
            // Bottom row.
            (
                Node {
                    width: percent(100),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::End,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        panel(),
                        children![(
                            Text::new("Arrows / WASD: hop    Space: swipe    Click: go there    R: restart level"),
                            font(20.0),
                            TextColor(TEXT),
                        )],
                    ),
                    (
                        Text::new(""),
                        font(30.0),
                        TextColor(Color::srgb_u8(70, 140, 40)),
                        TextShadow::default(),
                        SlimyText,
                    ),
                ],
            ),
        ],
    ));
}

fn update_hud(
    level: Res<CurrentLevel>,
    phase: Res<Phase>,
    cat: Query<Has<Slimy>, With<Cat>>,
    mut level_text: Query<&mut Text, (With<LevelText>, Without<BannerText>, Without<SlimyText>)>,
    mut banner: Query<(&mut Text, &mut TextColor), (With<BannerText>, Without<LevelText>, Without<SlimyText>)>,
    mut slimy_text: Query<&mut Text, (With<SlimyText>, Without<LevelText>, Without<BannerText>)>,
) {
    for mut text in &mut level_text {
        let want = format!("Level {} / {}", level.0, LEVEL_COUNT);
        if text.0 != want {
            text.0 = want;
        }
    }
    for (mut text, mut color) in &mut banner {
        let (want, want_color) = match &*phase {
            Phase::Play => (String::new(), TEXT),
            Phase::Intro(_) => (format!("Level {}\nFind the PURPLE yarn!", level.0), Color::srgb_u8(120, 50, 160)),
            Phase::Wrong { color, .. } => (
                format!("Oops! That was {} yarn.\nBack to the start!", color.name()),
                color.color(),
            ),
            Phase::Complete(_) => (
                if level.0 >= LEVEL_COUNT {
                    "PURRFECT!\nThat was the last one!".to_string()
                } else {
                    format!("PURRFECT!\nLevel {} done!", level.0)
                },
                YarnColor::Purple.color(),
            ),
        };
        if text.0 != want {
            text.0 = want;
        }
        color.0 = want_color;
    }
    let slimy = cat.iter().any(|s| s);
    for mut text in &mut slimy_text {
        let want = if slimy { "SLIMY!  (slow...)" } else { "" };
        if text.0 != want {
            text.0 = want.to_string();
        }
    }
}

fn playing_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<Phase>,
    mut load: MessageWriter<LoadLevel>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Title);
    }
    if keys.just_pressed(KeyCode::KeyR) && phase.is_play() {
        load.write(LoadLevel);
    }
}

// ---------------------------------------------------------------------------
// Victory screen
// ---------------------------------------------------------------------------

fn spawn_victory(mut commands: Commands, mut sfx: MessageWriter<Sfx>) {
    sfx.write(Sfx::Win);
    commands.spawn((
        DespawnOnExit(AppState::Victory),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: px(24),
            ..default()
        },
        BackgroundColor(Color::srgb_u8(190, 140, 230)),
        children![
            (
                Text::new("YOU WIN!"),
                font(110.0),
                TextColor(Color::srgb_u8(255, 240, 120)),
                TextShadow::default(),
            ),
            (
                Text::new("You found all 22 purple yarn balls.\nWhat a clever cat!"),
                font(36.0),
                TextColor(Color::WHITE),
                TextLayout::justify(Justify::Center),
            ),
            (
                Text::new("Press SPACE or click to play again"),
                font(30.0),
                TextColor(Color::WHITE),
                Blink,
            ),
        ],
    ));
}

fn victory_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) || mouse.just_pressed(MouseButton::Left) {
        next.set(AppState::Title);
    }
}
