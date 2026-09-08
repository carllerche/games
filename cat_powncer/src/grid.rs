//! The logical grid the game is played on.
//!
//! Every platform tile lives at an integer `(x, z)` cell with an integer height
//! level. The cat and dogs hop between adjacent cells. World-space positions are
//! derived from cells with [`cell_to_world`].

use bevy::prelude::*;
use std::collections::HashMap;

/// Vertical distance between height levels, in world units.
pub const LEVEL_HEIGHT: f32 = 1.1;
/// Thickness of a carpeted island tile.
pub const TILE_THICKNESS: f32 = 0.5;
/// Height of the floor far below the islands (where a falling cat lands).
pub const FLOOR_Y: f32 = -7.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// A carpeted island tile; the index identifies the island for dog patrols.
    Island(usize),
    /// A narrow wooden plank connecting islands, running along `axis`.
    Bridge(Axis),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Z,
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub height: i32,
    pub kind: TileKind,
    /// Tiles at either end of a jump gap must stay clear so the gap is always crossable.
    pub keep_clear: bool,
}

impl Tile {
    pub fn island(&self) -> Option<usize> {
        match self.kind {
            TileKind::Island(i) => Some(i),
            TileKind::Bridge(_) => None,
        }
    }
}

/// The tile map for the current level.
#[derive(Resource, Default, Debug, Clone)]
pub struct Grid {
    pub tiles: HashMap<IVec2, Tile>,
    pub start: IVec2,
    pub max_height: i32,
}

impl Grid {
    pub fn tile(&self, cell: IVec2) -> Option<&Tile> {
        self.tiles.get(&cell)
    }

    pub fn height(&self, cell: IVec2) -> Option<i32> {
        self.tiles.get(&cell).map(|t| t.height)
    }

    /// World position of the top surface centre of a cell at a given height level.
    pub fn top_of(&self, cell: IVec2) -> Option<Vec3> {
        self.height(cell).map(|h| cell_to_world(cell, h))
    }
}

pub fn level_y(height: i32) -> f32 {
    height as f32 * LEVEL_HEIGHT
}

pub fn cell_to_world(cell: IVec2, height: i32) -> Vec3 {
    Vec3::new(cell.x as f32, level_y(height), cell.y as f32)
}

/// The four hop directions. `y` of the offset is the grid's z axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    North,
    South,
    East,
    West,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::North, Dir::South, Dir::East, Dir::West];

    pub fn offset(self) -> IVec2 {
        match self {
            Dir::North => IVec2::new(0, -1),
            Dir::South => IVec2::new(0, 1),
            Dir::East => IVec2::new(1, 0),
            Dir::West => IVec2::new(-1, 0),
        }
    }

    /// Yaw (radians) so that a model whose "forward" is -Z faces this direction.
    pub fn yaw(self) -> f32 {
        match self {
            Dir::North => 0.0,
            Dir::West => std::f32::consts::FRAC_PI_2,
            Dir::South => std::f32::consts::PI,
            Dir::East => -std::f32::consts::FRAC_PI_2,
        }
    }

    pub fn from_offset(offset: IVec2) -> Option<Dir> {
        Dir::ALL.into_iter().find(|d| d.offset() == offset)
    }

    /// The direction whose axis dominates `delta` (for "walk toward the click").
    pub fn toward(delta: IVec2) -> Option<Dir> {
        if delta == IVec2::ZERO {
            return None;
        }
        if delta.x.abs() >= delta.y.abs() {
            Some(if delta.x > 0 { Dir::East } else { Dir::West })
        } else {
            Some(if delta.y > 0 { Dir::South } else { Dir::North })
        }
    }
}

/// Logical grid position of an actor (cat, dog, toy, yarn ball).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPos(pub IVec2);

/// The direction an actor is facing.
#[derive(Component, Clone, Copy, Debug)]
pub struct Facing(pub Dir);

/// What happens if an actor standing on `from` hops toward `dir`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HopPlan {
    /// A normal hop onto the neighbouring tile.
    Step(IVec2),
    /// A long jump across a one-tile gap.
    Leap(IVec2),
    /// The neighbouring tile is too high to climb.
    TooHigh,
    /// Nothing to land on: the actor leaves the platforms.
    Edge,
}

impl Grid {
    /// The cat's movement rules. A neighbouring tile can be stepped onto when it is at
    /// most one level higher. When there is no neighbouring tile, the cat leaps over the
    /// gap if the tile beyond it is at the same height or lower.
    pub fn plan_hop(&self, from: IVec2, dir: Dir) -> HopPlan {
        let here_h = self.height(from).unwrap_or(0);
        let next = from + dir.offset();
        match self.height(next) {
            Some(h) if h > here_h + 1 => HopPlan::TooHigh,
            Some(_) => HopPlan::Step(next),
            None => {
                let far = from + dir.offset() * 2;
                match self.height(far) {
                    Some(h) if h <= here_h => HopPlan::Leap(far),
                    _ => HopPlan::Edge,
                }
            }
        }
    }
}
