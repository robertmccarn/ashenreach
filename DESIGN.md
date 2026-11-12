# Ashen Reach — Design Document

**Project:** A first-person fantasy RPG built from scratch in Rust  
**Status:** M0–M3 prototype complete; M4 architecture/content foundation in progress
**Current target:** Windows development environment; Linux desktop remains a target to verify

## 1. Vision

Build a first-person role-playing game inspired by the scale and systems of *Daggerfall*: explorable dungeons and settlements, procedural locations, combat, items, quests, and a persistent character. The project is an original game. It will develop its own setting, names, maps, art, and writing.

The central constraint is to keep the engine specific to this game. Each technical feature should support a playable RPG milestone. Avoid building general-purpose engine systems before gameplay needs them.

## 2. Current Prototype

The current Rust binary opens a desktop window using Macroquad. It raycasts a small grid dungeon and supports first-person movement, turning, doors, an exit, and a completion screen. A developer overlay, minimap, ray visualization, noclip, and command console are present. A JSON content pipeline now loads the prototype map, sample items, enemies, loot tables, and encounters. These samples are data-model fixtures; enemies, inventory, quests, settlements, and campaign gameplay are not implemented.

Run with `cargo run` from the project directory. Controls: `W`/`S` move, `A`/`D` or arrow keys turn, `Q`/`E` turn, and `Esc` quits.

The current objective is to find the exit, open two doors, and reach the exit tile. The level includes a dead-end side room. Completion shows elapsed simulation time and tile steps, and `R` restarts the level. Combat, usable items, NPCs, quests, saves, audio, and procedural generation remain unimplemented.

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

Runtime code is split into `world`, `player`, `game`, `console`, `debug`, and `render` modules. `main.rs` translates Macroquad input into plain values and owns the fixed-step accumulator. Simulation modules use no drawing API; console parsing is pure and independent of rendering. This is a prototype structure, not a settled production architecture; future separation should follow actual gameplay needs.

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

## 12. Roadmap: Prototype to Retail Release

This roadmap describes product and content gates, not calendar estimates. Complete each gate with a working build and evidence that the feature works; do not treat the listed content quantities as a reason to generate low-quality filler. Content targets define a deliberately bounded indie RPG rather than a one-to-one recreation of *Daggerfall*'s enormous world.

### Current baseline — M0 to M3 (complete)

- **M0, renderer and controls:** Rust/Macroquad window, raycast grid, movement and turning.
- **M1, exploration objective:** fixed map, two doors, dead end, exit, win screen, elapsed time and tile-step count.
- **M3, developer tools:** hidden F3 overlay, F4 noclip, F5 fog, F6 minimap, F7 ray visualization, console parser and commands, ten-line output log.
- **Current content:** one small test map plus sample JSON item/enemy/loot/encounter definitions. These are pipeline fixtures, not playable RPG systems. No authored RPG narrative, characters, audio, persistent save, or production assets exist.
- **M2, simulation and test foundation:** fixed-step simulation and headless unit tests; this was present in the baseline but omitted from the original milestone list.
- **Baseline gate:** `cargo check`, `cargo clippy -- -D warnings`, and `cargo test` pass; desktop launch and controls are manually confirmed.

### M4 — Architecture and content foundation (in progress)

- **Completed in this workstream:** Serde DTOs, JSON content directories and schemas, sample definitions, O(1) indexes, strict parsing, cross-reference checks, map loading, startup validation, and transactional `reload_content` command.
- Define stable typed runtime IDs and separate immutable definitions from mutable instances before inventory or saves depend on them.
- Split simulation ownership by domain as needed; keep `Game` the deterministic orchestration root and add a simple typed event queue when the first cross-system interaction is implemented. Do not create an ECS or speculative framework.
- Resolve map/interaction edge cases and decide input policy (keyboard-only or mouse look), pause behavior, and supported platform baseline.
- Add a headless content validation entry point and CI checks; validate map connectivity and report actionable file/field errors.
- **Exit gate:** content validates from a clean checkout, reload failures preserve the last good set, runtime IDs are stable, and the first combat slice can use content without embedding definitions in simulation code.

### M5 — First combat vertical slice (next gameplay milestone)

