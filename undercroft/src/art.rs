//! Code-generated pixel art. Every sprite is a 16x16 pattern of palette keys
//! (see `palette::from_key`), baked into a single texture atlas at startup.
//! Each sprite has two animation frames stored side by side in the atlas.

use crate::palette;
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

/// Size of one tile / sprite cell in world units (and in canvas pixels).
pub const TILE: f32 = 16.0;
const CELL: usize = 16;
const ATLAS_COLS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(usize)]
pub enum SpriteId {
    HeroDown,
    HeroUp,
    HeroSide,
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
    Shopkeeper,
    Coin,
    Potion,
    Key,
    Arrows,
    Sword,
    Bow,
    Armor,
    Heart,
    Crystal,
    Ring,
    Amulet,
    Boots,
    Feather,
    ChestClosed,
    ChestOpen,
    Arrow,
    Bolt,
    Rock,
    Fireball,
    Slash,
    Spikes,
    Torch,
    Floor0,
    Floor1,
    Floor2,
    Floor3,
    Wall,
    WallFace,
    CrackedWall,
    Stairs,
    Carpet,
    Rubble,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
}

impl SpriteId {
    pub const ALL: [SpriteId; 60] = [
        Self::HeroDown,
        Self::HeroUp,
        Self::HeroSide,
        Self::Slime,
        Self::Bat,
        Self::Skeleton,
        Self::Archer,
        Self::Spider,
        Self::Ghost,
        Self::Knight,
        Self::Ogre,
        Self::Lich,
        Self::Rat,
        Self::Kobold,
        Self::Imp,
        Self::Cultist,
        Self::Golem,
        Self::Shopkeeper,
        Self::Coin,
        Self::Potion,
        Self::Key,
        Self::Arrows,
        Self::Sword,
        Self::Bow,
        Self::Armor,
        Self::Heart,
        Self::Crystal,
        Self::Ring,
        Self::Amulet,
        Self::Boots,
        Self::Feather,
        Self::ChestClosed,
        Self::ChestOpen,
        Self::Arrow,
        Self::Bolt,
        Self::Rock,
        Self::Fireball,
        Self::Slash,
        Self::Spikes,
        Self::Torch,
        Self::Floor0,
        Self::Floor1,
        Self::Floor2,
        Self::Floor3,
        Self::Wall,
        Self::WallFace,
        Self::CrackedWall,
        Self::Stairs,
        Self::Carpet,
        Self::Rubble,
        Self::Digit0,
        Self::Digit1,
        Self::Digit2,
        Self::Digit3,
        Self::Digit4,
        Self::Digit5,
        Self::Digit6,
        Self::Digit7,
        Self::Digit8,
        Self::Digit9,
    ];

    /// Atlas cell index of the given animation frame (0 or 1).
    pub fn index(self, frame: usize) -> usize {
        self as usize * 2 + (frame & 1)
    }

    pub fn digit(d: u32) -> SpriteId {
        Self::ALL[Self::Digit0 as usize + (d.min(9) as usize)]
    }
}

/// Handles to the baked atlas.
#[derive(Resource, Clone)]
pub struct Atlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl Atlas {
    pub fn sprite(&self, id: SpriteId, frame: usize) -> Sprite {
        Sprite::from_atlas_image(
            self.image.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index: id.index(frame),
            },
        )
    }
}

type Rows = [&'static str; 16];
type Cell = [[Option<Color>; CELL]; CELL];

/// How the second frame of a sprite is produced.
enum Frame2 {
    /// Identical to the first frame.
    Same,
    /// First frame shifted up one pixel: a cheap walk/idle bob.
    Bob,
    /// A hand-drawn second frame.
    Rows(Rows),
}

enum Pattern {
    Drawn(Rows, Frame2),
    Proc(fn(&mut Cell, usize)),
}

pub fn build_atlas(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let cells = SpriteId::ALL.len() * 2;
    let rows = cells.div_ceil(ATLAS_COLS);
    let (w, h) = (ATLAS_COLS * CELL, rows * CELL);
    let mut data = vec![0u8; w * h * 4];

    for id in SpriteId::ALL {
        let (a, b) = frames(id);
        for (frame, cell) in [a, b].into_iter().enumerate() {
            let index = id.index(frame);
            let (cx, cy) = (index % ATLAS_COLS * CELL, index / ATLAS_COLS * CELL);
            for (y, row) in cell.iter().enumerate() {
                for (x, px) in row.iter().enumerate() {
                    if let Some(color) = px {
                        let rgba = color.to_srgba().to_u8_array();
                        let i = ((cy + y) * w + cx + x) * 4;
                        data[i..i + 4].copy_from_slice(&rgba);
                    }
                }
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let image = images.add(image);
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(CELL as u32),
        ATLAS_COLS as u32,
        rows as u32,
        None,
        None,
    ));
    commands.insert_resource(Atlas { image, layout });
}

fn frames(id: SpriteId) -> (Cell, Cell) {
    match pattern(id) {
        Pattern::Drawn(rows, second) => {
            let a = paint(&rows);
            let b = match second {
                Frame2::Same => a,
                Frame2::Bob => {
                    let mut b = [[None; CELL]; CELL];
                    b[..CELL - 1].copy_from_slice(&a[1..]);
                    b
                }
                Frame2::Rows(rows) => paint(&rows),
            };
            (a, b)
        }
        Pattern::Proc(f) => {
            let mut a = [[None; CELL]; CELL];
            let mut b = [[None; CELL]; CELL];
            f(&mut a, 0);
            f(&mut b, 1);
            (a, b)
        }
    }
}

fn paint(rows: &Rows) -> Cell {
    let mut cell = [[None; CELL]; CELL];
    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), CELL, "sprite row {y} has wrong width: {row:?}");
        for (x, key) in row.chars().enumerate() {
            cell[y][x] = palette::from_key(key);
        }
    }
    cell
}

