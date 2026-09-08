//! A tiny deterministic PCG-style random number generator.
//!
//! Levels are generated from a fixed seed per level number, so every play of
//! "Level 7" is the same Level 7. Keeping the generator in-tree avoids pulling
//! in a crate and keeps behaviour identical across platforms.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03,
        };
        // Warm up so nearby seeds diverge quickly.
        for _ in 0..4 {
            rng.next_u32();
        }
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        // PCG-XSH-RR, 64-bit state, 32-bit output.
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform float in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }


    /// Uniform integer in `[lo, hi]` (inclusive). `hi` must be >= `lo`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u32() as u64 % span) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range_i32(0, items.len() as i32 - 1) as usize]
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range_i32(0, i as i32) as usize;
            items.swap(i, j);
        }
    }
}