- Implement health, damage, attack windup/recovery, hit detection, enemy awareness, pursuit, attack, death, and player defeat/recovery.
- Add one readable combat HUD and a small set of interactions: attack, use a healing item, loot a defeated enemy, and leave the encounter.
- Implement enemy line of sight and navigation on the current grid, with explicit behavior when paths are blocked.
- Add debug commands for spawning, healing, and resetting test encounters; keep deterministic test seeds.
- **Content level:** one 20–30 minute dungeon slice; 3 enemy archetypes (melee, ranged, durable); 2 weapon types; 1 healing consumable; 1 boss or elite encounter; 1 reward cache; a beginning, midpoint escalation, and exit. Use placeholder art/audio if needed, but finish and tune the loop.
- **Exit gate:** a new player can complete the slice; combat outcomes are readable; no enemy can trap the player or make the exit impossible; headless combat tests cover damage, death, rewards, and deterministic outcomes. Persistence is a later gate and must not be implied complete here.

### M6 — RPG character, inventory, and item systems

- Add a character sheet with a small set of understandable attributes and skills. Define how attributes affect movement, combat, defense, and dialogue/quest checks before expanding the list.
- Add inventory, equipment slots, item use, item comparison, merchants or other acquisition/sale flow, currency, weight/capacity rules if desired, and complete item descriptions.
- Add progression through experience or quest rewards, with a bounded level curve and respec/irreversibility decision documented.
- Design a minimal set of status effects and resistance rules; ensure every effect has UI feedback and a counterplay.
- **Content level:** 8–12 enemy archetypes; 25–40 useful items across weapons, armor, consumables, keys, and quest objects; 3 equipment tiers; 4 status effects; at least 2 viable character builds. Variants should change play meaningfully rather than only increase numbers.
- **Exit gate:** inventory, equipment, progression, item use, and vendor transactions survive save/load; balance tests cover early, middle, and late character power.

### M7 — Saves, data formats, and authoring pipeline

- Introduce versioned saves with atomic writes, backup/recovery, validation, and a visible load/new-game flow. Persist player, equipment, inventory, quest flags, location, world seed, and world changes.
- Move maps, items, enemies, dialogue, and quest definitions from Rust source into readable data files. Add a validator that reports file and field context for bad content.
- Add content IDs, schema versions, localization-ready text keys, and deterministic generation/content version fields.
- Add a headless content validation command and golden fixtures for save migration and generated maps.
- **Content level:** representative samples of every planned content type, including two saves that migrate across a schema change, a short quest with alternate completion states, and deliberately invalid content fixtures.
- **Exit gate:** the game loads a new or existing character, rejects invalid content safely, retains progress across restart, and can migrate supported earlier saves without silent data loss.

### M8 — Quest, dialogue, and narrative framework

- Implement a serializable quest state machine with prerequisites, objectives, branching choices, fail/expire conditions where appropriate, rewards, and journal history.
- Add dialogue interactions with choices, conditions, skill/attribute checks, and links to quest state. Separate authored text from code.
- Build static analysis for missing IDs, unreachable objectives, impossible prerequisites, missing dialogue targets, and quests with no valid terminal state.
- Decide and document the narrative tone, setting, factions, central conflict, player role, and the limits of procedural storytelling before large-scale writing.
- **Content level:** 5 finished quests (main, faction, investigation, dungeon, and social examples), 10 named NPCs, one short dialogue hub, and one branching quest tested across every outcome. This is a writing/style pilot, not placeholder volume.
- **Exit gate:** quests can be completed, failed, saved, loaded, and resumed; validators catch intentionally broken examples; dialogue remains comprehensible without relying on unstated player knowledge.

### M9 — World travel and deterministic location generation

- Define the world map as authored regions connected by roads, routes, waterways, and travel points. Keep critical story geography authored and use generation for repeatable side content.
- Implement seeded dungeon generation with room/corridor grammar, required entrances/exits, encounter pacing, loot budgets, and validation for connectivity and solvability.
- Add location identity and generation-version rules so an existing save resolves to the same place after updates. Store player-made changes as deltas only after the generated base is stable.
- Add town-block generation only if it supports the authored campaign; do not make critical NPCs, quests, or services depend on unconstrained generation.
- **Content level:** 3 distinct regions; 2 dungeon themes; 10 handcrafted benchmark dungeons; a generated catalog capable of producing many validated variants; 3 travel routes with landmarks and encounter rules. Build a curated seed set that demonstrates the full range and a large automated seed-validation set.
- **Exit gate:** seeds reproduce results; generated locations pass connectivity, spawn, encounter, and exit checks; hand-authored story locations remain stable; loading a save does not regenerate its visited places unexpectedly.

### M10 — Campaign and world content production

