# Games

A Cargo workspace of small games built on [Bevy](https://bevyengine.org) 0.19.

- **Undercroft** (`undercroft/`): a top-down retro dungeon escape. See [undercroft/README.md](undercroft/README.md).
- **Cat Powncer** (`cat_powncer/`): a cartoon 3D grid-hop platformer for young kids. See [cat_powncer/README.md](cat_powncer/README.md).
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

## Run in a browser

Every game also builds to WebAssembly. Install the tools once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-server-runner
cargo install wasm-bindgen-cli --version <version of wasm-bindgen in Cargo.lock>
```

To play a game during development, run it with the wasm target. `.cargo/config.toml`
routes it to `wasm-server-runner`, which serves `web/dev.html` and prints a local URL:

```sh
cargo run --target wasm32-unknown-unknown -p flappy_bird
```

To publish, build the whole site:

```sh
web/build.sh
```

This writes `dist/`: a landing page at `dist/index.html` that lists the games as a
picker (`web/site.html`, with cover images from `web/covers/`), and each game under
`dist/<game>/` as `index.html` plus the JS glue and the `.wasm`, built with the
size-tuned `wasm-release` profile. Upload the directory to any static host; all links
are relative, so it also works under a sub-path. The `.wasm` must be served with the
`application/wasm` MIME type, which every common host does by default. If `wasm-opt`
from Binaryen is installed, the script runs it too.

`.github/workflows/pages.yml` does all of this on every push to `main` and deploys to
GitHub Pages. Enable it once under Settings > Pages by setting the source to
"GitHub Actions". Netlify, Cloudflare Pages, and similar hosts can run `web/build.sh`
as the build command with `dist` as the output directory.

`web/covers.sh` regenerates the cover images from each game's screenshot mode.

### How the page fits the screen

The games draw into a `<canvas id="game">` inside a `<div id="frame">`, and ask Bevy to
keep the canvas the size of the frame. Undercroft fills the whole viewport and scales its
pixel-art canvas to the largest integer factor that fits. The Flappy games are portrait,
so at startup they size the frame to the largest 3:4 rectangle that fits, and the page
letterboxes the rest in black. Each game's UI scales with the canvas.

### Version matching

`wasm-server-runner` bundles its own copy of `wasm-bindgen`, and `wasm-bindgen-cli` must
match the `wasm-bindgen` crate in `Cargo.lock` exactly. If either complains about a
schema mismatch, run `cargo update wasm-bindgen js-sys web-sys wasm-bindgen-futures` or
reinstall the tool at the locked version. `web/build.sh` checks this before building.

Browser notes: audio stays silent until the first click or keypress, and Escape does not
quit in the browser.

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
