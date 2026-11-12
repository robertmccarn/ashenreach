# Ashen Reach

A from-scratch, first-person fantasy RPG project in Rust. The first milestone is a small playable desktop dungeon: a fixed-step-friendly game loop, a software raycaster, collision-aware movement, and a hand-authored test map.

## Run

Install Rust with Cargo, then run:

```sh
cargo run
```

Cargo downloads Macroquad on the first build. No SDL or system graphics SDK is required.

## Controls

- `W` / `S` or up/down: move forward/back
- `A` / `D` or left/right: turn
- `Q` / `E`: turn
- `F` or `Space`: open/close a nearby door or use the exit
- `R`: restart after completing the level
- `Esc`: quit

## Current shape

The renderer casts one ray per vertical screen strip against a grid map. Player movement checks walls and closed doors and slides along them by resolving each axis separately. Find and open both doors, explore the dead-end side room, and interact with the green exit. The completion screen reports elapsed time and tile steps; press `R` to restart.

Simulation input is represented as plain data and the world/player/game modules do not draw. Simulation updates run at a fixed 60 Hz; rendering remains frame-driven. Keep generation deterministic and content data-driven; add chunking, a thread pool, and editor tools only when the playable loop needs them.
