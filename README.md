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
- `F3`: show/hide developer overlay
- `F4`: toggle noclip
- `F5`: toggle distance fog
- `F6`: toggle minimap
- `F7`: toggle ray visualization on the minimap
- `` ` ``: open/close developer console
- `Esc`: quit

## Developer Console

Open the console with the backtick key. Gameplay input pauses while it is open. Enter one command and press `Enter`:

```text
tp <x> <y>       Teleport to the center of a walkable tile
speed <n>        Set movement speed multiplier (0.1–10)
fov <degrees>    Set field of view (30–120)
open_all         Open every door
close_all        Close every door
win              Complete the level
restart          Restart the level
help             List commands
```

The console retains the latest ten output lines. `F3` displays FPS, frame time, fixed-step count, player position/facing, tile underfoot and ahead, game state, elapsed time, steps, and active debug settings. Speed accepts `0.1`–`10`; FOV accepts `30`–`120` degrees. F7 displays 24 representative ray paths on the minimap; the renderer casts 240 rays per frame.

## Current shape

The renderer casts one ray per vertical screen strip against a grid map. Player movement checks walls and closed doors and slides along them by resolving each axis separately. Find and open both doors, explore the dead-end side room, and interact with the green exit. The completion screen reports elapsed time and tile steps; press `R` to restart.

Simulation input is represented as plain data and the world/player/game modules do not draw. Simulation updates run at a fixed 60 Hz; rendering remains frame-driven. Developer toggles and console state are isolated from gameplay state; noclip and speed are passed to the simulation as explicit movement settings. Command parsing is independent from rendering. Keep generation deterministic and content data-driven; add chunking, a thread pool, and editor tools only when the playable loop needs them.
