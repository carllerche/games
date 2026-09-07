# Games

A Cargo workspace of small games built on [Bevy](https://bevyengine.org) 0.19.

- **Undercroft** (`undercroft/`): a top-down retro dungeon escape. See [undercroft/README.md](undercroft/README.md).
- **Flappy Bird, twice**: two Flappy Bird games that share one gameplay implementation, described below.

## Flappy Bird

| Crate            | What it is                                                                 |
| ---------------- | -------------------------------------------------------------------------- |
| `flappy_core`    | All game logic: physics, state machine, pipes, scoring, collision, debris. |
| `flappy_bird`    | The 2D game, drawn with flat colored sprites.                               |
| `flappy_bird_3d` | The same game rendered in 3D with cel shading and ink outlines, in the style of *The Wind Waker*. |

## Run

```sh
cargo run -p flappy_bird
cargo run -p flappy_bird_3d
```

Add `--release` for a smoother frame rate. The first build takes a few minutes because
Bevy is large; later builds are quick and both games share the compiled dependencies.

## Controls

| Action  | Keys                                    |
| ------- | --------------------------------------- |
| Flap    | Space, Up arrow, W, or left mouse click |
| Start   | Any flap input on the title screen      |
| Restart | Any flap input on the game over screen  |
| Quit    | Escape                                  |

## How the crates fit together

`flappy_core` owns every entity that matters to gameplay but never draws anything.
It spawns pipe pairs (with invisible colliders), moves the bird, scrolls the ground,
and spawns explosion debris. Each game crate:

- spawns the bird, ground tiles, and HUD at startup, and
- watches for newly added `PipePair` and `Particle` entities and attaches visuals to them.

Because both games run the exact same systems from `flappy_core`, they play identically.
All tuning constants (gravity, flap strength, gap size, speed) live at the top of
`flappy_core/src/lib.rs`, and `cargo test -p flappy_core` checks the physics and geometry.

## The 3D look

Everything in `flappy_bird_3d` is built in code from four unit primitives (sphere, cube,
cylinder, cone), so there are no model files. Two tricks give the cartoon style:

- **Cel shading** (`src/shaders/toon.wgsl`): a custom material with two hard lighting
  bands and a faint rim highlight, ignoring Bevy's lights in favor of one fixed sun direction.
- **Ink outlines**: every part gets a slightly enlarged black copy of its mesh drawn with
  front faces culled, so only a thin rim shows around the silhouette.

The shader is embedded in the binary, so the game runs from anywhere.

### Screenshots without a keyboard

```sh
FLAPPY_SCREENSHOT_DIR=/some/dir cargo run -p flappy_bird_3d
```

plays the game on autopilot for a few seconds, saves `title.png`, `playing.png`, and
`late.png` into the directory, and quits.

## Note on dependencies

`tinyvec` is pinned to 1.10.0 in `Cargo.lock` because 1.13.0 fails to compile on the
current Rust toolchain. If a `cargo update` pulls in the newer version and the build
breaks, run `cargo update tinyvec --precise 1.10.0`.
