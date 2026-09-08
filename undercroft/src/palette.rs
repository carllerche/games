//! The game's fixed colour palette. Every pixel drawn by the game comes from
//! here, which is most of what gives it a coherent retro look.

use bevy::prelude::*;

const fn hex(rgb: u32) -> Color {
    Color::srgb_u8(
        (rgb >> 16) as u8,
        (rgb >> 8 & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

pub const OUTLINE: Color = hex(0x1a1423);
pub const WHITE: Color = hex(0xf4f4f0);
pub const LIGHT_GREY: Color = hex(0xc0cbdc);
pub const GREY: Color = hex(0x8b9bb4);
pub const DARK_GREY: Color = hex(0x5a6988);
pub const DARKER_GREY: Color = hex(0x3a4466);
pub const NIGHT: Color = hex(0x262b44);
pub const SKIN: Color = hex(0xe8b796);
pub const SKIN_SHADOW: Color = hex(0xc28569);
pub const HAIR: Color = hex(0x7a4a2b);
pub const BLUE: Color = hex(0x4d9be6);
pub const DARK_BLUE: Color = hex(0x2f5fa8);
pub const RED: Color = hex(0xe43b44);
pub const DARK_RED: Color = hex(0x9e2835);
pub const GREEN: Color = hex(0x63c74d);
pub const DARK_GREEN: Color = hex(0x3e8948);
pub const LIME: Color = hex(0x9bf06b);
pub const YELLOW: Color = hex(0xfee761);
pub const ORANGE: Color = hex(0xfeae34);
pub const DARK_ORANGE: Color = hex(0xf77622);
pub const BROWN: Color = hex(0xb86f50);
pub const DARK_BROWN: Color = hex(0x733e39);
pub const PURPLE: Color = hex(0xa86bd9);
pub const DARK_PURPLE: Color = hex(0x6d3ba0);
pub const CYAN: Color = hex(0x2ce8f5);
pub const TEAL: Color = hex(0x0099db);
pub const BONE: Color = hex(0xead4aa);
pub const BONE_SHADOW: Color = hex(0xb9a27a);
pub const PINK: Color = hex(0xf6757a);

/// Dungeon stonework.
pub const FLOOR: Color = hex(0x4a4a63);
pub const FLOOR_DARK: Color = hex(0x3d3d54);
pub const FLOOR_LIGHT: Color = hex(0x56567a);
pub const WALL: Color = hex(0x6b6b8a);
pub const WALL_DARK: Color = hex(0x4e4e6d);
pub const WALL_MORTAR: Color = hex(0x2b2b40);
pub const WALL_FACE: Color = hex(0x83839f);
pub const CARPET: Color = hex(0x8c2f3b);
pub const CARPET_TRIM: Color = hex(0xd9a441);
pub const VOID: Color = hex(0x0b0912);

/// Maps a character in a sprite pattern to a palette colour. `.` is transparent.
pub fn from_key(key: char) -> Option<Color> {
    Some(match key {
        '.' => return None,
        'k' => OUTLINE,
        'w' => WHITE,
        'W' => LIGHT_GREY,
        'e' => GREY,
        'E' => DARK_GREY,
        'D' => DARKER_GREY,
        'd' => NIGHT,
        's' => SKIN,
        'S' => SKIN_SHADOW,
        'h' => HAIR,
        'b' => BLUE,
        'B' => DARK_BLUE,
        'r' => RED,
        'R' => DARK_RED,
        'g' => GREEN,
        'G' => DARK_GREEN,
        'q' => LIME,
        'y' => YELLOW,
        'Y' => ORANGE,
        'o' => DARK_ORANGE,
        'n' => BROWN,
        'N' => DARK_BROWN,
        'p' => PURPLE,
        'P' => DARK_PURPLE,
        'c' => CYAN,
        'C' => TEAL,
        't' => BONE,
        'T' => BONE_SHADOW,
        'i' => PINK,
        'f' => FLOOR,
        'F' => FLOOR_DARK,
        'l' => FLOOR_LIGHT,
        'a' => WALL,
        'A' => WALL_DARK,
        'm' => WALL_MORTAR,
        'M' => WALL_FACE,
        'u' => CARPET,
        'U' => CARPET_TRIM,
        other => panic!("unknown palette key {other:?}"),
    })
}
