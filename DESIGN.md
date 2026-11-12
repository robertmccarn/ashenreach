# Ashen Reach — Design Document

**Project:** A first-person fantasy RPG built from scratch in Rust  
**Status:** Milestone 3 complete; a short desktop dungeon objective and developer tools are available
**Current target:** Linux desktop, with portability kept in view

## 1. Vision

Build a first-person role-playing game inspired by the scale and systems of *Daggerfall*: explorable dungeons and settlements, procedural locations, combat, items, quests, and a persistent character. The project is an original game. It will develop its own setting, names, maps, art, and writing.

The central constraint is to keep the engine specific to this game. Each technical feature should support a playable RPG milestone. Avoid building general-purpose engine systems before gameplay needs them.

## 2. Current Prototype

The current Rust binary opens a desktop window using Macroquad. It raycasts a small, fixed grid dungeon and supports first-person movement and turning. Movement is checked against walls one axis at a time, allowing the player to slide along them.

Run with `cargo run` from the project directory. Controls: `W`/`S` move, `A`/`D` or arrow keys turn, `Q`/`E` turn, and `Esc` quits.

The current objective is to find the exit, open two doors, and reach the exit tile. The level includes a dead-end side room. Completion shows elapsed simulation time and tile steps, and `R` restarts the level. The map is a fixed grid; enemies, combat, items, NPCs, quests, saves, audio, and procedural generation remain out of scope for this milestone.

## 3. Technical Choices

- **Language:** Rust, using stable toolchain features and Cargo.
- **Window, input, and rendering:** Macroquad. It provides a small cross-platform desktop layer without requiring an installed SDL or Vulkan SDK.
- **Rendering approach:** Start with the current grid raycaster. It fits the low-resolution, low-poly visual target and makes dungeon iteration quick. Revisit a polygon renderer only if the design requires geometry the raycaster cannot represent.
- **World randomness:** Use an explicitly selected, seeded PRNG for all generated content. Do not depend on platform-specific default random streams where save compatibility or reproducibility matters.
- **Persistence:** Versioned save format; store player changes and generated-world seeds. Add world deltas only after stable procedural generation exists.
- **Dependencies:** Keep the dependency list small. Add libraries to solve a current feature, not speculatively.

## 4. Design Principles

1. Reach a playable loop early: explore, encounter danger, fight, find a reward, and return safely.
2. Keep simulation data separate from drawing and input so it can run headlessly in tests.
3. Make generation deterministic: the same seed and content version should produce the same location.
4. Prefer simple data structures—typed records and vectors—over a general ECS until entity complexity demonstrates a need.
5. Make collision and save compatibility explicit and testable.
6. Build debug visibility early: show position, seed, collision, and entity state before adding content volume.

## 5. Runtime Architecture

The application is organized as small modules with one-way dependencies:

```text
application
  ├── platform       window, frame timing, keyboard/gamepad input
  ├── simulation     player, world state, entities, combat, quests
  ├── world          map data, deterministic generation, collision queries
  ├── renderer       raycasting, sprites, UI drawing
  ├── audio          effects and music
  ├── save            versioned serialization and migration
  └── debug           overlays, commands, deterministic repro information
```

Runtime code is split into `world`, `player`, `game`, `console`, `debug`, and `render` modules. `main.rs` translates Macroquad input into plain values and owns the fixed-step accumulator. Simulation modules use no drawing API; console parsing is pure and independent of rendering.

The main loop should use a fixed simulation step (target 60 Hz) with rendering decoupled from simulation. Clamp large frame deltas. Start with a single-threaded update loop; add background jobs only when world generation or rendering work is measurably costly.

## 6. World and Rendering

### 6.1 Dungeon Prototype

Represent the dungeon as a two-dimensional grid with explicit `Floor`, `Wall`, `Door { open }`, and `Exit` tile kinds. The raycaster queries this grid and renders wall strips with distance shading. Closed doors block movement and rays; open doors allow both through. The exit is passable and visibly distinct.

### 6.2 Generated Locations

Generation functions should be pure where practical:

```text
generate(location_id, seed, content_version) -> Location
```

The output contains map geometry, entrances, entity spawn definitions, and loot placements. Validate connectivity and required exits before accepting a generated location. Begin with small handcrafted test seeds, then add seed sweeps to catch invalid layouts.

### 6.3 Visual Direction

Aim for readable retro first-person visuals: restrained palette, nearest-neighbor textures, strong silhouettes, and atmospheric lighting. Start with flat shaded walls. Add texture atlases, billboard sprites, fog, and limited dynamic lights as needed. Preserve legibility over exact imitation of any one game.

## 7. Player and Collision

Player state should include world position, facing, movement velocity if needed, health, and interaction state. Input maps to intent; the simulation applies movement and collision.

The current axis-separated grid collision is a temporary foundation. Before adding jumping or complex slopes, define the intended movement model and extend automated movement tests. Important properties include:

- the player cannot enter solid cells;
- movement along one axis remains possible when blocked on the other;
- spawn points are valid and reachable;
- repeated movement cannot escape map bounds.

If collision becomes geometric, use a swept circle/capsule and spatial partitioning suited to the generated rooms. Do not add a general physics engine without a concrete need.

## 8. Gameplay Systems

Implement systems in dependency order:

