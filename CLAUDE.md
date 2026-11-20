# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Load the Bevy skill first

For any game development, debugging, or playtesting request in this checkout, load the
`bevy-game-dev` skill as the first action if it is not already loaded. The checkout
itself is the trigger: a request like "the ground still looks flat" need not mention
Bevy, Roadside-Picnic, or the skill by name.

If the skill is missing from discovery, read
`~/Projects/agentic/agentic-tooling/skills/bevy-game-dev/SKILL.md` if available.
If neither is available, continue with the project guidance below.

## Project Overview

**Roadside-Picnic** is a sci-fi game built in Rust with the [Bevy](https://bevyengine.org) game engine.
The setting is inspired by the Strugatsky brothers' novel *Roadside Picnic*: a mysterious, dangerous
area called **the Zone**, full of deadly **anomalies**, coveted **artifacts**, and the **stalkers**
who risk their lives to enter it and smuggle artifacts out.

Themes: exploration, survival, anomaly physics, artifact hunting, atmospheric dread.

## Controls

- **WASD** — walk (camera-relative); hold **Shift** to run
- **Space** — jump
- **Ctrl** (hold) — crawl prone
- **E** — enter or exit the armored vehicle; **W/S** to accelerate/reverse, **A/D** to steer, **Space** to brake
- **Right mouse drag** — orbit the camera; **mouse wheel** — zoom
- **Esc** — pause / resume
- **M** — mute / unmute game audio
- **Q** — quit

## Procedural content

- **Terrain** (`src/world/terrain.rs`) is generated deterministically (value-noise fBm,
  no external crates): rolling overgrown hills, a quarry with a raised lip, a scorched
  bald hill, a marshy lowland, and a dirt road to the quarry rim. Heights are sampled
  through the `Terrain` resource (`Terrain::height_at`) — never hardcode ground heights
  in other plugins. `tools/prepare_terrain_textures.py` extracts tiling albedo,
  normal, and packed AO/roughness/height maps from `terrain_texture_image_2.webp`
  (moss) and `terrain_texture_1.webp` (bare ground). The shader blends irregular
  patches over the terrain masks and uses the heights for shallow parallax.
- **Fauna** (`src/world/trees.rs`): four extracted, textured models
  (`assets/models/fauna/*.glb`) are scattered deterministically on a jittered grid,
  weighted by `Terrain::woodland_at` (off-road, outside the quarry, flat-ish, sparse
  on the bald hill). Rebuild them from the packed source with
  `tools/prepare_fauna_asset.py <source.glb> assets/models/fauna assets/textures/fauna`
  (run in headless Blender after `--`). It reuses the saved albedo, normal and
  roughness PNGs; `--boards <image_3> <image_2> <image_1> <image_0>` rebuilds
  512 px maps by combining swatches and close-ups from the reference sheets.
  The forest uses
  separate birch/petrified bark, and plants share the mud material at their bases.
  Texture cards, material balls and miniature LOD previews are excluded, and
  each extracted cluster is centered with its lowest point at ground level.
  Regenerate each collision pillar file with `tools/trim_tree_base.py` whenever
  the source geometry changes.
- **Base** (`src/world/base.rs`): the stalkers' home, an industrial catwalk yard
  (`assets/models/base/warehouse.glb`, Tripo, decimated like the trees) on a levelled
  pad at `terrain::BASE` on the south edge. Runs start at `SPAWN` inside it, and a
  `Respawn` message (sent when the stalker touches an anomaly) brings them back there.
  Seat the imported catwalk feet and floor props with `tools/ground_warehouse.py`
  before regenerating its pillars with `tools/trim_tree_base.py ... - 0 src/world/pillars/warehouse.rs`.
  `tools/prepare_hangar_textures.py` derives the slab, panels, steel, and trim
  textures from the four supplied hangar reference boards.
- **Armored vehicle** (`src/player/vehicle.rs`): parked in the hangar, driveable with
  independent wheel pivots. `tools/prepare_vehicle_asset.py` rebuilds the GLB with
  UVs and embedded material maps from four reference sheets. Regenerate
  `src/world/pillars/armored_vehicle.rs` with `tools/trim_tree_base.py` after mesh changes.
- **Collision** (`src/world/collision.rs`): every solid model carries a `Collider`, a
  set of upright `Pillar`s posed by its `GlobalTransform`; `push_out` moves bodies out
  of them. Pillars under `STEP_HEIGHT` are stepped over, ones above the body are passed
  under (crawling body is 0.5 m). Give every new solid model a `Collider`.
- **Audio** (`src/audio/`) is synthesized at startup: looping Zone ambience,
  gait-dependent footsteps, jump and landing, vehicle doors, and a speed-driven
  engine. Playback follows the system's default audio output.
- **Stalker model** (`assets/models/stalker.glb`) is a Tripo mesh auto-rigged in headless
  Blender by `tools/rig_stalker.py`, which also bakes the Crawl/Idle/Jump/Run/Walk clips
  (glTF animation indices 0–4, sorted by name). `src/player/animation.rs` cross-fades
  them by `Gait` and pitches the body face-down while crawling (the Crawl clip is posed
  standing). Edit motion in the script and regenerate rather than hand-editing the GLB.
- **Sky & lighting** (`src/world/mod.rs`) use Bevy's physical `Atmosphere` with a `SunDisk`
  lit at `lux::RAW_SUNLIGHT`; the camera runs HDR at `Exposure { ev100: 13.0 }`. Tune light
  and emissive values in physical units for that exposure, and steer the look with `SUN_DIRECTION`
  and `HAZE` rather than adding ambient light or distance fog.

## Build & Run Commands

```bash
# Development run (fast, unoptimized)
cargo run

# Release build (playable framerates — Bevy games should use release for real testing)
cargo run --release

# Type-check only (faster iteration than full builds)
cargo check

# Lint with project-wide defaults
cargo clippy -- -D warnings

# Format
cargo fmt

# Run tests
cargo test

# Update Bevy + plugins to latest compatible versions
cargo update
```

> **Always prefer `cargo check` during iterative edits.** It is several times faster than a full build.

### Hot reloading (development only)

Never enable these features in release or `wasm32` builds. Pass them on the command line rather than
adding them to the default features in `Cargo.toml`.

- **Assets:** the `file_watcher` feature makes the `AssetServer` watch `assets/` and reload textures,
  shaders, meshes, audio and scenes on save, with no restart. Run
  `cargo run --features bevy/file_watcher`. Only assets loaded from file paths reload; procedural
  terrain and synthesized audio are rebuilt only on restart.
- **Rust code:** the `hotpatching` feature patches edits made inside existing system bodies (logic,
  math, constants) into the running game. It uses Dioxus Subsecond via the Dioxus CLI:
  `dx serve --hotpatch --features bevy/hotpatching`. New types, component or resource changes, system
  signature changes and newly registered systems still need a normal rebuild.
- **Fast rebuilds:** add `bevy/dynamic_linking` (for example,
  `cargo run --features bevy/dynamic_linking,bevy/file_watcher`) to cut incremental compile times.

## Architecture Conventions

This project follows idiomatic Bevy ECS. Read these before adding code:

- **Systems are functions, not structs.** Prefer plain functions over implementing `System` trait.
- **Group systems into plugins** via `Plugin` impls (`GamePlugin`, `ZonePlugin`, `PlayerPlugin`, etc.).
  Each plugin owns its related components, resources, and systems.
- **States:** use Bevy `States` for top-level flow (`AppState::Loading` → `AppState::InGame`)
  and `SubStates` for gameplay overlays (`InGamePhase::Running`/`Paused`, toggled with Esc).
  Never use boolean resources for state. Pausing must never despawn the Zone.
- **Spawn objects in `OnEnter` schedules** rather than `Startup`, and despawn them in `OnExit`.
- **Define components in the plugin's module** as `#[derive(Component)]` structs, with `#[require(...)]`
  for required components instead of bundling.
- **Queries:** keep systems lean and precise — filter with `With`/`Without`, use `Query` data access
  over `Commands` mutations when possible.
- **Events** for one-shot communication (damage taken, artifact picked up, anomaly triggered).
  Avoid sending messages via resources.
- **Assets:** load via `AssetServer`, store handles in resources (e.g. `GameAssets` resource).
- **Physics/collision:** if a physics plugin is added, declare it in Cargo.toml and keep its systems
  isolated from game logic where practical.
- **WASM support is a goal.** Keep the crate compatible with `wasm32-unknown-unknown` builds:
  no `std::thread`/`std::net` in game code, gate platform-specific code with `#[cfg(target_arch = "wasm32")]`.

### Suggested module layout

```
src/
├── main.rs          # App entry point: default plugins + state setup
├── states.rs        # AppState + InGamePhase, state transitions, pause toggle
├── player/          # player controller, camera, inventory
├── zone/            # Zone generation, anomalies, artifacts
├── world/           # terrain, lighting, environment
├── ui/              # HUD, menus; ui/text.rs holds display strings
└── assets/          # asset paths as constants
```

## Code Style

- **No `unwrap()` in game logic.** Use `Option`/`Result` handling, or `expect()` with a message
  only where failure is impossible by construction.
- `cargo clippy -- -D warnings` must pass before declaring a change done.
- Name ECS systems `verb_noun` (e.g. `move_player`, `detect_anomaly_collisions`).
- Prefer small focused systems over monolithic ones; split when a system exceeds ~60 lines.
- Keep systems generic-free unless truly needed; Bevy's typed queries are enough.
- `use` statements: `std` first, then external crates, then `crate` modules, then `super` modules.
- Comments: `//` for explanations of non-obvious logic; avoid commenting obvious code.
- All text shown to players must go through localization-ready constants/resources, never hardcoded strings.

## Dependencies

- **Bevy is the only engine.** Do not add another engine or framework.
- Plugin choices should be minimal and justified — each added crate increases compile time.
- Keep Cargo.toml organized with comments grouping `[dependencies]`.

## Testing

- Unit-test pure logic (artifact pricing, inventory weight, anomaly damage formulas) with `#[cfg(test)]` mods.
- Integration smoke tests: spawn the `App` with plugins and step the schedule manually.
- No snapshot/golden tests for graphics.

## Git Workflow

- Commit messages in imperative mood, e.g. `add stalker movement system`.
- One logical change per commit.
- Do not commit `target/` or local asset dumps (see `.gitignore`).
