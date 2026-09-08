# Cat Powncer

A cartoon 3D platformer for young kids, written in Rust with [Bevy](https://bevy.org).

You are a cat hopping across a giant floating cat-tree world. Swipe the toys off
the platforms, scratch at the slobbery dogs, and find the **purple** yarn ball to
finish each level. There are 22 levels.

## Running

```sh
cargo run --release -p cat_powncer
```

The first build compiles Bevy and takes a few minutes. After that, builds are quick.
The game also runs in a browser; see the [workspace README](../README.md).

## How to play

| Action | Keyboard | Mouse |
| --- | --- | --- |
| Hop one tile | Arrow keys or WASD | Click a tile |
| Swipe (toys and dogs) | Space or Enter | Click the toy or dog next to you |
| Restart the level | R | |
| Back to the title screen | Esc | |

- Hops are one tile at a time in the four grid directions.
- Hop toward a one-tile gap and the cat long-jumps across it, as long as the far
  side is not higher than where you stand.
- Hop off an edge and the cat falls, then reappears on the tile it left.
- **Toys** block a tile until you swipe them off the platform.
- **Dogs** patrol their island and chase you when you get close. If a dog licks
  you, the cat turns slimy and hops slowly for a few seconds. Two swipes send a dog
  tumbling off the cat tree.
- **Yarn balls** come in several colours. Only purple finishes the level. Land on
  any other colour and you go back to the start of the level.

## Levels

Levels are generated procedurally from a fixed seed per level number, so level 7
is always the same level 7. Later levels add more islands, height changes, jump
gaps, dogs, toys, and decoy yarn balls. A unit test checks that every level can be
completed without touching a wrong-coloured yarn ball.

Print a level's layout without opening a window:

```sh
cargo run -p cat_powncer -- --map 12
```

## Debug flags

| Flag | Effect |
| --- | --- |
| `--level N` | Skip the title screen and start on level N |
| `--map N` | Print level N as ASCII art and exit |
| `--script "R,U,S"` | Feed scripted input (R/L/U/D hop, S swipe), one key every half second |
| `--screenshot PATH [--after SECONDS]` | Render offscreen, save a PNG, and exit |

## Code layout

| File | What it does |
| --- | --- |
| `src/main.rs` | App setup, game states, level phase timers, debug flags |
| `src/grid.rs` | The tile grid, directions, and the cat's hop rules |
| `src/level.rs` | Procedural level generation and solvability tests |
| `src/world.rs` | Meshes and materials, island construction, camera, particles |
| `src/cat.rs` | The cat: model, input, hopping, swiping, falling, slime |
| `src/dog.rs` | Dogs: model, patrol and chase behaviour, licking |
| `src/ui.rs` | Title screen, HUD, banners, victory screen |
| `src/audio.rs` | Synthesized sound effects |
| `src/rng.rs` | Small deterministic random number generator |

All models are built from Bevy's primitive shapes, so there are no asset files.