1. **Exploration:** maps, collision, doors, interact action, transitions between locations.
2. **Interaction:** examine objects, talk to NPCs, show dialogue and contextual prompts.
3. **Combat:** one enemy, player attack, enemy response, health, defeat and reward.
4. **Items:** pickups, inventory, equipment, consumables, and item descriptions.
5. **Character progression:** attributes, skills, experience, and leveling.
6. **Persistence:** save/load player state and location; validate corrupted or old saves.
7. **Quests:** data-driven objectives and dialogue choices with serializable state.
8. **World scale:** connected dungeons, towns, travel, and eventually an overworld.

Keep each milestone end-to-end playable. Avoid broad content expansion before its underlying loop is enjoyable and stable.

## 9. Content and Scripting

Keep maps, item definitions, enemy definitions, and quests in external data files once formats are stable. For early development, use simple human-readable formats and validate them at load time. A scripting language is not a starting requirement. First determine whether quest logic fits a small data-driven state machine; embed Lua only if the authoring needs justify the extra runtime and tooling.

All content references should be validated before play: missing map exits, invalid item IDs, missing dialogue targets, and unreachable quest states should report actionable errors.

## 10. Save Data and Determinism

Save files should include a format version, game/content version, world seed, player state, and changes from generated defaults. Loading must reject malformed data safely and preserve the previous save where possible.

Random choices should use named deterministic streams or derived seeds so adding a cosmetic random call does not silently change dungeon layouts. Record the seed and relevant generation version in debug output and bug reports.

## 11. Tooling and Validation

Prioritize tools in this order:

1. Debug overlay with position, facing, frame time, current map, and seed.
2. Simple in-game commands for teleporting, restoring health, and spawning test enemies.
3. Headless simulation mode for deterministic world and quest validation.
4. Replayable input traces for reproducing movement and combat bugs.
5. Hot reload and editors only after content formats settle.

Validation should focus on behavior: map connectivity, collision bounds, deterministic generation, combat state transitions, save round trips, and content-reference validation. Keep unit tests for these systems independent of opening a window.

## 12. Milestones

### M0 — Window and dungeon view (complete)

- Rust/Cargo project builds.
- Macroquad desktop window opens.
- Fixed grid is raycast into a first-person view.
- Player can turn and move while blocked by walls.

### M1 — Doors, exit, and completion (complete)

- Add a separate game-state module and typed door/exit tiles.
- Add Space/F interaction, nearby prompts, and distinct door/exit colors.
- Build a fixed map with two doors, a dead end, and a reachable exit.
- Show completion time and steps; allow restart with `R`.
- Verify collision, interactions, map reachability, and deterministic fixed-step simulation with unit tests.

**Decisions:** Count a step when the player crosses into a different grid cell. Completion time counts fixed simulation time, excluding time spent on the completion screen. Doors toggle open/closed and remain in their current state until the level restarts. The exit is traversable; interacting with it completes the level.

### M1.1 — Robust exploration polish

- Add a debug overlay with frame time and target tile.
- Add further collision edge-case coverage and map authoring validation.
- Keep simulation independent of Macroquad drawing and window state.

### M3 — Debug overlay and developer console (complete)

- Add a hidden-by-default F3 stats overlay with FPS, frame time, fixed-step count, player, tile, game, and movement data.
- Add F4 noclip, F5 fog, F6 minimap, and F7 minimap ray visualization.
- Add a backtick command console with teleport, speed, FOV, door, win, restart, and help commands.
- Keep the latest ten console lines visible while the console is open or overlay is shown.
- Keep command parsing pure, settings in `DebugState`, and simulation overrides passed explicitly.
- Validate command parsing, teleport bounds, door batches, noclip, and deterministic updates in tests.

**Decisions:** Teleport arguments identify grid tile coordinates and place the player at the tile center; walls, closed doors, and out-of-bounds targets are rejected. Speed is clamped by validation to 0.1–10× and FOV to 30–120 degrees. Opening the console pauses simulation input and elapsed-time advancement. F7 enables the minimap automatically. Debug ray visualization draws 24 representative rays; the overlay reports the renderer's full ray count.

### M4 — First complete RPG loop

- Add one enemy with a visible billboard sprite and simple behavior.
- Implement attack, damage, death, and a reward pickup.
- Add health and a minimal inventory screen.
- Make a short dungeon completable from entrance to exit.

### M5 — Persistence and authored content

- Add versioned save/load.
- Move dungeon and enemy definitions into validated data files.
- Add dialogue and a small quest with persistent state.

### M6 — Deterministic procedural dungeon

- Generate room-and-corridor layouts from a seed.
- Guarantee entrance-to-exit connectivity and valid spawn positions.
- Add batch seed validation and deterministic regression fixtures.

### M7 — Town and broader world

- Add a compact town with enterable buildings and NPC interactions.
- Add travel between locations and a world map representation.
- Expand quests and progression only after the dungeon/town loop holds up.

## 13. Risks and Scope

- **Engine drift:** keep each milestone tied to a playable feature; defer generic systems.
- **Collision complexity:** specify movement before adding vertical movement or slopes, and test generated spaces at scale.
- **Content burden:** use original, reusable data-driven content and build validation before volume.
- **Save breakage:** version serialized data from the first persistent milestone.
- **Rendering limits:** keep the raycaster until a specific game feature requires a different representation.
- **Schedule:** a solo, content-rich RPG is a multi-year effort. Treat estimates as planning ranges, not promises; validate scope with small vertical slices.

## 14. Immediate Next Work

1. Begin the first enemy and combat vertical slice, keeping its simulation testable without rendering.
2. Keep validating authored map and content edge cases as systems expand.