- Write and implement the complete main quest with a clear opening, escalating middle, climax, resolution, and credits. Ensure the player can finish without relying on random content.
- Add factions with distinct motivations and quest arcs; define mutually exclusive outcomes and any reputation consequences.
- Add towns, interiors, NPC routines or schedules where they improve the world, and side content that reinforces each region's identity.
- Use content templates for reusable low-stakes tasks, while reserving authored quests for major characters and consequences. Generated content must not impersonate authored plot-critical events.
- **Retail content target:** 3–4 regions; 1 capital and 5–7 smaller settlements; 20–30 authored dungeons/major sites plus validated procedural variants; 35–50 completed quests (about 10–15 main-story, 10–15 faction, and the remainder side/exploration); 20–30 named recurring NPCs plus reusable background population; 15–20 enemy archetypes; 60–100 meaningful items; 3–5 dungeon/environment themes. Scope should support a complete main campaign and substantial optional play, with an intended 15–25 hour main path and more for completionists.
- **Content quality bar:** every main quest and faction quest receives an editorial pass, end-to-end playthrough, state-branch check, reward/balance pass, and localization-ready text review. No quest is counted as complete merely because its data file exists.
- **Exit gate:** all required story content is present and completable; every region has a distinct gameplay and visual identity; no critical path depends on random generation or missable content without warning.

### M11 — Final art, audio, UI, and accessibility production

- Replace prototype colors and primitive geometry with a coherent, original visual language: environment materials, doors, props, enemy silhouettes/animations, UI icons, and readable interaction cues.
- Establish an audio identity with ambience, movement, interactions, combat feedback, enemy cues, music themes, and mix controls. Provide silence-safe play and avoid audio-only critical information.
- Finish title screen, new/load flow, settings, keybinding UI, inventory, character sheet, quest journal, map, dialogue, pause, death, completion, and credits screens.
- Add accessibility options appropriate to the design: text size, contrast, color-safe cues, camera sensitivity, motion reduction where possible, input alternatives, and subtitle controls.
- **Content level:** final assets for every production location/theme and enemy archetype; complete UI icon set; authored sound/music coverage for all gameplay states; full credits and third-party asset/license inventory. Avoid temporary placeholder art/audio in release-critical content.
- **Exit gate:** visual and audio readability holds across supported resolutions and settings; accessibility options persist and are tested; all shipped assets have known provenance and rights.

### M12 — Alpha: feature and content complete

- Freeze the feature list. Only fix defects or make explicitly approved scope changes after this gate.
- Integrate all campaign, RPG, save, settings, UI, audio, and debug systems into one start-to-credits build.
- Run complete playthroughs with fresh saves and migrated saves; cover alternate quest branches, deaths, inventory limits, vendor flows, travel, dungeon seeds, and ending states.
- Profile representative minimum and target hardware. Set measurable frame-time, memory, load-time, and save-time budgets; remove debug-only overhead from release builds.
- **Content level:** all retail-target content implemented, even if some assets remain marked for polish. No missing main quest, faction arc, ending, tutorial, credits, or required UI screen.
- **Exit gate:** full game can be completed from a clean install; no known progression blocker; all content validates; release scope and supported platforms are locked.

### M13 — Beta: polish and compatibility

- Address usability, balance, performance, compatibility, accessibility, save integrity, and content defects. Add no large new systems.
- Conduct external playtests with players unfamiliar with the project; observe navigation, combat understanding, quest comprehension, settings discoverability, and completion without coaching.
- Test clean install, upgrade install, uninstall/reinstall, save backup/recovery, corrupted saves, controller/keyboard paths if supported, window resize/fullscreen, and supported OS/GPU configurations.
- Run automated seed sweeps, quest-state validation, save migration tests, and repeatable regression scenarios in CI or an equivalent local gate.
- **Content level:** final text edit, proofread, subtitle review, final encounter balance, final credits, and complete localization of the declared launch languages. Cut or simplify content that cannot meet the quality bar; do not ship broken volume to hit a count.
- **Exit gate:** no known blocker or critical defect; high-severity defects have an accepted fix or documented resolution; save compatibility is verified; performance targets are met on declared minimum hardware.

### M14 — Release candidate and retail delivery

- Produce versioned, reproducible release builds for each declared platform and architecture. Sign packages where the platform supports signing.
- Prepare installers/packages, first-run experience, default settings, save locations, crash logs with privacy controls, and an offline-capable play path.
- Complete store assets and product copy, screenshots/trailer capture, system requirements, accessibility notes, support contact, privacy disclosure, EULA/third-party notices as required, and final credits.
- Verify storefront build IDs, depot/package contents, update path, rollback/previous-save safety, and the final downloadable package from a clean user account.
- **Content level:** all localized and credited launch content included; no development console, cheat commands, test maps, debug overlays, or unreleased assets exposed in the retail configuration unless deliberately retained as documented options.
- **Exit gate:** release candidate installs, launches, saves, loads, updates, and completes on every declared platform; package contents and legal/asset records are approved; the release build is traceable to source and build instructions.