/// Tiny deterministic random generator for procedural tiles.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn chance(&mut self, one_in: u32) -> bool {
        self.next() % one_in == 0
    }
}

fn fill(cell: &mut Cell, color: Color) {
    for row in cell.iter_mut() {
        for px in row.iter_mut() {
            *px = Some(color);
        }
    }
}

fn floor_tile(cell: &mut Cell, seed: u64) {
    fill(cell, palette::FLOOR);
    let mut rng = Lcg(seed);
    for y in 0..CELL {
        for x in 0..CELL {
            if rng.chance(9) {
                cell[y][x] = Some(palette::FLOOR_DARK);
            } else if rng.chance(14) {
                cell[y][x] = Some(palette::FLOOR_LIGHT);
            }
        }
    }
    // A faint flagstone seam.
    let seam_y = (rng.next() % 12 + 2) as usize;
    let seam_x = (rng.next() % 12 + 2) as usize;
    for i in 0..CELL {
        if rng.chance(3) {
            cell[seam_y][i] = Some(palette::FLOOR_DARK);
        }
        if rng.chance(3) {
            cell[i][seam_x] = Some(palette::FLOOR_DARK);
        }
    }
}

fn bricks(cell: &mut Cell, brick: Color, mortar: Color, seed: u64) {
    let mut rng = Lcg(seed);
    for y in 0..CELL {
        for x in 0..CELL {
            let course = y / 4;
            let offset = if course % 2 == 0 { 0 } else { 4 };
            let is_mortar = y % 4 == 0 || (x + offset) % 8 == 0;
            cell[y][x] = Some(if is_mortar {
                mortar
            } else if rng.chance(7) {
                palette::WALL_DARK
            } else {
                brick
            });
        }
    }
}

fn wall_tile(cell: &mut Cell, frame: usize) {
    bricks(cell, palette::WALL, palette::WALL_MORTAR, 77 + frame as u64);
}

fn wall_face_tile(cell: &mut Cell, frame: usize) {
    bricks(cell, palette::WALL_FACE, palette::WALL_MORTAR, 91 + frame as u64);
    // A lit top edge and a shadow at the base give the wall some height.
    for x in 0..CELL {
        cell[0][x] = Some(palette::LIGHT_GREY);
        cell[CELL - 1][x] = Some(palette::OUTLINE);
        cell[CELL - 2][x] = Some(palette::WALL_MORTAR);
    }
}

fn cracked_wall_tile(cell: &mut Cell, frame: usize) {
    bricks(cell, palette::WALL, palette::WALL_MORTAR, 13 + frame as u64);
    let crack = [
        (7, 0),
        (7, 1),
        (8, 2),
        (8, 3),
        (7, 4),
        (6, 5),
        (6, 6),
        (7, 7),
        (8, 8),
        (9, 9),
        (9, 10),
        (8, 11),
        (7, 12),
        (7, 13),
        (6, 14),
        (6, 15),
        (4, 6),
        (5, 6),
        (10, 9),
        (11, 8),
    ];
    for (x, y) in crack {
        cell[y][x] = Some(palette::VOID);
    }
}

fn stairs_tile(cell: &mut Cell, _frame: usize) {
    fill(cell, palette::VOID);
    for y in 0..CELL {
        for x in 0..CELL {
            if x == 0 || x == CELL - 1 || y == 0 || y == CELL - 1 {
                cell[y][x] = Some(palette::OUTLINE);
            } else if y % 4 == 1 {
                cell[y][x] = Some(palette::GREY);
            } else if y % 4 == 2 {
                cell[y][x] = Some(palette::DARK_GREY);
            } else if y < 8 {
                cell[y][x] = Some(palette::DARKER_GREY);
            } else {
                cell[y][x] = Some(palette::NIGHT);
            }
        }
    }
}

