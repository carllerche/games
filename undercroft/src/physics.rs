//! Axis-aligned movement against the tile grid.

use crate::{art::TILE, dungeon::Dungeon};
use bevy::prelude::*;

pub fn tile_of(pos: Vec2) -> IVec2 {
    (pos / TILE).floor().as_ivec2()
}

pub fn tile_center(t: IVec2) -> Vec2 {
    t.as_vec2() * TILE + Vec2::splat(TILE / 2.0)
}

fn overlaps_solid(d: &Dungeon, min: Vec2, max: Vec2) -> bool {
    let lo = tile_of(min);
    let hi = tile_of(max - Vec2::splat(0.001));
    for y in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            if d.solid(IVec2::new(x, y)) {
                return true;
            }
        }
    }
    false
}

/// Moves a box of half-size `half` centred at `pos` by `delta`, sliding along
/// walls. Returns the new position and whether each axis was blocked.
pub fn move_box(d: &Dungeon, pos: Vec2, delta: Vec2, half: Vec2) -> (Vec2, BVec2) {
    let mut p = pos;
    let mut blocked = BVec2::FALSE;
    let eps = 0.01;

    if delta.x != 0.0 {
        let nx = p.x + delta.x;
        if overlaps_solid(d, Vec2::new(nx - half.x, p.y - half.y), Vec2::new(nx + half.x, p.y + half.y)) {
            blocked.x = true;
            if delta.x > 0.0 {
                let edge = ((nx + half.x) / TILE).floor() * TILE;
                p.x = edge - half.x - eps;
            } else {
                let edge = ((nx - half.x) / TILE).floor() * TILE + TILE;
                p.x = edge + half.x + eps;
            }
        } else {
            p.x = nx;
        }
    }
    if delta.y != 0.0 {
        let ny = p.y + delta.y;
        if overlaps_solid(d, Vec2::new(p.x - half.x, ny - half.y), Vec2::new(p.x + half.x, ny + half.y)) {
            blocked.y = true;
            if delta.y > 0.0 {
                let edge = ((ny + half.y) / TILE).floor() * TILE;
                p.y = edge - half.y - eps;
            } else {
                let edge = ((ny - half.y) / TILE).floor() * TILE + TILE;
                p.y = edge + half.y + eps;
            }
        } else {
            p.y = ny;
        }
    }
    (p, blocked)
}

/// Bresenham-style line of sight test between two world positions.
pub fn line_of_sight(d: &Dungeon, from: Vec2, to: Vec2) -> bool {
    let steps = ((to - from).length() / (TILE * 0.5)).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let p = from.lerp(to, i as f32 / steps as f32);
        if d.solid(tile_of(p)) {
            return false;
        }
    }
    true
}

/// Breadth-first distance field from `origin` over walkable tiles, used by
/// monsters to chase the player around corners. `u16::MAX` means unreachable.
pub fn flow_field(d: &Dungeon, origin: IVec2, limit: u16) -> Vec<u16> {
    let mut dist = vec![u16::MAX; (d.w * d.h) as usize];
    if !d.in_bounds(origin) {
        return dist;
    }
    let idx = |p: IVec2| (p.y * d.w + p.x) as usize;
    let mut queue = std::collections::VecDeque::from([origin]);
    dist[idx(origin)] = 0;
    while let Some(p) = queue.pop_front() {
        let here = dist[idx(p)];
        if here >= limit {
            continue;
        }
        for delta in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
            let q = p + delta;
            if d.in_bounds(q) && !d.solid(q) && dist[idx(q)] == u16::MAX {
                dist[idx(q)] = here + 1;
                queue.push_back(q);
            }
        }
    }
    dist
}

/// Direction (unit vector, or zero) an entity at `pos` should walk to descend
/// the distance field, without cutting wall corners.
pub fn flow_direction(d: &Dungeon, field: &[u16], pos: Vec2) -> Vec2 {
    let t = tile_of(pos);
    if !d.in_bounds(t) {
        return Vec2::ZERO;
    }
    let idx = |p: IVec2| (p.y * d.w + p.x) as usize;
    let here = field[idx(t)];
    if here == u16::MAX || here == 0 {
        return Vec2::ZERO;
    }
    let mut best: Option<(u16, IVec2)> = None;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let q = t + IVec2::new(dx, dy);
            if !d.in_bounds(q) || d.solid(q) {
                continue;
            }
            // Diagonals need both orthogonal neighbours open.
            if dx != 0 && dy != 0 && (d.solid(t + IVec2::new(dx, 0)) || d.solid(t + IVec2::new(0, dy))) {
                continue;
            }
            let v = field[idx(q)];
            if v < here && best.is_none_or(|(b, _)| v < b) {
                best = Some((v, q));
            }
        }
    }
    match best {
        Some((_, q)) => (tile_center(q) - pos).normalize_or_zero(),
        None => Vec2::ZERO,
    }
}