### M15 — Launch and post-launch stabilization

- Monitor crashes, save failures, performance reports, and progression blockers. Provide a clear support and issue-triage route.
- Ship focused stability fixes with regression coverage and save compatibility checks. Keep patch notes and supported-version policy current.
- Decide post-launch additions only after the shipped game is stable; do not treat future content as a substitute for launch completeness.
- **Content level:** day-one fixes and essential localization corrections only; expansions or new regions are separate, fully scoped projects.
- **Exit gate:** critical launch issues are resolved, supported saves remain safe, and the team can maintain the shipped build reproducibly.

## 13. Retail Definition of Done

Ashen Reach is retail-shippable when all of the following are true:

- A player can install the game, start a new character, understand the controls and core systems, complete the main story, see a resolved ending/credits, quit, and later resume a valid save.
- The declared retail content target is complete and has passed content validation, editorial review, and full-path playthroughs.
- No known issue blocks installation, startup, saving/loading, a main quest, a required location, or an ending on supported configurations.
- Settings and accessibility options work and persist. Input, UI, text, audio, and rendering remain usable on supported hardware and display sizes.
- Builds are reproducible and versioned. The release package contains only intended assets and includes required credits, notices, system requirements, and support information.
- Automated tests cover critical simulation and data rules; manual QA covers full playthroughs, content branches, compatibility, and the release package.

## 14. Production Gates and Scope Control

- Every milestone ends with a playable build and a written acceptance result. Keep simulation tests headless wherever practical.
- Build one representative example of a system, validate the workflow, then scale content. Do not author dozens of quests before quest validation and save behavior are stable.
- Freeze or reduce content counts when quality, accessibility, or main-path completion would otherwise suffer. A smaller complete game is preferable to a larger unfinished catalog.
- Distinguish authored critical-path content from generated repeatable content. Procedural generation expands replay value; it does not replace authored pacing, characters, or resolution.
- Track content by status: draft, implemented, validated, playtested, edited, final, and localized. Count only final, verified content toward release readiness.
- Keep a release checklist for clean install, settings, saves, crash handling, performance, licenses, credits, localization, store package, and support.

## 15. Main Risks and Responses

- **Engine work displaces game work:** tie engine changes to the next playable gate and keep the grid raycaster until a real content requirement exceeds it.
- **Content target exceeds production capacity:** protect the main story and quality bar; reduce optional volume before cutting testing, accessibility, or the ending.
- **Procedural generation creates repetition or broken progression:** constrain generation to validated grammars and curated themes; keep critical characters and story beats authored.
- **Save updates break player progress:** version the format early, test migrations and corrupted-file recovery, and retain a recoverable previous save.
- **Input, UI, or rendering excludes players/configurations:** define supported platforms and minimum hardware; test settings and accessibility throughout production, not only in beta.
- **Unlicensed or unclear assets block release:** track asset source, author, license, attribution, and redistribution rights as content enters the project.
- **Retail polish is postponed until feature completion:** include accessibility, final UI, credits, settings, packaging, and QA as explicit release gates rather than optional cleanup.

## 16. Audit Reconciliation and Immediate Implementation Plan

The November 12, 2026 repository audit was written against baseline commit `d571b91` and predates the current content-pipeline changes. Its assessment of the prototype and absent RPG gameplay remains useful. Its statement that no external content loader exists is stale: maps, items, enemies, loot tables, and encounters now have JSON DTOs, schemas, sample data, startup validation, cross-reference checks, and a console reload command. The sample enemies and items are not yet connected to gameplay. The audit's recommendation to build M4 architecture/content foundations before M5 combat is the sequencing authority for work from this point; the earlier M4 movement foundation / M5 combat text has been replaced accordingly. Retail readiness percentages in the audit are directional judgments, not project metrics; use milestone gates and playable evidence as the progress measure.

### Execution plan and acceptance gates

This plan follows the engineering instructions for M4 completion and one M5 combat slice. It does not include full inventory UI, campaign systems, save/load, or content-volume production. Keep each stage reviewable, preserve the current Macroquad renderer, and do not mark a stage complete until its behavior is integrated and verified.

#### Stage A — Content contract and validation

