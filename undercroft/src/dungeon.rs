//! Procedural floor generation: rooms joined by corridors, a locked stairway,
//! shops, a treasure room, a secret room behind a cracked wall, traps,
//! torches, loot and monsters. Pure data; nothing here touches the ECS.

use bevy::math::IVec2;
use rand::{Rng, SeedableRng, rngs::StdRng, seq::IndexedRandom};

pub const MAX_FLOOR: u32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Wall,
    Floor,
    Carpet,
    Stairs,
    Cracked,
    Spikes,
    Rubble,
}

impl Tile {
    pub fn solid(self) -> bool {
        matches!(self, Tile::Wall | Tile::Cracked)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoomKind {
    Start,
    Normal,
    Shop,
    Treasure,
    Exit,
    Boss,
    Secret,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub kind: RoomKind,
}

impl Room {
    pub fn center(&self) -> IVec2 {
        IVec2::new(self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn contains(&self, p: IVec2) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }

    fn overlaps(&self, other: &Room, margin: i32) -> bool {
        self.x - margin < other.x + other.w
            && self.x + self.w + margin > other.x
            && self.y - margin < other.y + other.h
            && self.y + self.h + margin > other.y
    }

    pub fn area(&self) -> i32 {
        self.w * self.h
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MonsterKind {
    Slime,
    Bat,
    Skeleton,
    Archer,
    Spider,
    Ghost,
    Knight,
    Ogre,
    Lich,
    Rat,
    Kobold,
    Imp,
    Cultist,
    Golem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChestLoot {
    Key,
    Coins(u32),
    Potion,
    Arrows(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spawn {
    Monster(MonsterKind),
    Coin,
    Potion,
    ArrowBundle,
    Chest(ChestLoot),
    Shopkeeper,
    Torch,
}

#[derive(Clone, Debug)]
pub struct Dungeon {
    pub floor: u32,
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    pub rooms: Vec<Room>,
    pub spawns: Vec<(IVec2, Spawn)>,
    pub start: IVec2,
    pub exit: IVec2,
    /// Doorway tiles of the boss room, sealed while the boss fight is on.
    pub boss_doors: Vec<IVec2>,
}

impl Dungeon {
    pub fn in_bounds(&self, p: IVec2) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h
    }

    pub fn get(&self, p: IVec2) -> Tile {
        if self.in_bounds(p) {
            self.tiles[(p.y * self.w + p.x) as usize]
        } else {
            Tile::Wall
        }
    }

    pub fn set(&mut self, p: IVec2, tile: Tile) {
        if self.in_bounds(p) {
            self.tiles[(p.y * self.w + p.x) as usize] = tile;
        }
    }

    pub fn solid(&self, p: IVec2) -> bool {
        self.get(p).solid()
    }

    pub fn room_at(&self, p: IVec2) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(p))
    }

    pub fn room_of_kind(&self, kind: RoomKind) -> Option<&Room> {
        self.rooms.iter().find(|r| r.kind == kind)
    }

    fn carve(&mut self, room: &Room, tile: Tile) {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                self.set(IVec2::new(x, y), tile);
            }
        }
    }

    fn corridor(&mut self, a: IVec2, b: IVec2, rng: &mut StdRng) {
        let carve_line = |d: &mut Dungeon, from: IVec2, to: IVec2| {
            let mut p = from;
            loop {
                if d.get(p) == Tile::Wall {
                    d.set(p, Tile::Floor);
                }
                if p == to {
                    break;
                }
                p += (to - p).signum();
            }
        };
        let corner = if rng.random_bool(0.5) {
            IVec2::new(b.x, a.y)
        } else {
            IVec2::new(a.x, b.y)
        };
        carve_line(self, a, corner);
        carve_line(self, corner, b);
    }

    fn floor_tiles_in(&self, room: &Room) -> Vec<IVec2> {
        let mut out = Vec::new();
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                let p = IVec2::new(x, y);
                if matches!(self.get(p), Tile::Floor | Tile::Carpet) {
                    out.push(p);
                }
            }
        }
        out
    }

    fn occupied(&self, p: IVec2) -> bool {
        self.spawns.iter().any(|(q, _)| *q == p)
    }

    /// A random free floor tile inside `room`, at least `keep_clear` tiles
    /// away from `avoid`.
    fn free_tile(
        &self,
        room: &Room,
        avoid: &[IVec2],
        keep_clear: i32,
        rng: &mut StdRng,
    ) -> Option<IVec2> {
        let candidates: Vec<IVec2> = self
            .floor_tiles_in(room)
            .into_iter()
            .filter(|p| !self.occupied(*p))
            .filter(|p| {
                avoid
                    .iter()
                    .all(|a| (*a - *p).abs().max_element() > keep_clear)
            })
            .collect();
        candidates.choose(rng).copied()
    }
}

fn monster_pool(floor: u32) -> &'static [MonsterKind] {
    use MonsterKind::*;
    match floor {
        1 => &[Rat, Rat, Slime, Slime, Bat],
        2 => &[Rat, Slime, Bat, Kobold, Kobold, Skeleton],
        3 => &[Slime, Skeleton, Bat, Spider, Kobold],
        4 => &[Skeleton, Archer, Spider, Kobold, Imp, Bat],
        5 => &[Skeleton, Archer, Spider, Ghost, Imp, Cultist],
        6 => &[Knight, Ghost, Archer, Spider, Imp, Cultist],
        7 => &[Knight, Ghost, Archer, Skeleton, Cultist, Golem, Imp],
        _ => &[Knight, Ghost, Archer, Cultist, Golem],
    }
}

