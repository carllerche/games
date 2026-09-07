//! Keyboard and gamepad input folded into one per-frame snapshot.

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Controls {
    pub move_dir: Vec2,
    pub sprint: bool,
    pub attack: bool,
    pub bow: bool,
    pub potion: bool,
    pub interact: bool,
    pub cycle: bool,
    pub confirm: bool,
    pub cancel: bool,
    pub pause: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    prev_stick: Vec2,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Controls>()
            .add_systems(PreUpdate, read_input);
    }
}

pub fn read_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut c: ResMut<Controls>) {
    use KeyCode::*;
    let key = |k: &[KeyCode]| k.iter().any(|k| keys.pressed(*k));
    let just = |k: &[KeyCode]| k.iter().any(|k| keys.just_pressed(*k));

    let mut dir = Vec2::ZERO;
    if key(&[KeyW, ArrowUp]) {
        dir.y += 1.0;
    }
    if key(&[KeyS, ArrowDown]) {
        dir.y -= 1.0;
    }
    if key(&[KeyA, ArrowLeft]) {
        dir.x -= 1.0;
    }
    if key(&[KeyD, ArrowRight]) {
        dir.x += 1.0;
    }

    c.sprint = key(&[ShiftLeft, ShiftRight]);
    c.attack = just(&[KeyJ, Space]);
    c.bow = just(&[KeyK]);
    c.potion = just(&[KeyQ]);
    c.interact = just(&[KeyE]);
    c.cycle = just(&[Tab]);
    c.confirm = just(&[Enter, Space, KeyE, KeyJ]);
    c.cancel = just(&[Escape, KeyQ]);
    c.pause = just(&[Escape]);
    c.up = just(&[KeyW, ArrowUp]);
    c.down = just(&[KeyS, ArrowDown]);
    c.left = just(&[KeyA, ArrowLeft]);
    c.right = just(&[KeyD, ArrowRight]);

    let mut stick = Vec2::ZERO;
    for pad in &pads {
        use GamepadButton::*;
        let s = pad.left_stick();
        if s.length() > 0.25 {
            stick = s;
        }
        if pad.pressed(DPadUp) {
            stick.y = 1.0;
        }
        if pad.pressed(DPadDown) {
            stick.y = -1.0;
        }
        if pad.pressed(DPadLeft) {
            stick.x = -1.0;
        }
        if pad.pressed(DPadRight) {
            stick.x = 1.0;
        }
        c.sprint |= pad.pressed(RightTrigger2) || pad.pressed(LeftTrigger2) || pad.pressed(RightTrigger);
        c.attack |= pad.just_pressed(South);
        c.bow |= pad.just_pressed(West);
        c.potion |= pad.just_pressed(North);
        c.interact |= pad.just_pressed(East);
        c.cycle |= pad.just_pressed(LeftTrigger);
        c.confirm |= pad.just_pressed(South) || pad.just_pressed(Start);
        c.cancel |= pad.just_pressed(East);
        c.pause |= pad.just_pressed(Start);
        c.up |= pad.just_pressed(DPadUp);
        c.down |= pad.just_pressed(DPadDown);
        c.left |= pad.just_pressed(DPadLeft);
        c.right |= pad.just_pressed(DPadRight);
    }
    // Edge-detect the analogue stick for menu navigation.
    let prev = c.prev_stick;
    c.up |= stick.y > 0.5 && prev.y <= 0.5;
    c.down |= stick.y < -0.5 && prev.y >= -0.5;
    c.left |= stick.x < -0.5 && prev.x >= -0.5;
    c.right |= stick.x > 0.5 && prev.x <= 0.5;
    c.prev_stick = stick;

    if dir == Vec2::ZERO {
        dir = stick;
    }
    c.move_dir = if dir.length() > 1.0 { dir.normalize() } else { dir };
}