- Add a shared content manifest with an explicit content-format version; reject missing or unsupported versions. Keep this independent of future save-format versions and defer migrations until a real format transition is needed.
- Keep the handwritten JSON Schemas and DTOs for now; validate every shipped definition against its matching schema in tests. Enforce `deny_unknown_fields`, required fields, numeric ranges, and Rust-side cross-references. Document schema changes and add fixtures/tests that catch drift.
- Validate rectangular maps, legal glyphs, exactly one start, at least one exit, and exit reachability. Treat doors as traversable for solvability because the player can open them; report disconnected exits with map ID and coordinates. Reuse one connectivity implementation.
- Add invalid fixtures for malformed JSON, unsupported schema version, unknown fields, duplicate IDs, missing references, invalid numeric values, malformed maps, and unreachable exits. Error messages should name the source file and field or map coordinate where available.
- **Gate:** valid shipped data passes schema and semantic validation; each invalid fixture fails with an actionable diagnostic; content validation remains callable without graphics.

#### Stage B — Runtime loading, reload, and developer workflow

- Resolve content paths from an explicit development root and the executable/package resource location, with a clear error naming attempted paths. Keep path policy outside simulation.
- Build candidate content completely before swapping it into the active manager. On any parse/schema/semantic failure, retain the previous manager and game state. Restart the player only after a fully successful reload.
- Route both console restart and `R` through one game-reset function built from the active content definitions.
- Add `cargo run -- validate-content`; it must exit before Macroquad initialization, print diagnostics, and return failure status for invalid content. Reuse the same loader/validator used at startup and reload.
- **Gate:** run the command from repository root and a different working directory; prove failure leaves the last good content and active game unchanged, and success loads the replacement map and restarts once.

#### Stage C — CI and project contracts

- Add GitHub Actions for pushes and pull requests to the primary branch: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets`, and `cargo run -- validate-content`.
- Introduce human-readable newtype IDs for item, enemy, encounter, map, and loot table references. Parse serialized IDs at the content boundary; use typed IDs in simulation APIs and keep filenames unrelated to identity.
- Keep authored definitions immutable. Add runtime instances only for mutable state: enemy health/position/state and item quantity/durability/runtime identity. Instances hold typed definition IDs rather than cloned definitions.
- Keep `Game` the deterministic simulation orchestrator. Document update order. Add domain modules only when they own real behavior; use a small typed event queue when combat must report defeat/reward across systems.
- **Gate:** all CI commands pass; invalid IDs fail safely; runtime state references definitions without mutating or duplicating them; simulation tests need no window.

#### Stage D — Headless combat implementation

- Create combat state and rules in simulation-only Rust. Add player health, attack input, JSON-defined weapon damage, one JSON-defined enemy, defense-aware damage, enemy death, player defeat, encounter completion/reset, and one consumable that heals up to maximum health.
- Spawn composition from the existing encounter JSON. Generate the reward from the enemy's referenced JSON loot table using a deterministic seed; prevent dead enemies from attacking or rewarding more than once.
- Implement enemy awareness and pursuit on the existing grid. Start with deterministic grid pathfinding; closed doors and walls block movement. Repath or wait safely when a route is blocked. Attack only in range, with a defined cooldown.
- Keep balance constants and authored stats/data in definitions. Any unavoidable rules constants (cooldowns, reach, seed policy) belong in documented simulation rules, not duplicated content.
- Emit observable combat results for hit/miss, damage, death, defeat, healing, and reward so UI and tests can report outcomes without owning combat logic.
- **Gate:** tests cover attack damage, defense, enemy attack and range, death idempotency, loot table selection, healing limits, collision/pathfinding, defeat, deterministic repeatability, and encounter reset.

#### Stage E — Integrated playable encounter

- Build one compact encounter with an opening approach, a combat escalation, a useful reward, and an obvious exit. Use the existing encounter data format and a repeatable starting state; do not add large placeholder catalogs.
- Add only the HUD and placeholder feedback needed to communicate attack timing, hits and damage, enemy death, received damage, healing, reward, and player defeat. Rendering reads simulation state/events.
- Perform a human playthrough to confirm the encounter is completable, the controls and outcomes are understandable, and resetting produces the same encounter. Record the acceptance result and known limitations.
- **Gate:** one complete combat loop runs in the desktop build, all automated gates pass, and the human playthrough confirms completion and reset.

After Stage E, the next milestone is inventory and versioned persistence design, followed by broader RPG progression. Do not fold those systems into this assignment. The schedule is intentionally expressed as gates rather than week estimates; revise effort from the measured implementation and playtest results.