fn carpet_tile(cell: &mut Cell, _frame: usize) {
    fill(cell, palette::CARPET);
    for i in 0..CELL {
        for edge in [1, CELL - 2] {
            cell[edge][i] = Some(palette::CARPET_TRIM);
            cell[i][edge] = Some(palette::CARPET_TRIM);
        }
    }
    for (x, y) in [(7, 6), (8, 6), (6, 7), (9, 7), (6, 8), (9, 8), (7, 9), (8, 9)] {
        cell[y][x] = Some(palette::CARPET_TRIM);
    }
}

fn rubble_tile(cell: &mut Cell, frame: usize) {
    floor_tile(cell, 300 + frame as u64);
    let mut rng = Lcg(500);
    for _ in 0..7 {
        let x = (rng.next() % 14) as usize;
        let y = (rng.next() % 14) as usize;
        cell[y][x] = Some(palette::GREY);
        cell[y][x + 1] = Some(palette::DARK_GREY);
        cell[y + 1][x] = Some(palette::DARK_GREY);
    }
}

fn digit_tile(cell: &mut Cell, digit: usize) {
    const GLYPHS: [[&str; 5]; 10] = [
        ["www", "w.w", "w.w", "w.w", "www"],
        [".w.", "ww.", ".w.", ".w.", "www"],
        ["www", "..w", "www", "w..", "www"],
        ["www", "..w", "www", "..w", "www"],
        ["w.w", "w.w", "www", "..w", "..w"],
        ["www", "w..", "www", "..w", "www"],
        ["www", "w..", "www", "w.w", "www"],
        ["www", "..w", "..w", "..w", "..w"],
        ["www", "w.w", "www", "w.w", "www"],
        ["www", "w.w", "www", "..w", "www"],
    ];
    for (y, row) in GLYPHS[digit].iter().enumerate() {
        for (x, key) in row.chars().enumerate() {
            if key == 'w' {
                cell[5 + y][6 + x] = Some(palette::WHITE);
            }
        }
    }
}