pub fn generate(floor: u32, seed: u64) -> Dungeon {
    let mut rng = StdRng::seed_from_u64(seed ^ (floor as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let w = (50 + floor as i32 * 3).min(76);
    let h = (38 + floor as i32 * 2).min(56);
    let mut d = Dungeon {
        floor,
        w,
        h,
        tiles: vec![Tile::Wall; (w * h) as usize],
        rooms: Vec::new(),
        spawns: Vec::new(),
        start: IVec2::ZERO,
        exit: IVec2::ZERO,
        boss_doors: Vec::new(),
    };

    place_rooms(&mut d, &mut rng);
    connect_rooms(&mut d, &mut rng);
    assign_roles(&mut d, &mut rng);
    place_secret_room(&mut d, &mut rng);
    place_traps(&mut d, &mut rng);
    place_torches(&mut d, &mut rng);
    place_loot(&mut d, &mut rng);
    place_monsters(&mut d, &mut rng);
    d
}

fn place_rooms(d: &mut Dungeon, rng: &mut StdRng) {
    let target = (9 + d.floor as usize).min(16);
    let mut attempts = 0;
    // The last floor gets a big arena for the boss fight.
    if d.floor == MAX_FLOOR {
        loop {
            let room = Room {
                x: rng.random_range(2..d.w - 17),
                y: rng.random_range(2..d.h - 14),
                w: 15,
                h: 12,
                kind: RoomKind::Boss,
            };
            attempts += 1;
            if attempts > 50 || d.rooms.iter().all(|r| !r.overlaps(&room, 3)) {
                d.carve(&room, Tile::Floor);
                d.rooms.push(room);
                break;
            }
        }
    }
    while d.rooms.len() < target && attempts < 400 {
        attempts += 1;
        let room = Room {
            w: rng.random_range(5..=11),
            h: rng.random_range(4..=9),
            x: 0,
            y: 0,
            kind: RoomKind::Normal,
        };
        let room = Room {
            x: rng.random_range(1..d.w - room.w - 1),
            y: rng.random_range(1..d.h - room.h - 1),
            ..room
        };
        if d.rooms.iter().all(|r| !r.overlaps(&room, 2)) {
            d.carve(&room, Tile::Floor);
            d.rooms.push(room);
        }
    }
}

fn connect_rooms(d: &mut Dungeon, rng: &mut StdRng) {
    // Minimum-spanning-tree style: each room links to the nearest already
    // connected room, then a few extra links create loops.
    let n = d.rooms.len();
    let mut connected = vec![false; n];
    connected[0] = true;
    for _ in 1..n {
        let mut best: Option<(i32, usize, usize)> = None;
        for i in 0..n {
            if connected[i] {
                continue;
            }
            for j in 0..n {
                if !connected[j] {
                    continue;
                }
                let dist = (d.rooms[i].center() - d.rooms[j].center())
                    .abs()
                    .element_sum();
                if best.is_none_or(|(b, _, _)| dist < b) {
                    best = Some((dist, i, j));
                }
            }
        }
        let (_, i, j) = best.unwrap();
        connected[i] = true;
        let (a, b) = (d.rooms[i].center(), d.rooms[j].center());
        d.corridor(a, b, rng);
    }
    let extra = 1 + d.floor as usize / 3;
    for _ in 0..extra {
        let i = rng.random_range(0..n);
        let j = rng.random_range(0..n);
        if i != j {
            let (a, b) = (d.rooms[i].center(), d.rooms[j].center());
            d.corridor(a, b, rng);
        }
    }
}

fn assign_roles(d: &mut Dungeon, rng: &mut StdRng) {
    let n = d.rooms.len();
    // Start in a smallish room; on the boss floor the arena was placed first,
    // so the start must be another room.
    let start = if d.floor == MAX_FLOOR { 1 } else { 0 };
    d.rooms[start].kind = RoomKind::Start;
    d.start = d.rooms[start].center();

    let exit = if d.floor == MAX_FLOOR {
        0
    } else {
        let sc = d.rooms[start].center();
        (0..n)
            .filter(|&i| i != start)
            .max_by_key(|&i| (d.rooms[i].center() - sc).abs().element_sum())
            .unwrap()
    };
    if d.floor != MAX_FLOOR {
        d.rooms[exit].kind = RoomKind::Exit;
    }
    d.exit = d.rooms[exit].center();
    d.set(d.exit, Tile::Stairs);

    if d.floor == MAX_FLOOR {
        // Record the arena doorways so they can be sealed during the fight.
        let room = d.rooms[exit].clone();
        for x in room.x - 1..=room.x + room.w {
            for y in [room.y - 1, room.y + room.h] {
                if d.get(IVec2::new(x, y)) == Tile::Floor {
                    d.boss_doors.push(IVec2::new(x, y));
                }
            }
        }
        for y in room.y - 1..=room.y + room.h {
            for x in [room.x - 1, room.x + room.w] {
                if d.get(IVec2::new(x, y)) == Tile::Floor {
                    d.boss_doors.push(IVec2::new(x, y));
                }
            }
        }
    }

    let mut others: Vec<usize> = (0..n).filter(|&i| i != start && i != exit).collect();
    others.sort_by_key(|&i| d.rooms[i].area());

    let shops = if d.floor >= 3 && rng.random_bool(0.6) {
        2
    } else {
        1
    };
    for _ in 0..shops {
        if others.is_empty() {
            break;
        }
        let pick = rng.random_range(0..others.len().min(6));
        let i = others.remove(pick);
        d.rooms[i].kind = RoomKind::Shop;
        let room = d.rooms[i].clone();
        d.carve(&room, Tile::Carpet);
        d.spawns.push((room.center(), Spawn::Shopkeeper));
    }
    if let Some(i) = others.pop() {
        d.rooms[i].kind = RoomKind::Treasure;
    }
}

fn place_secret_room(d: &mut Dungeon, rng: &mut StdRng) {
    for _ in 0..40 {
        let host = rng.random_range(0..d.rooms.len());
        let host = d.rooms[host].clone();
        if matches!(host.kind, RoomKind::Shop | RoomKind::Boss) {
            continue;
        }
        let (sw, sh) = (rng.random_range(3..=4), rng.random_range(3..=4));
        let side = rng.random_range(0..4);
        let (room, door) = match side {
            0 => {
                let y = rng.random_range(host.y..host.y + host.h - sh + 1);
                let x = host.x + host.w + 1;
                (
                    Room {
                        x,
                        y,
                        w: sw,
                        h: sh,
                        kind: RoomKind::Secret,
                    },
                    IVec2::new(host.x + host.w, y + sh / 2),
                )
            }
            1 => {
                let y = rng.random_range(host.y..host.y + host.h - sh + 1);
                let x = host.x - sw - 1;
                (
                    Room {
                        x,
                        y,
                        w: sw,
                        h: sh,
                        kind: RoomKind::Secret,
                    },
                    IVec2::new(host.x - 1, y + sh / 2),
                )
            }
            2 => {
                let x = rng.random_range(host.x..host.x + host.w - sw + 1);
                let y = host.y + host.h + 1;
                (
                    Room {
                        x,
                        y,
                        w: sw,
                        h: sh,
                        kind: RoomKind::Secret,
                    },
                    IVec2::new(x + sw / 2, host.y + host.h),
                )
            }
            _ => {
                let x = rng.random_range(host.x..host.x + host.w - sw + 1);
                let y = host.y - sh - 1;
                (
                    Room {
                        x,
                        y,
                        w: sw,
                        h: sh,
                        kind: RoomKind::Secret,
                    },
                    IVec2::new(x + sw / 2, host.y - 1),
                )
            }
        };
        if room.x < 1 || room.y < 1 || room.x + room.w >= d.w - 1 || room.y + room.h >= d.h - 1 {
            continue;
        }
        let mut clear = true;
        for y in room.y - 1..=room.y + room.h {
            for x in room.x - 1..=room.x + room.w {
                if d.get(IVec2::new(x, y)) != Tile::Wall {
                    clear = false;
                }
            }
        }
        if !clear || d.get(door) != Tile::Wall {
            continue;
        }
        // The doorway must open onto the host room's floor.
        let inward = door + (host.center() - door).signum();
        if !host.contains(inward) && d.get(inward) != Tile::Floor {
            continue;
        }
        d.carve(&room, Tile::Floor);
        d.set(door, Tile::Cracked);
        let c = room.center();
        d.spawns
            .push((c, Spawn::Chest(ChestLoot::Coins(30 + d.floor * 12))));
        if let Some(p) = d.free_tile(&room, &[c], 0, rng) {
            d.spawns.push((p, Spawn::Chest(ChestLoot::Potion)));
        }
        d.rooms.push(room);
        return;
    }
}

fn place_traps(d: &mut Dungeon, rng: &mut StdRng) {
    if d.floor < 2 {
        return;
    }
    for y in 1..d.h - 1 {
        for x in 1..d.w - 1 {
            let p = IVec2::new(x, y);
            if d.get(p) != Tile::Floor {
                continue;
            }
            let room = d.room_at(p).map(|i| d.rooms[i].kind);
            let chance = match room {
                None => 0.05,
                Some(RoomKind::Normal | RoomKind::Treasure) => 0.02,
                _ => 0.0,
            };
            if rng.random_bool(chance)
                && (p - d.start).abs().max_element() > 3
                && (p - d.exit).abs().max_element() > 1
            {
                d.set(p, Tile::Spikes);
            }
        }
    }
}

fn place_torches(d: &mut Dungeon, rng: &mut StdRng) {
    for room in d.rooms.clone() {
        if room.kind == RoomKind::Secret {
            continue;
        }
        let y = room.y + room.h;
        let spacing = if room.kind == RoomKind::Shop { 2 } else { 4 };
        for x in (room.x..room.x + room.w).step_by(spacing) {
            let p = IVec2::new(x, y);
            if d.get(p) == Tile::Wall && rng.random_bool(0.7) {
                d.spawns.push((p, Spawn::Torch));
            }
        }
    }
}

fn place_loot(d: &mut Dungeon, rng: &mut StdRng) {
    let rooms = d.rooms.clone();
    let key_candidates: Vec<&Room> = rooms
        .iter()
        .filter(|r| matches!(r.kind, RoomKind::Normal | RoomKind::Treasure))
        .collect();
    // The stairway key sits in a chest somewhere off the main path.
    if let Some(room) = key_candidates.choose(rng)
        && let Some(p) = d.free_tile(room, &[room.center()], 0, rng)
    {
        d.spawns.push((p, Spawn::Chest(ChestLoot::Key)));
    } else {
        // Fallback: drop it near the start so the floor is always completable.
        let p = d.start + IVec2::new(1, 0);
        d.spawns.push((p, Spawn::Chest(ChestLoot::Key)));
    }

    for room in rooms.iter().filter(|r| r.kind == RoomKind::Treasure) {
        for loot in [
            ChestLoot::Coins(20 + d.floor * 8),
            ChestLoot::Arrows(8),
            ChestLoot::Potion,
        ] {
            if let Some(p) = d.free_tile(room, &[], 0, rng) {
                d.spawns.push((p, Spawn::Chest(loot)));
            }
        }
    }

    let coin_rooms: Vec<&Room> = rooms
        .iter()
        .filter(|r| {
            matches!(
                r.kind,
                RoomKind::Normal | RoomKind::Exit | RoomKind::Treasure
            )
        })
        .collect();
    let coins = 6 + d.floor * 2;
    for _ in 0..coins {
        if let Some(room) = coin_rooms.choose(rng)
            && let Some(p) = d.free_tile(room, &[d.exit], 1, rng)
        {
            d.spawns.push((p, Spawn::Coin));
        }
    }
    for _ in 0..rng.random_range(1..=2) {
        if let Some(room) = coin_rooms.choose(rng)
            && let Some(p) = d.free_tile(room, &[d.exit], 1, rng)
        {
            d.spawns.push((p, Spawn::Potion));
        }
    }
    for _ in 0..rng.random_range(1..=2) {
        if let Some(room) = coin_rooms.choose(rng)
            && let Some(p) = d.free_tile(room, &[d.exit], 1, rng)
        {
            d.spawns.push((p, Spawn::ArrowBundle));
        }
    }
}

fn place_monsters(d: &mut Dungeon, rng: &mut StdRng) {
    let pool = monster_pool(d.floor);
    let rooms = d.rooms.clone();
    for room in &rooms {
        let extra = match room.kind {
            RoomKind::Normal => 0,
            RoomKind::Treasure | RoomKind::Exit => 1,
            RoomKind::Boss => {
                if let Some(p) = d.free_tile(room, &[d.exit], 2, rng) {
                    d.spawns.push((p, Spawn::Monster(MonsterKind::Lich)));
                }
                1
            }
            _ => continue,
        };
        let max = 2 + d.floor as i32 / 2;
        let count = (room.area() / 14).clamp(1, max) + extra + rng.random_range(0..=1);
        let count = if room.kind == RoomKind::Boss {
            2
        } else {
            count
        };
        for _ in 0..count {
            let kind = *pool.choose(rng).unwrap();
            if let Some(p) = d.free_tile(room, &[d.exit], 1, rng) {
                d.spawns.push((p, Spawn::Monster(kind)));
            }
        }
        // Mini-boss guarding the stairs on floors 3 and 6.
        if room.kind == RoomKind::Exit
            && matches!(d.floor, 3 | 6)
            && let Some(p) = d.free_tile(room, &[d.exit], 1, rng)
        {
            d.spawns.push((p, Spawn::Monster(MonsterKind::Ogre)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn reachable(d: &Dungeon, from: IVec2) -> Vec<bool> {
        let mut seen = vec![false; (d.w * d.h) as usize];
        let mut queue = VecDeque::from([from]);
        seen[(from.y * d.w + from.x) as usize] = true;
        while let Some(p) = queue.pop_front() {
            for delta in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
                let q = p + delta;
                if d.in_bounds(q) && !seen[(q.y * d.w + q.x) as usize] && !d.get(q).solid() {
                    seen[(q.y * d.w + q.x) as usize] = true;
                    queue.push_back(q);
                }
            }
        }
        seen
    }

    #[test]
    fn every_floor_is_completable() {
        for seed in 0..25u64 {
            for floor in 1..=MAX_FLOOR {
                let d = generate(floor, seed);
                let seen = reachable(&d, d.start);
                let at = |p: IVec2| seen[(p.y * d.w + p.x) as usize];
                assert!(at(d.exit), "floor {floor} seed {seed}: exit unreachable");
                let key = d
                    .spawns
                    .iter()
                    .find(|(_, s)| *s == Spawn::Chest(ChestLoot::Key))
                    .expect("key chest");
                assert!(at(key.0), "floor {floor} seed {seed}: key unreachable");
                for (p, s) in &d.spawns {
                    if matches!(s, Spawn::Shopkeeper) {
                        assert!(at(*p), "floor {floor} seed {seed}: shop unreachable");
                    }
                }
                assert_eq!(d.get(d.exit), Tile::Stairs);
                assert!(!d.get(d.start).solid());
            }
        }
    }

    #[test]
    fn boss_floor_has_lich_and_doors() {
        let d = generate(MAX_FLOOR, 7);
        assert!(
            d.spawns
                .iter()
                .any(|(_, s)| *s == Spawn::Monster(MonsterKind::Lich))
        );
        assert!(!d.boss_doors.is_empty());
    }
}
