# Undercroft

A top-down dungeon escape built with [Bevy](https://bevyengine.org) 0.19. Think
NES *Zelda* dungeons with a modern retro coat of paint: chunky code-drawn pixel
art rendered to a 426x240 canvas and scaled up by whole pixels, dithered
point lighting, screen shake, hit flashes, floating damage numbers, and
chiptune sound synthesized at startup. The game ships no asset files.

```sh
cargo run -p undercroft
```

Add `--release` for a smoother frame rate.

## The game

You wake at the top of an eight-floor dungeon with a rusty sword and one
potion. Each floor is procedurally generated: rooms joined by corridors, a
locked stairway down, a key hidden in a chest, one or two shops, a treasure
room, a secret room behind a cracked wall (hit it with the sword), spike traps,
torches, coins, and monsters. Monsters stay in the room they spawn in, so
you can always retreat into a corridor. Floors 3 and 6 put an Ogre between you and the
stairs. Floor 8 is the Lich's arena: the doors seal behind you and the exit
stays shut until it is destroyed.

- **Sprint** by holding Shift. It is very fast but drains the energy bar, which
  refills after a short pause. The bow costs a little energy too.
- **XP** from kills raises your character level. Levels add a little health and
  energy and, more importantly, unlock gear tiers in shops. Coins pay for them.
- **Shops** sell potions, arrows, three tiers each of swords, bows and armor,
  heart containers, energy crystals, and four magic items (Ring of
  Regeneration, Amulet of Light, Boots of Haste, Phoenix Feather).
- **Enchanted quivers**: shops also sell Poison, Fire and Frost quivers once
  you own a bow. Tab cycles the loaded arrow type. Poison keeps hurting after
  the hit, fire burns and spreads to nearby monsters, frost freezes a monster in
  place for a couple of seconds. Imps' fireballs set *you* on fire, too.
- **Perks**: after every floor you pick one of three random perks (arrow
  piercing, lifesteal, sprint attacks, double arrows, and so on).
- **Death** is permanent. A new run generates a fresh dungeon.

## Controls

| Action              | Keyboard          | Gamepad              |
| ------------------- | ----------------- | -------------------- |
| Move                | WASD / arrows     | Left stick / d-pad   |
| Sprint              | Shift             | Right trigger        |
| Sword               | J or Space        | A / South            |
| Bow                 | K                 | X / West             |
| Drink potion        | Q                 | Y / North            |
| Interact / trade    | E                 | B / East             |
| Cycle arrow type    | Tab               | Left bumper          |
| Pause               | Esc               | Start                |
| Menus               | Arrows + Enter    | D-pad + A            |

## Monsters

| Monster  | Floors | Behaviour                                                   |
| -------- | ------ | ----------------------------------------------------------- |
| Rat      | 1-2    | Weak, quick, jittery. Comes in packs.                       |
| Slime    | 1-3    | Hops toward you in short bursts.                            |
| Bat      | 1-4, 7 | Fast, erratic flight.                                       |
| Kobold   | 2-4    | Ranged. Keeps its distance and lobs slow stones.            |
| Skeleton | 2-8    | Chases you around corners once it has seen you.             |
| Spider   | 3-7    | Waits in ambush, then rushes.                               |
| Archer   | 4-8    | Ranged. Shoots arrows from range.                           |
| Imp      | 4-7    | Ranged. Circles you and spits fireballs.                    |
| Ghost    | 5-8    | Drifts through walls; only solid (and vulnerable) when bright. |
| Cultist  | 5-8    | Ranged. Fires three-way bolt spreads and blinks away when cornered. |
| Knight   | 6-8    | Armored against arrows; winds up a heavy lunge.             |
| Golem    | 7-8    | Slow and very tough; ground slams shake the screen.         |
| Ogre     | 3, 6   | Mini-boss. Charges in straight lines and stuns itself on walls. |
| Lich     | 8      | Boss. Teleports, fires bolt rings and spreads, raises skeletons. |

## How it is put together

| Module         | Role                                                                  |
| -------------- | --------------------------------------------------------------------- |
| `art.rs`       | Every sprite as a 16x16 character pattern, baked into one atlas.      |
| `palette.rs`   | The fixed colour palette.                                             |
| `dungeon.rs`   | Floor generation. Pure data, with tests that every floor is beatable. |
| `floor.rs`     | Spawns the generated floor as entities.                               |
| `physics.rs`   | Tile collision, line of sight, and the flow field monsters chase with.|
| `player.rs`    | Movement, sprint, sword, bow, potions, chests, stairs, pickups.       |
| `enemy.rs`     | Monster stats and per-kind AI.                                        |
| `combat.rs`    | Damage, death, drops, projectiles, traps, particles, shake, numbers.  |
| `items.rs`     | Gear tiers, perks, magic items, shop stock.                           |
| `shop.rs`, `skills.rs` | The shop and perk overlays.                                   |
| `hud.rs`       | Bars, inventory, minimap, messages.                                   |
| `lighting.rs`  | The darkness overlay and its dithered point-light shader.             |
| `audio.rs`     | Sound effects and the music loop, synthesized to WAV in memory.       |
| `input.rs`     | Keyboard and gamepad folded into one snapshot.                        |
| `debug.rs`     | Screenshot autopilot (below).                                         |

The world is drawn by one camera into a 426x240 texture; a second camera shows
that texture scaled by the largest whole number that fits the window, and
draws the UI at full resolution on top.

### Screenshots without a keyboard

```sh
UNDERCROFT_SCREENSHOT_DIR=/some/dir cargo run -p undercroft
```

starts a run on autopilot, wanders, opens the shop and the perk screen, saves
`title.png`, `floor.png`, `explore.png`, `shop.png`, `perks.png` and
`gameover.png`, and quits. `UNDERCROFT_SEED=<n>` fixes the dungeon seed, and
`UNDERCROFT_AUTOBOW=fire` (or `poison`, `frost`, `plain`) also has the hero shoot
enchanted arrows at two knights spawned in front of them.

One rendering gotcha worth knowing: sprite tints must stay within 0..1. A
`Sprite.color` channel above 1.0 blanks the entire frame on Metal (Bevy 0.19),
so flashes are done with saturated in-range colours rather than HDR values.
