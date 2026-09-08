//! Procedural level generation.
//!
//! A level is a chain of floating cat-tree islands. Consecutive islands are
//! joined either by a wooden plank bridge or by a one-tile gap the cat can
//! long-jump across. Later levels add more islands, height changes, dogs,
//! toys, and decoy yarn balls.

use crate::grid::{Axis, Dir, Grid, HopPlan, Tile, TileKind};
use crate::rng::Rng;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub const LEVEL_COUNT: u32 = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YarnColor {
    Purple,
    Red,
    Blue,
    Green,
    Yellow,
    Orange,
}

impl YarnColor {
    pub const DECOYS: [YarnColor; 5] = [
        YarnColor::Red,
        YarnColor::Blue,
        YarnColor::Green,
        YarnColor::Yellow,
        YarnColor::Orange,
    ];

    pub fn name(self) -> &'static str {
        match self {
            YarnColor::Purple => "PURPLE",
            YarnColor::Red => "RED",
            YarnColor::Blue => "BLUE",
            YarnColor::Green => "GREEN",
            YarnColor::Yellow => "YELLOW",
            YarnColor::Orange => "ORANGE",
        }
    }

    pub fn color(self) -> Color {
        match self {
            YarnColor::Purple => Color::srgb_u8(150, 60, 220),
            YarnColor::Red => Color::srgb_u8(230, 60, 60),
            YarnColor::Blue => Color::srgb_u8(60, 120, 235),
            YarnColor::Green => Color::srgb_u8(70, 190, 90),
            YarnColor::Yellow => Color::srgb_u8(245, 220, 60),
            YarnColor::Orange => Color::srgb_u8(245, 140, 40),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToyKind {
    Mouse,
    Fish,
    Feather,
}

#[derive(Clone, Debug)]
pub struct IslandInfo {
    pub cells: Vec<IVec2>,
    pub height: i32,
    pub center: Vec2,
    pub depth: usize,
}

/// Per-level knobs that scale the challenge.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Difficulty {
    pub dog_patrol_interval: f32,
    pub dog_chase_interval: f32,
    pub dog_chase_range: i32,
}

impl Default for Difficulty {
    fn default() -> Self {
        Difficulty {
            dog_patrol_interval: 1.0,
            dog_chase_interval: 0.7,
            dog_chase_range: 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LevelData {
    pub grid: Grid,
    pub islands: Vec<IslandInfo>,
    pub toys: Vec<(IVec2, ToyKind)>,
    pub dogs: Vec<(IVec2, usize)>,
    pub yarn: Vec<(IVec2, YarnColor)>,
    pub difficulty: Difficulty,
}

struct Params {
    islands: i32,
    max_height: i32,
    gap_chance: f32,
    dogs: i32,
    toys: i32,
    decoys: i32,
    branch_chance: f32,
    island_min: i32,
    island_max: i32,
}

fn params(level: u32) -> Params {
    let n = level as i32;
    Params {
        islands: (2 + n / 3).min(8),
        max_height: if n < 4 {
            0
        } else if n < 10 {
            1
        } else {
            2
        },
        gap_chance: if n < 5 { 0.0 } else { (0.25 + n as f32 * 0.02).min(0.55) },
        dogs: if n < 3 { 0 } else { (1 + (n - 3) / 4).min(5) },
        toys: (1 + n / 2).min(11),
        decoys: (1 + n / 6).min(4),
        branch_chance: if n < 8 { 0.0 } else { 0.35 },
        island_min: 3,
        island_max: if n < 6 { 4 } else { 5 },
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x0: i32,
    z0: i32,
    w: i32,
    d: i32,
}

impl Rect {
    fn x1(&self) -> i32 {
        self.x0 + self.w - 1
    }
    fn z1(&self) -> i32 {
        self.z0 + self.d - 1
    }
    fn cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        (self.x0..=self.x1()).flat_map(move |x| (self.z0..=self.z1()).map(move |z| IVec2::new(x, z)))
    }
    fn center(&self) -> Vec2 {
        Vec2::new(
            (self.x0 + self.x1()) as f32 * 0.5,
            (self.z0 + self.z1()) as f32 * 0.5,
        )
    }
}

pub fn generate(level: u32) -> LevelData {
    let p = params(level);
    let mut rng = Rng::new(0xCA7 + level as u64 * 7919);

    let mut tiles: HashMap<IVec2, Tile> = HashMap::new();
    let mut rects: Vec<Rect> = Vec::new();
    let mut islands: Vec<IslandInfo> = Vec::new();

    // First island at the origin.
    let first = Rect {
        x0: -1,
        z0: -1,
        w: rng.range_i32(p.island_min, p.island_max),
        d: rng.range_i32(p.island_min, p.island_max),
    };
    commit_island(&mut tiles, &mut rects, &mut islands, first, 0, 0);

    let mut attempts = 0;
    while (islands.len() as i32) < p.islands && attempts < 400 {
        attempts += 1;
        // Usually extend the newest island; sometimes branch off an older one.
        let from = if islands.len() > 1 && rng.chance(p.branch_chance) {
            rng.range_i32(0, islands.len() as i32 - 2) as usize
        } else {
            islands.len() - 1
        };
        let prev = rects[from];
        let prev_h = islands[from].height;

        // Favour heading east / north so levels read as a journey.
        let dir = *rng.pick(&[Dir::East, Dir::East, Dir::North, Dir::North, Dir::South, Dir::West]);
        let use_gap = p.gap_chance > 0.0 && rng.chance(p.gap_chance);
        let conn_len = if use_gap { 1 } else { rng.range_i32(1, 3) };
        let height = if use_gap {
            prev_h
        } else {
            (prev_h + rng.range_i32(-1, 1)).clamp(0, p.max_height)
        };

        let w = rng.range_i32(p.island_min, p.island_max);
        let d = rng.range_i32(p.island_min, p.island_max);

        // Choose the lane the connector runs along, then place the new island so it
        // includes that lane.
        let (rect, lane_cells, edge_prev, edge_new) = match dir {
            Dir::East => {
                let lane = rng.range_i32(prev.z0, prev.z1());
                let x0 = prev.x1() + 1 + conn_len;
                let z0 = lane - rng.range_i32(0, d - 1);
                let rect = Rect { x0, z0, w, d };
                let lane_cells: Vec<IVec2> =
                    (prev.x1() + 1..x0).map(|x| IVec2::new(x, lane)).collect();
                (rect, lane_cells, IVec2::new(prev.x1(), lane), IVec2::new(x0, lane))
            }
            Dir::West => {
                let lane = rng.range_i32(prev.z0, prev.z1());
                let x1 = prev.x0 - 1 - conn_len;
                let x0 = x1 - w + 1;
                let z0 = lane - rng.range_i32(0, d - 1);
                let rect = Rect { x0, z0, w, d };
                let lane_cells: Vec<IVec2> =
                    (x1 + 1..prev.x0).map(|x| IVec2::new(x, lane)).collect();
                (rect, lane_cells, IVec2::new(prev.x0, lane), IVec2::new(x1, lane))
            }
            Dir::North => {
                let lane = rng.range_i32(prev.x0, prev.x1());
                let z1 = prev.z0 - 1 - conn_len;
                let z0 = z1 - d + 1;
                let x0 = lane - rng.range_i32(0, w - 1);
                let rect = Rect { x0, z0, w, d };
                let lane_cells: Vec<IVec2> =
                    (z1 + 1..prev.z0).map(|z| IVec2::new(lane, z)).collect();
                (rect, lane_cells, IVec2::new(lane, prev.z0), IVec2::new(lane, z1))
            }
            Dir::South => {
                let lane = rng.range_i32(prev.x0, prev.x1());
                let z0 = prev.z1() + 1 + conn_len;
                let x0 = lane - rng.range_i32(0, w - 1);
                let rect = Rect { x0, z0, w, d };
                let lane_cells: Vec<IVec2> =
                    (prev.z1() + 1..z0).map(|z| IVec2::new(lane, z)).collect();
                (rect, lane_cells, IVec2::new(lane, prev.z1()), IVec2::new(lane, z0))
            }
        };

        // Reject if the island or its connector touches anything (with a 1-tile margin).
        let margin = Rect {
            x0: rect.x0 - 1,
            z0: rect.z0 - 1,
            w: rect.w + 2,
            d: rect.d + 2,
        };
        let clash = margin.cells().any(|c| tiles.contains_key(&c))
            || lane_cells.iter().any(|c| {
                tiles.contains_key(c)
                    || Dir::ALL.iter().any(|d| {
                        let n = *c + d.offset();
                        n != edge_prev && tiles.contains_key(&n)
                    })
            });
        if clash {
            continue;
        }

        let depth = islands[from].depth + 1;
        commit_island(&mut tiles, &mut rects, &mut islands, rect, height, depth);

        if use_gap {
            for c in [edge_prev, edge_new] {
                if let Some(t) = tiles.get_mut(&c) {
                    t.keep_clear = true;
                }
            }
        } else {
            let axis = match dir {
                Dir::East | Dir::West => Axis::X,
                Dir::North | Dir::South => Axis::Z,
            };
            for c in lane_cells {
                tiles.insert(
                    c,
                    Tile {
                        height: prev_h,
                        kind: TileKind::Bridge(axis),
                        keep_clear: false,
                    },
                );
            }
        }
    }

    let max_height = islands.iter().map(|i| i.height).max().unwrap_or(0);

    // Start in the middle of the first island.
    let start = {
        let r = rects[0];
        IVec2::new(r.x0 + r.w / 2, r.z0 + r.d / 2)
    };

    let mut occupied: Vec<IVec2> = vec![start];
    let free = |c: IVec2, occupied: &[IVec2]| !occupied.contains(&c);

    // The purple yarn goes on the deepest island, as far from the start as possible.
    let goal_island = (0..islands.len())
        .max_by_key(|&i| (islands[i].depth, (islands[i].center - Vec2::ZERO).length() as i32))
        .unwrap_or(0);
    let mut yarn = Vec::new();
    {
        let cells = &islands[goal_island].cells;
        let far = cells
            .iter()
            .copied()
            .filter(|c| free(*c, &occupied))
            .max_by_key(|c| (c.x - start.x).abs() + (c.y - start.y).abs())
            .unwrap_or(cells[0]);
        yarn.push((far, YarnColor::Purple));
        occupied.push(far);
    }

    // Decoys go on other islands (or on the first island for tiny levels).
    let mut decoy_colors = YarnColor::DECOYS.to_vec();
    rng.shuffle(&mut decoy_colors);
    let mut decoy_islands: Vec<usize> = (0..islands.len()).filter(|&i| i != goal_island).collect();
    if decoy_islands.is_empty() {
        decoy_islands.push(goal_island);
    }
    rng.shuffle(&mut decoy_islands);
    let purple = yarn[0].0;
    let mut placed = 0;
    let mut tries = 0;
    while placed < p.decoys && tries < 60 {
        tries += 1;
        let island = decoy_islands[(placed + tries) as usize % decoy_islands.len()];
        let cell = *rng.pick(&islands[island].cells);
        let near_start = (cell - start).abs().max_element() <= 1;
        if !free(cell, &occupied) || near_start || tiles[&cell].keep_clear {
            continue;
        }
        // Wrong-colour yarn is a wall the cat must walk around, so make sure the purple
        // ball is still reachable with this decoy in place.
        let grid = Grid {
            tiles: tiles.clone(),
            start,
            max_height: 0,
        };
        let mut blocked: Vec<IVec2> = yarn.iter().skip(1).map(|(c, _)| *c).collect();
        blocked.push(cell);
        if !reachable(&grid, start, &blocked).contains(&purple) {
            continue;
        }
        yarn.push((cell, decoy_colors[placed as usize % decoy_colors.len()]));
        occupied.push(cell);
        placed += 1;
    }

    // Toys: anywhere not reserved, including bridges (they block the path until swiped).
    let mut all_cells: Vec<IVec2> = tiles.keys().copied().collect();
    all_cells.sort_by_key(|c| (c.y, c.x));
    let mut toys = Vec::new();
    let mut tries = 0;
    while (toys.len() as i32) < p.toys && tries < 200 {
        tries += 1;
        let cell = *rng.pick(&all_cells);
        let tile = tiles[&cell];
        let near_start = (cell - start).abs().max_element() <= 1;
        if !free(cell, &occupied) || near_start || tile.keep_clear {
            continue;
        }
        let kind = *rng.pick(&[ToyKind::Mouse, ToyKind::Fish, ToyKind::Feather]);
        toys.push((cell, kind));
        occupied.push(cell);
    }

    // Dogs: on islands other than the first, not on bridges, away from the start.
    let mut dogs = Vec::new();
    let mut tries = 0;
    while (dogs.len() as i32) < p.dogs && tries < 200 {
        tries += 1;
        let island = rng.range_i32(0, islands.len() as i32 - 1) as usize;
        if island == 0 && islands.len() > 1 {
            continue;
        }
        let cell = *rng.pick(&islands[island].cells);
        let near_start = (cell - start).abs().max_element() <= 2;
        if !free(cell, &occupied) || near_start || tiles[&cell].keep_clear {
            continue;
        }
        dogs.push((cell, island));
        occupied.push(cell);
    }

    let n = level as f32;
    let difficulty = Difficulty {
        dog_patrol_interval: (1.1 - n * 0.02).max(0.7),
        dog_chase_interval: (0.75 - n * 0.015).max(0.45),
        dog_chase_range: if level < 8 { 2 } else { 3 },
    };

    LevelData {
        grid: Grid {
            tiles,
            start,
            max_height,
        },
        islands,
        toys,
        dogs,
        yarn,
        difficulty,
    }
}

/// Every cell the cat can reach from `start` using its hop rules, treating
/// `blocked` cells as walls.
pub fn reachable(grid: &Grid, start: IVec2, blocked: &[IVec2]) -> HashSet<IVec2> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    seen.insert(start);
    while let Some(c) = queue.pop_front() {
        for d in Dir::ALL {
            let target = match grid.plan_hop(c, d) {
                HopPlan::Step(t) | HopPlan::Leap(t) => t,
                HopPlan::TooHigh | HopPlan::Edge => continue,
            };
            if blocked.contains(&target) {
                continue;
            }
            if seen.insert(target) {
                queue.push_back(target);
            }
        }
    }
    seen
}

fn commit_island(
    tiles: &mut HashMap<IVec2, Tile>,
    rects: &mut Vec<Rect>,
    islands: &mut Vec<IslandInfo>,
    rect: Rect,
    height: i32,
    depth: usize,
) {
    let idx = islands.len();
    let cells: Vec<IVec2> = rect.cells().collect();
    for &c in &cells {
        tiles.insert(
            c,
            Tile {
                height,
                kind: TileKind::Island(idx),
                keep_clear: false,
            },
        );
    }
    rects.push(rect);
    islands.push(IslandInfo {
        cells,
        height,
        center: rect.center(),
        depth,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every level must be solvable: the purple yarn is reachable from the start using
    /// only legal cat moves, without stepping on any wrong-coloured yarn.
    #[test]
    fn every_level_is_solvable() {
        for level in 1..=LEVEL_COUNT {
            let data = generate(level);
            let purple = data.yarn.iter().find(|(_, c)| *c == YarnColor::Purple).unwrap().0;
            let decoys: Vec<IVec2> = data
                .yarn
                .iter()
                .filter(|(_, c)| *c != YarnColor::Purple)
                .map(|(c, _)| *c)
                .collect();
            let seen = reachable(&data.grid, data.grid.start, &decoys);
            assert!(seen.contains(&purple), "level {level} purple yarn unreachable:\n{}", ascii_map(&data));
            assert!(data.islands.len() >= 2, "level {level} too small");
            assert!(!decoys.is_empty(), "level {level} has no decoy yarn");
        }
    }

    /// Levels must be identical every time they are generated.
    #[test]
    fn generation_is_deterministic() {
        for level in 1..=LEVEL_COUNT {
            assert_eq!(ascii_map(&generate(level)), ascii_map(&generate(level)), "level {level}");
        }
    }

    /// Nothing sits on a tile the cat needs to land on after a long jump.
    #[test]
    fn gap_edges_stay_clear() {
        for level in 1..=LEVEL_COUNT {
            let data = generate(level);
            let obstacles = data.toys.iter().map(|(c, _)| *c).chain(data.dogs.iter().map(|(c, _)| *c));
            for cell in obstacles {
                assert!(!data.grid.tile(cell).unwrap().keep_clear, "level {level} obstacle at gap edge {cell}");
            }
        }
    }
}

/// An ASCII picture of a level, for debugging (`--map N`).
///
/// `S` start, `P` purple yarn, other yarn by first letter (lower case), `t` toy,
/// `D` dog, `=`/`|` planks, digits are island heights.
pub fn ascii_map(data: &LevelData) -> String {
    let grid = &data.grid;
    let (min, max) = grid
        .tiles
        .keys()
        .fold((IVec2::MAX, IVec2::MIN), |(lo, hi), c| (lo.min(*c), hi.max(*c)));
    let mut out = String::new();
    for z in min.y..=max.y {
        for x in min.x..=max.x {
            let cell = IVec2::new(x, z);
            let ch = if cell == grid.start {
                'S'
            } else if let Some((_, color)) = data.yarn.iter().find(|(c, _)| *c == cell) {
                match color {
                    YarnColor::Purple => 'P',
                    other => other.name().chars().next().unwrap().to_ascii_lowercase(),
                }
            } else if data.toys.iter().any(|(c, _)| *c == cell) {
                't'
            } else if data.dogs.iter().any(|(c, _)| *c == cell) {
                'D'
            } else {
                match grid.tile(cell) {
                    None => ' ',
                    Some(t) => match t.kind {
                        TileKind::Bridge(Axis::X) => '=',
                        TileKind::Bridge(Axis::Z) => '|',
                        TileKind::Island(_) => char::from_digit(t.height as u32, 10).unwrap_or('#'),
                    },
                }
            };
            out.push(ch);
        }
        out.push('\n');
    }
    out
}