fn pattern(id: SpriteId) -> Pattern {
    use Frame2::*;
    use Pattern::*;
    match id {
        SpriteId::HeroDown => Drawn(
            [
                "................",
                "................",
                ".....kkkkkk.....",
                "....khhhhhhk....",
                "...khhhhhhhhk...",
                "...khsssssshk...",
                "...kskssskssk...",
                "...kssssssssk...",
                "....kssSSssk....",
                ".....kbbbbk.....",
                "...kbbBbbBbbk...",
                "..ksbbbbbbbbsk..",
                "..kskbbbbbbksk..",
                "....kBBBBBBk....",
                "....kNNkkNNk....",
                ".....kk..kk.....",
            ],
            Bob,
        ),
        SpriteId::HeroUp => Drawn(
            [
                "................",
                "................",
                ".....kkkkkk.....",
                "....khhhhhhk....",
                "...khhhhhhhhk...",
                "...khhhhhhhhk...",
                "...khhhhhhhhk...",
                "...kshhhhhhsk...",
                "....ksssssk.....",
                ".....kbbbbk.....",
                "...kbbbbbbbbk...",
                "..ksbbbbbbbbsk..",
                "..kskbbbbbbksk..",
                "....kBBBBBBk....",
                "....kNNkkNNk....",
                ".....kk..kk.....",
            ],
            Bob,
        ),
        SpriteId::HeroSide => Drawn(
            [
                "................",
                "................",
                ".....kkkkkk.....",
                "....khhhhhhk....",
                "...khhhhhhhhk...",
                "...khhhsssssk...",
                "...khhssskssk...",
                "....kssssssk....",
                ".....ksSSsk.....",
                ".....kbbbbk.....",
                "....kbbbbbbk....",
                "....kbbbbbbkk...",
                "....kbbbbbksk...",
                "....kBBBBBBk....",
                ".....kNNNNk.....",
                ".....kNkkNk.....",
            ],
            Bob,
        ),
        SpriteId::Slime => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                ".....kkkkkk.....",
                "....kqqqqqqk....",
                "...kqqqqqqqqk...",
                "..kqqkqqqqkqqk..",
                "..kqqkqqqqkqqk..",
                "..kqqqqqqqqqqk..",
                "..kgqqqqqqqqgk..",
                "..kgggqqqqgggk..",
                "..kGgggggggggk..",
                "...kGGGGGGGGk...",
                "....kkkkkkkk....",
                "................",
            ],
            Rows([
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
                "....kkkkkkkk....",
                "..kkqqqqqqqqkk..",
                ".kqqkqqqqqqkqqk.",
                ".kqqkqqqqqqkqqk.",
                ".kqqqqqqqqqqqqk.",
                ".kggqqqqqqqqggk.",
                ".kGggggggggggGk.",
                "..kGGGGGGGGGGk..",
                "...kkkkkkkkkk...",
                "................",
            ]),
        ),
        SpriteId::Bat => Drawn(
            [
                "................",
                "................",
                "................",
                "..kk........kk..",
                ".kppk......kppk.",
                ".kpppk....kpppk.",
                ".kppppkkkkppppk.",
                "..kpppPPPPpppk..",
                "...kpPrPPrPpk...",
                "....kPPPPPPk....",
                ".....kPkkPk.....",
                "......k..k......",
                "................",
                "................",
                "................",
                "................",
            ],
            Rows([
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
                "......kkkk......",
                "..kkkkPPPPkkkk..",
                ".kpppkPrPPrPkppk",
                ".kppppkPPPPkpppk",
                "..kppppkPPkpppk.",
                "...kkkkkPPkkkk..",
                "........kk......",
                "................",
                "................",
                "................",
            ]),
        ),
        SpriteId::Skeleton => Drawn(
            [
                "................",
                ".....kkkkkk.....",
                "....kttttttk....",
                "...kttttttttk...",
                "...ktkkttkktk...",
                "...kttttttttk...",
                "....ktktktkk....",
                ".....kkttkk.....",
                "...kttkttkttk...",
                "..ktkttttttktk..",
                "..ktkkttttkktk..",
                "....kktttkk.....",
                ".....kTTTTk.....",
                "....kttkkttk....",
                "....kttk.kttk...",
                ".....kk...kk....",
            ],
            Bob,
        ),
        SpriteId::Archer => Drawn(
            [
                "................",
                ".....kkkkkk.....",
                "....kGGGGGGk....",
                "...kGGGGGGGGk...",
                "...kGkkttkkGk...",
                "...kGtttttGGk...",
                "....kGttttGk....",
                ".....kkttkk.....",
                "...kttkttkttkn..",
                "..ktkttttttktkn.",
                "..ktkkttttkktkn.",
                "....kktttkk...n.",
                ".....kTTTTk...n.",
                "....kttkkttk....",
                "....kttk.kttk...",
                ".....kk...kk....",
            ],
            Bob,
        ),
        SpriteId::Spider => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "..k....kk....k..",
                "..k.k.kddk.k.k..",
                "...kkkdddddkkk..",
                "..k.kdrdddrdk.k.",
                "..k.kdddddddk.k.",
                "...kkkdddddkkk..",
                "..k.k.kdddk.k.k.",
                "..k....kdk...k..",
                ".......kkk......",
                "................",
                "................",
                "................",
            ],
            Bob,
        ),
        SpriteId::Ghost => Drawn(
            [
                "................",
                "................",
                ".....kkkkkk.....",
                "....kwwwwwwk....",
                "...kwwwwwwwwk...",
                "...kwkwwwwkwk...",
                "...kwkwwwwkwk...",
                "...kwwwwwwwwk...",
                "...kwwwWWwwwk...",
                "...kwwwwwwwwk...",
                "...kwWwwwwWwk...",
                "...kWwwWWwwWk...",
                "...kWkWkkWkWk...",
                "....k.k..k.k....",
                "................",
                "................",
            ],
            Bob,
        ),
        SpriteId::Knight => Drawn(
            [
                "......kkk.......",
                ".....krrrk......",
                "....kkrrrkk.....",
                "...kWWWWWWWWk...",
                "...kWeeeeeeWk...",
                "...kekkkkkkek...",
                "...kWeeeeeeWk...",
                "....kWWWWWWk....",
                "...keWWWWWWekw..",
                "..kWkWWrrWWkWkw.",
                "..kWkWWrrWWkWkw.",
                "...kkWWWWWWkkkw.",
                "....keeeeeek....",
                "....kEEkkEEk....",
                "....kEEk.kEEk...",
                ".....kk...kk....",
            ],
            Bob,
        ),
        SpriteId::Ogre => Drawn(
            [
                "................",
                "....kkkkkkkk....",
                "...kggggggggk...",
                "..kggkggggkggk..",
                "..kggggggggggk..",
                "..kgggkkkkgggk..",
                "...kggkwkwkgk...",
                "....kkkkkkkk....",
                "..kkkNNNNNNkkk..",
                ".kggkNNNNNNkggk.",
                ".kggkNNNNNNkggk.",
                ".kkkkNNNNNNkkkk.",
                "....kNNkkNNk....",
                "...kNNNkkNNNk...",
                "...kkkk..kkkk...",
                "................",
            ],
            Bob,
        ),
        SpriteId::Lich => Drawn(
            [
                ".....y.yy.y.....",
                ".....kyyyyk.....",
                "....kttttttk....",
                "...kttttttttk...",
                "...ktkcttkctk...",
                "...kttttttttk...",
                "....ktktktkk....",
                ".....kkttkk.....",
                "....kpppppppk...",
                "...kppPPPPPppk..",
                "..kppPPcPPPppk..",
                "..kpPPPPPPPPpk..",
                "..kPPPPPPPPPPk..",
                "..kPPPPPPPPPPk..",
                "...kkkkkkkkkk...",
                "................",
            ],
            Bob,
        ),
        SpriteId::Rat => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "................",
                "......kk........",
                ".....kEEk.kk....",
                "....kEkEEkEEk...",
                "...kEEEEEEEEEk..",
                "..kEEEEEEEEEEEkk",
                "..kEEEEEEEEEEk.k",
                "...kEEEEEEEEk..k",
                "....kkkEEkkk..k.",
                "....kik.kik..k..",
                ".....k...k..k...",
                "................",
            ],
            Bob,
        ),
        SpriteId::Kobold => Drawn(
            [
                "................",
                ".....kkkk.......",
                "....kNNNNk......",
                "...kNyNNyNk.....",
                "...kNNNNNNk.....",
                "....kNkkNk......",
                ".....kkkk.......",
                "....kooook......",
                "...kNkoookNk....",
                "..kNk.kooook.kk.",
                "..kk...kookk.ke.",
                ".......kkkk...k.",
                "......kNNkNNk...",
                "......kNNk.kNNk.",
                ".......kk...kk..",
                "................",
            ],
            Bob,
        ),
        SpriteId::Imp => Drawn(
            [
                "................",
                "....k......k....",
                "...krk....krk...",
                "....kkrrrrkk....",
                "....krrrrrrk....",
                "...krrkrrkrrk...",
                "..kkkrrrrrrkkk..",
                ".kRRkrkkkkrkRRk.",
                ".kRRRkrrrrkRRRk.",
                "..kRkrrrrrrkRk..",
                "...k.krrrrk.k...",
                ".....kRkkRk.....",
                ".....kRk.kRk....",
                "......k...k.....",
                "................",
                "................",
            ],
            Rows([
                "................",
                "....k......k....",
                "...krk....krk...",
                "....kkrrrrkk....",
                "....krrrrrrk....",
                "...krrkrrkrrk...",
                "....krrrrrrk....",
                "..kkkrkkkkrkkk..",
                ".kRRRkrrrrkRRRk.",
                ".kRRRkrrrrkRRRk.",
                "..kkkkrrrrkkkk..",
                ".....kRkkRk.....",
                ".....kRk.kRk....",
                "......k...k.....",
                "................",
                "................",
            ]),
        ),
        SpriteId::Cultist => Drawn(
            [
                "................",
                ".....kkkkkk.....",
                "....kPPPPPPk....",
                "...kPPPPPPPPk...",
                "...kPkkkkkkPk...",
                "...kPkcPPckPk...",
                "...kPPkkkkPPk...",
                "....kPPPPPPk....",
                "...kPPPPPPPPk...",
                "..kPPkPPPPkPPk..",
                "..kPPkPPPPkPPk..",
                "..kkkkPPPPkkkk..",
                "...kPPPPPPPPk...",
                "...kPPPPPPPPk...",
                "...kkkkkkkkkk...",
                "................",
            ],
            Bob,
        ),
        SpriteId::Golem => Drawn(
            [
                "................",
                "...kkkkkkkkkk...",
                "..keeeeeeeeeek..",
                "..keEkeeeekEek..",
                "..keeyeeeeyeek..",
                "..kEeeeeeeeeEk..",
                "..kkEEEEEEEEkk..",
                ".kekEeeeeeeEkek.",
                ".kekEeeeeeeEkek.",
                ".kekEEeeeeEEkek.",
                ".kEkkEEEEEEkkEk.",
                ".kkk.kEEEEk.kkk.",
                ".....kEEEEk.....",
                "....kEEkkEEk....",
                "....kkkk.kkkk...",
                "................",
            ],
            Bob,
        ),
        SpriteId::Shopkeeper => Drawn(
            [
                "................",
                ".....kkkkkk.....",
                "....knnnnnnk....",
                "...knnnnnnnnk...",
                "...knkssssknk...",
                "...knskssksnk...",
                "...knnssssnnk...",
                "....knnnnnnk....",
                "...kUnnnnnnUk...",
                "..ksnnnUUnnnsk..",
                "..kskNNUUNNksk..",
                "...kNNNUUNNNk...",
                "...kNNNNNNNNk...",
                "...kNNNNNNNNk...",
                "....kkkkkkkk....",
                "................",
            ],
            Bob,
        ),
        SpriteId::Coin => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                ".....kkkkkk.....",
                "....kyyyyyyk....",
                "...kyyYYYYyyk...",
                "...kyYyyyyYyk...",
                "...kyYyYYyYyk...",
                "...kyYyyyyYyk...",
                "...kyyYYYYyyk...",
                "....kyyyyyyk....",
                ".....kkkkkk.....",
                "................",
                "................",
                "................",
            ],
            Rows([
                "................",
                "................",
                "................",
                "................",
                ".......kk.......",
                "......kyyk......",
                "......kyYk......",
                "......kyYk......",
                "......kyYk......",
                "......kyYk......",
                "......kyYk......",
                "......kyyk......",
                ".......kk.......",
                "................",
                "................",
                "................",
            ]),
        ),
        SpriteId::Potion => Drawn(
            [
                "................",
                "................",
                "......kkkk......",
                "......kNNk......",
                "......kttk......",
                ".....kkttkk.....",
                "....kWWttWWk....",
                "...kWWrrrrWWk...",
                "...kWrrrrrrWk...",
                "...krrrRRrrrk...",
                "...krrRRRRrrk...",
                "...kRRRRRRRRk...",
                "....kRRRRRRk....",
                ".....kkkkkk.....",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Key => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "....kkkk........",
                "...kyyyyk.......",
                "...kyk.kyk......",
                "...kyk.kyk......",
                "...kyyyyyk......",
                "....kkkyyk......",
                "......kyykkkkk..",
                "......kyyyyyyyk.",
                "......kkkykykyk.",
                ".........kkkkk..",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Arrows => Drawn(
            [
                "................",
                "....k...k...k...",
                "...kWk.kWk.kWk..",
                "...kWk.kWk.kWk..",
                "....k...k...k...",
                "....n...n...n...",
                "....n...n...n...",
                "....n...n...n...",
                "....n...n...n...",
                "....n...n...n...",
                "....n...n...n...",
                "...krk.krk.krk..",
                "...krk.krk.krk..",
                "...krk.krk.krk..",
                "....k...k...k...",
                "................",
            ],
            Same,
        ),
        SpriteId::Sword => Drawn(
            [
                "................",
                ".......kk.......",
                "......kWWk......",
                "......kWWk......",
                "......kWWk......",
                "......kWek......",
                "......kWek......",
                "......kWek......",
                "......kWek......",
                "......kWek......",
                "....kkkWekkk....",
                "....kYYYYYYk....",
                ".....kkNNkk.....",
                "......kNNk......",
                "......kyyk......",
                ".......kk.......",
            ],
            Same,
        ),
        SpriteId::Bow => Drawn(
            [
                "................",
                ".......kkk......",
                "......knnnk.....",
                ".....knk.wk.....",
                ".....knk..w.....",
                "....knk...w.....",
                "....knk...w.....",
                "....knk...w.....",
                "....knk...w.....",
                "....knk...w.....",
                "....knk...w.....",
                ".....knk..w.....",
                ".....knk.wk.....",
                "......knnnk.....",
                ".......kkk......",
                "................",
            ],
            Same,
        ),
        SpriteId::Armor => Drawn(
            [
                "................",
                "................",
                "...kkk....kkk...",
                "..kWWWkkkkWWWk..",
                "..kWWWWeeWWWWk..",
                "..kWWWWWWWWWWk..",
                "..kekWWWWWWkek..",
                "..kkkWWeeWWkkk..",
                "....kWWeeWWk....",
                "....kWWWWWWk....",
                "....kWWWWWWk....",
                "....kWeWWeWk....",
                ".....kWWWWk.....",
                "......kkkk......",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Heart => Drawn(
            [
                "................",
                "................",
                "................",
                "...kkk....kkk...",
                "..kirrk..krrik..",
                ".kirrrrkkrrrrrk.",
                ".krrrrrrrrrrrrk.",
                ".krrrrrrrrrrrrk.",
                ".kRrrrrrrrrrrRk.",
                "..kRrrrrrrrrRk..",
                "...kRRrrrrRRk...",
                "....kRRrrRRk....",
                ".....kRRRRk.....",
                "......kRRk......",
                ".......kk.......",
                "................",
            ],
            Same,
        ),
        SpriteId::Crystal => Drawn(
            [
                "................",
                ".......kk.......",
                "......kcck......",
                "......kwck......",
                ".....kcwcck.....",
                ".....kcwcCk.....",
                "....kccwcCCk....",
                "....kccccCCk....",
                "....kCccCCCk....",
                "....kCCcCCCk....",
                ".....kCCCCk.....",
                ".....kCCCCk.....",
                "......kCCk......",
                ".......kk.......",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Ring => Drawn(
            [
                "................",
                "................",
                "................",
                "......krrk......",
                ".....krrrrk.....",
                "......krrk......",
                ".....kkyykk.....",
                "....kyykkkyyk...",
                "...kyk.....kyk..",
                "...kyk.....kyk..",
                "...kyk.....kyk..",
                "....kyk...kyk...",
                ".....kyykkyyk...",
                "......kkkkk.....",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Amulet => Drawn(
            [
                "................",
                ".....kkkkkk.....",
                "....ky....yk....",
                "...ky......yk...",
                "...ky......yk...",
                "...ky......yk...",
                "....ky....yk....",
                ".....kyyyyk.....",
                ".....kyccyk.....",
                "....kycwccyk....",
                "....kyccCcyk....",
                "....kycCCcyk....",
                ".....kyCCyk.....",
                "......kyyk......",
                ".......kk.......",
                "................",
            ],
            Same,
        ),
        SpriteId::Boots => Drawn(
            [
                "................",
                "................",
                "................",
                "....kkk..kkk....",
                "....kNNk.kNNk...",
                "....knnk.knnk...",
                "....knnk.knnk...",
                "....knnk.knnk...",
                "...kynnk.kynnk..",
                "..kyynnkkyynnk..",
                "..kynnnkkynnnkk.",
                "..knnnnkknnnnnk.",
                "..kNNNNkkNNNNNk.",
                "...kkkk..kkkkk..",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Feather => Drawn(
            [
                "................",
                "...........kk...",
                "..........koYk..",
                ".........koYYk..",
                "........koYYyk..",
                "........koYyk...",
                ".......koYYyk...",
                ".......koYyk....",
                "......koYYyk....",
                "......koYyk.....",
                ".....korYk......",
                ".....korYk......",
                "....kNrrk.......",
                "...kNNk.........",
                "..kNk...........",
                "..kk............",
            ],
            Same,
        ),
        SpriteId::ChestClosed => Drawn(
            [
                "................",
                "................",
                "................",
                "...kkkkkkkkkk...",
                "..knnnnnnnnnnk..",
                "..knNnnnnnnNnk..",
                "..kkkkkkkkkkkk..",
                "..knnnnkyknnnk..",
                "..knnnnkkknnnk..",
                "..knNnnnnnnNnk..",
                "..knNnnnnnnNnk..",
                "..kNNNNNNNNNNk..",
                "...kkkkkkkkkk...",
                "................",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::ChestOpen => Drawn(
            [
                "................",
                "...kkkkkkkkkk...",
                "..knnnnnnnnnnk..",
                "..knNnnnnnnNnk..",
                "..kkkkkkkkkkkk..",
                "..kddddddddddk..",
                "..kdyydddyyddk..",
                "..knnnnkyknnnk..",
                "..knnnnkkknnnk..",
                "..knNnnnnnnNnk..",
                "..knNnnnnnnNnk..",
                "..kNNNNNNNNNNk..",
                "...kkkkkkkkkk...",
                "................",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Arrow => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
                "..kk............",
                "..krk.......kk..",
                "..krknnnnnnnkWk.",
                "..krk.......kk..",
                "..kk............",
                "................",
                "................",
                "................",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Bolt => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "................",
                "......kkkk......",
                ".....kppppk.....",
                "....kpcwwcpk....",
                "....kpcwwcpk....",
                ".....kppppk.....",
                "......kkkk......",
                "................",
                "................",
                "................",
                "................",
                "................",
            ],
            Rows([
                "................",
                "................",
                "................",
                "................",
                "......kkkk......",
                ".....kccppk.....",
                "....kcwwccpk....",
                "....kpwwwcpk....",
                "....kpcwwcpk....",
                "....kpccccpk....",
                ".....kppppk.....",
                "......kkkk......",
                "................",
                "................",
                "................",
                "................",
            ]),
        ),
        SpriteId::Rock => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
                "......kkk.......",
                ".....keeEk......",
                ".....kEEEk......",
                "......kkk.......",
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
            ],
            Same,
        ),
        SpriteId::Fireball => Drawn(
            [
                "................",
                "................",
                "................",
                "................",
                "................",
                "......kkk.......",
                ".....kYyyk......",
                "....kYyywyk.....",
                "....koYyyyk.....",
                ".....kooYk......",
                "......kkk.......",
                "................",
                "................",
                "................",
                "................",
                "................",
            ],
            Rows([
                "................",
                "................",
                "................",
                "................",
                "......kkk.......",
                ".....kyYyk......",
                "....kYywyyk.....",
                "....kYyyyYk.....",
                "....kooYYok.....",
                ".....kookk......",
                "......kk........",
                "................",
                "................",
                "................",
                "................",
                "................",
            ]),
        ),
        SpriteId::Slash => Drawn(
            [
                "................",
                ".......kk.......",
                "........kwk.....",
                ".........kwk....",
                "..........kwk...",
                "..........kwwk..",
                "..........kwwk..",
                "..........kwwk..",
                "..........kwwk..",
                "..........kwwk..",
                "..........kwwk..",
                "..........kwk...",
                ".........kwk....",
                "........kwk.....",
                ".......kk.......",
                "................",
            ],
            Same,
        ),
        SpriteId::Spikes => Drawn(
            [
                "................",
                "................",
                "....d......d....",
                "...ddd....ddd...",
                "....d......d....",
                "................",
                "................",
                ".......d........",
                "......ddd.......",
                ".......d........",
                "................",
                "................",
                "....d......d....",
                "...ddd....ddd...",
                "....d......d....",
                "................",
            ],
            Rows([
                "....k......k....",
                "...kWk....kWk...",
                "...kWk....kWk...",
                "..kWWek..kWWek..",
                "..keeek..keeek..",
                ".......k........",
                "......kWk.......",
                "......kWk.......",
                ".....kWWek......",
                ".....keeek......",
                "....k......k....",
                "...kWk....kWk...",
                "...kWk....kWk...",
                "..kWWek..kWWek..",
                "..keeek..keeek..",
                "................",
            ]),
        ),
        SpriteId::Torch => Drawn(
            [
                "................",
                "................",
                ".......y........",
                "......yYy.......",
                "......yYYy......",
                ".....yYYoYy.....",
                ".....yYooYy.....",
                "......YooY......",
                ".......oo.......",
                "......kkkk......",
                ".......kNk......",
                ".......kNk......",
                ".......kNk......",
                "......kkkkk.....",
                "................",
                "................",
            ],
            Rows([
                "................",
                "................",
                "........y.......",
                ".......yy.......",
                "......yYYy......",
                "......yYYYy.....",
                ".....yYoYYy.....",
                "......YooYy.....",
                ".......oo.......",
                "......kkkk......",
                ".......kNk......",
                ".......kNk......",
                ".......kNk......",
                "......kkkkk.....",
                "................",
                "................",
            ]),
        ),
        SpriteId::Floor0 => Proc(|c, f| floor_tile(c, 1 + f as u64)),
        SpriteId::Floor1 => Proc(|c, f| floor_tile(c, 20 + f as u64)),
        SpriteId::Floor2 => Proc(|c, f| floor_tile(c, 40 + f as u64)),
        SpriteId::Floor3 => Proc(|c, f| floor_tile(c, 60 + f as u64)),
        SpriteId::Wall => Proc(wall_tile),
        SpriteId::WallFace => Proc(wall_face_tile),
        SpriteId::CrackedWall => Proc(cracked_wall_tile),
        SpriteId::Stairs => Proc(stairs_tile),
        SpriteId::Carpet => Proc(carpet_tile),
        SpriteId::Rubble => Proc(rubble_tile),
        SpriteId::Digit0 => Proc(|c, _| digit_tile(c, 0)),
        SpriteId::Digit1 => Proc(|c, _| digit_tile(c, 1)),
        SpriteId::Digit2 => Proc(|c, _| digit_tile(c, 2)),
        SpriteId::Digit3 => Proc(|c, _| digit_tile(c, 3)),
        SpriteId::Digit4 => Proc(|c, _| digit_tile(c, 4)),
        SpriteId::Digit5 => Proc(|c, _| digit_tile(c, 5)),
        SpriteId::Digit6 => Proc(|c, _| digit_tile(c, 6)),
        SpriteId::Digit7 => Proc(|c, _| digit_tile(c, 7)),
        SpriteId::Digit8 => Proc(|c, _| digit_tile(c, 8)),
        SpriteId::Digit9 => Proc(|c, _| digit_tile(c, 9)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sprite_is_16_by_16() {
        for id in SpriteId::ALL {
            let _ = frames(id);
        }
    }

    #[test]
    fn all_list_matches_discriminants() {
        for (i, id) in SpriteId::ALL.iter().enumerate() {
            assert_eq!(*id as usize, i, "{id:?} is out of order in ALL");
        }
    }
}
