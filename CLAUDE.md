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

The upper-right in-game HUD shows the smoothed frame rate.

## Controls

- **Any key** — enter the Zone from the hangar splash once loading finishes
- **WASD** — walk (camera-relative); hold **Shift** to run
- **Space** — jump
- **Ctrl** (hold) — crawl prone
- **E** — enter or exit the armored vehicle; **W/S** to accelerate/reverse, **A/D** to steer, **Space** to brake
- **Right mouse drag** — orbit the camera; **mouse wheel** — zoom
- **Esc** — pause / resume
- **M** — mute / unmute game audio
- **Q** — quit

## Procedural content

- **Terrain** (`src/world/terrain.rs`) is a deterministic 3.44 km square built in
  128 m chunks: mountains, rolling hills, the old quarry and bald hill. The
  graded dirt network (`src/world/roads.rs`) leads from the hangar checkpoint
  to the quarry, bald hill, swamp overlook, lakeshore, river fords, and outlying sites.
  The same network keeps vegetation off the lanes and paints a world-space
  gravel and tire-rut mask independent of terrain LOD. Heights are sampled
  through the `Terrain` resource (`Terrain::height_at`) — never hardcode ground heights
  in other plugins. `Terrain::mesh_height_at` tracks the rendered surface near
  the player. Ground tiles swap from 2 m to 8 m sampling at distance, with
  stitched edges across both resolutions. Larger 16 m horizon tiles keep the
  coastal water, lake basin and rolling country visible beyond the traversal
  limit without nearby collision or plant detail.
  `tools/prepare_terrain_textures.py` extracts tiling albedo,
  normal, and packed AO/roughness/height maps from `terrain_texture_image_2.webp`
  (moss) and `terrain_texture_1.webp` (bare ground). The shader blends irregular
  patches over the terrain masks, uses the heights for shallow parallax, and adds
  snow on high, gentler slopes.
- **Water** (`src/world/{hydrology,water}.rs`): the main river feeds a lake in
  the northwest and joins a northern tributary; the nearby swamp has shallow pools
  between drier hummocks. Use `Terrain::water_depth_at` for traversal. Walk and
  drive through the shallows; deep water stops you. The main dirt track crosses
  at a vehicle-depth ford.
- **Ground fog** (`src/world/ground_fog.rs`): a deterministic, scrolling
  128x24x128 density volume forms variable low cotton-like banks inside a
  10 m slab centered one meter above the player's rendered ground height. The
  camera uses 48-step volumetric fog plus a far-weighted, regional 10–16-tile
  distance curve, with the far plane fixed at 16 tiles. The terrain shader
  masks that distance fog to low ground and leaves automatic Bevy fog disabled,
  so the cloud bank thickens toward the horizon without washing the ridgelines;
  bird materials also disable classic distance fog above the bank.
- **Scenery** (`src/world/scenery.rs`): procedural pine groves and textured rock
  outcrops are drawn in chunk batches. Imported saplings, broadleaves, tall
  conifers, birches, and dead trees (`assets/models/trees/`) are reserved for
  the hangar and named landmark groves. Their GLBs use one ~500-triangle primitive each: simplified
  textured wood and, for living trees, cutout crown faces baked from the
  supplied models.
  `src/world/canopy.rs` fits the roots and trunks, limits
  shadows to nearby trees, and sways some living trees without moving their
  colliders. Individual trees can randomly snap and fall while in view; their
  collision goes away immediately and they don't respawn during the run.
  Colliders and undergrowth stream in a 3×3 chunk neighborhood.
  Camera frusta and distance ranges cull scenery detail independently of
  collision and leave the terrain skyline visible.
- **Plants** (`src/world/plants.rs`): dry grass and thornbushes favor dry ground,
  shrubs favor woodland, and hogweed and emissive glowweed favor damp ground.
  Each dry-grass placement seeds a patch of up to ten procedural plants, while
  bushes and weeds seed up to 100 and spread across a proportional radius. The
  root of every shrub or thorn patch uses an imported PBR model: byte-identical
  4K UHD shrub/thornbush GLBs render within 8 m, then crossfade to 4K-textured
  medium variants through about 70 m. The remaining patch plants use small,
  frustum-culled mesh batches as chunks stream in, not an
  entity per plant. Ground cover has no collision or shadows. The rear and
  sides of the hangar form a dense shelterbelt; the central yard and roads stay
  clear.
- **Fauna** (`src/world/trees.rs`): four extracted, textured vegetation clusters
  (`assets/models/fauna/*.glb`) are scattered only around landmarks, on a jittered grid,
  weighted by `Terrain::woodland_at` (off-road, outside the quarry, sparse above
  the treeline and on the bald hill). Rebuild them from the packed source with
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
- **Cordons** (`src/world/cordon.rs`): the enclosed hangar yard has a level
  staging area, with the checkpoint on the west side of a 16 m opening on the
  existing north road. A separate continuous wall marks the Zone boundary;
  the panels follow the rendered terrain and stay above the water at river
  crossings. Wall meshes and solid collision stream in nearby terrain tiles.
  Keep the staging area and driveway clear; the tree and bush shelterbelt is
  concentrated behind the hangar and along the sides.
  For `assets/models/sites/`, simplify the wall with glTF Transform `simplify
  --ratio 0.24 --error 0.016`; resize the wall to 1536 px and the checkpoint to
  2048 px, bake tangents with `--vertex-layout separate`, then ground with
  `tools/prepare_plant_asset.py`. Re-sample `src/world/pillars/site_checkpoint.rs`
  with `tools/trim_tree_base.py` after editing the checkpoint geometry.
- **Sites** (`src/world/sites.rs`): two placements each of the supplied brick
  factory, cemetery, garage, plague quarter, schoolhouse, silos, and
  site-substation. Four railway embankments span the map quadrants, and three
  landmark-substation copies are isolated across the Zone. The other four
  supplied landmarks (`assets/models/landmarks/`) each have one sparse copy.
  The landmark centers are a fixed-seed scatter with at least 400 m separation.
  A 40 m train is grounded, yawed and listed beside the southeast embankment.
  Level pads keep foundations grounded and scenery clear; collision streams
  with nearby terrain and landmark groves use the existing canopy budget. Site
  approaches face the model's +X frontage, while the broader supplied landmarks
  use their +Z frontage.
  Resize their albedo to 1536 px, bake tangents with `--vertex-layout separate`,
  and ground with `tools/prepare_plant_asset.py`. Trim the factory's display
  plane with `tools/trim_tree_base.py` at `0.105` of grounded height before
  final tangents. Use the same tool to sample collision at cut `0`, passing
  `--split-columns` for the garage and landscape landmarks so roofs and
  overhangs do not fill open bays and sight lines. Supplied vehicles in
  `assets/models/vehicles/` use the same 1536 px texture, grounding, tangent
  and split-column collision pipeline.
- **Wildlife** (`src/world/wildlife.rs`): the optimized versions of the
  textured sources in `assets/models/{animals,birds}/` share mesh handles
  and preserve the original PBR maps. Rebuild with glTF Transform `simplify
  --ratio 0.0015 --error 0.009`, `resize --width 1024 --height 1024`, and
  `tangents` (all with `--vertex-layout separate`), then ground and repair
  tangent seams with `tools/prepare_plant_asset.py`.
  `src/world/wildlife/rig.rs` skins the static meshes with shared per-species
  rig weights measured in +Z-forward model space. Walking and running feet
  follow the terrain, and flyers flap or glide. Near the hangar, the raven
  lands, walks, and takes off again while the frostwolf occasionally chases
  and lunges at the hexadeer. The chameleon stays on its modeled perch.
  Every named site and landmark receives a deterministic local fauna route:
  ground creatures, mutants, and flying birds rotate by site index. Walkers
  stay outside the site's pad and road. Each base flight route fans into a
  five-bird flock with staggered phase, radius, and altitude while preserving
  landing routes and terrain clearance. Wildlife is otherwise decorative and
  has no collision.
- **Artifact showcase** (`src/world/base/artifacts.rs`): all 14 supplied
  artifacts sit on individual lit pedestals in two seven-table rows inside the
  hangar. The west row stays outside the aisle and the east row hugs the wall,
  leaving the armored vehicle's diagonal bay route clear. Every artifact keeps
  a stable rest transform and idly spins, bobs, and sways; display-table
  colliders remain static. When the player approaches a table, the HUD resolves
  its `ArtifactDisplay` index through `ui::text::ARTIFACT_COPY` and shows the
  localized name and lore; the readout hides away from range and while paused.
  The golden sphere retains its textured gold albedo
  but receives a fully metallic, low-roughness runtime material so its surface
  reflects the hangar light probe and spotlights. Artifact runtime GLBs keep the
  original geometry, 4K texture maps, and source PBR/normal materials. Centered
  sources are grounded by `tools/ground_full_asset.py`; already-grounded dense
  PBR scans are copied byte-for-byte. The six dense scans also have distant
  preview LODs in `assets/models/artifacts/lod/`; within 8 m the preview is
  hidden and the untouched full PBR scene is streamed in. Do not replace the
  primary showcase artifact with an LOD. Bevy's glTF loader generates MikkTSpace
  tangents when a normal map needs them.
- **Mutants** (`src/world/wildlife/mutants.rs`): bovines, chimeras, fleshdeer,
  and mutts occupy dry, biome-weighted patrol pockets across the Zone. Bovines
  give the stalker room; fleshdeer investigate up close; mutts and chimeras
  pursue on foot, brace before lunging, and retreat to their patrols. Site-local
  mutant patrols use the same pad, water, road, spacing, and leash checks. A landed
  strike respawns the stalker. The cordoned hangar yard and the occupied vehicle are safe.
  The original textured models use the shared wildlife rig, with species-specific
  foot and head weights. Rebuild the GLBs in `assets/models/mutants/` with the
  wildlife pipeline above, using `simplify --ratio 0.003 --error 0.009`.
- **Base** (`src/world/base.rs`): the imported steel hangar
  (`assets/models/base/hangar.glb`) sits on a levelled pad at `terrain::BASE`.
  The north (+Z) door stays open for the vehicle, and runs and respawns start at
  `SPAWN` inside. Rebuild the runtime GLB with glTF Transform `simplify --ratio
  0.045 --error 0.0014`, then `resize --width 2048 --height 2048` (use
  `--vertex-layout separate` for both), bake `tangents`, then run
  `tools/prepare_hangar_asset.py` to level the slab and apron. If the geometry
  changes, remeasure the walls and pendant mounts in `base.rs`. A stronger
  local light probe and broad warm pendants keep the interior readable without
  changing outdoor lighting. Prop and artifact spotlights add focused task and
  exhibit light on top of that soft fill.
  The workshop and tool rack, jack, map board, lab bench, table, and toilet
  use the hangar-local poses and collision in `src/world/base/props.rs`.
  Each prop also has a short-range warm spotlight without a shadow map.
  Rebuild their GLBs in `assets/models/base/props/` with glTF Transform `simplify
  --ratio 0.012 --error 0.0035`, `resize --width 1024 --height 1024`, and
  `tangents` (all with `--vertex-layout separate`), then ground them and repair
  tangent seams with `tools/prepare_plant_asset.py`. Keep the central aisle
  and the parked vehicle's path through the bay clear. The crawler, quarry
  vehicle, sedan, and freight train are grounded in a separated line behind
  the hangar's south wall, inside the level cordon yard and off the front gate
  route. Each yard vehicle has static collision sampled from its prepared GLB.
- **Armored vehicle** (`src/player/vehicle.rs`): parked in the hangar, driveable with
  independent wheel pivots and a forward spotlight that turns on while occupied.
  The chassis and suspension sample the rendered terrain under the wheels.
  Forward speed rises from 16 m/s off-road to 24 m/s on the center of a graded
  road, then blends back down near the shoulder and retains the ford water cap.
  Throttle and steering ease in, steering gets gentler at speed, and the chase
  camera damps vehicle movement and zoom without delaying mouse orbit.
  When throttle is held while blocked, the tires spin without moving the hull.
  Slopes beyond tire grip cause downhill sliding; the car and stalker fall at 9.81 m/s².
  `tools/prepare_vehicle_asset.py` bakes the GLB's 3072 × 2048 PBR atlases from
  four texture sheets, with matte paint, dark trim, louver and lamp details,
  and subtle procedural wear.
  The vehicle uses mipmaps and anisotropic filtering to keep angled armor sharp.
  Regenerate `src/world/pillars/armored_vehicle.rs` with `tools/trim_tree_base.py`
  after mesh changes.
- **Collision** (`src/world/collision.rs`): every solid model carries a `Collider`, a
  set of upright `Pillar`s posed by its `GlobalTransform`; `push_out` moves bodies out
  of them. Pillars under `STEP_HEIGHT` are stepped over, ones above the body are passed
  under (crawling body is 0.5 m). Give every new solid model a `Collider`.
- **Audio** (`src/audio/`) is synthesized at startup: looping Zone ambience,
  gait-dependent footsteps, jump and landing, vehicle doors, and a speed-driven
  engine. Wildlife calls, footfalls, takeoff/landing, and fight cues originate
  from the creature's world position; nearby sounds pan with the camera and fade
  with distance. Footfalls follow accepted travel and attack sounds follow
  chase, lunge, and hit events. M mutes both positional and non-positional audio.
  Playback follows the system's default audio output.
- **Stalker model** (`assets/models/stalker.glb`) is a Tripo mesh auto-rigged in headless
  Blender by `tools/rig_stalker.py`, which also bakes the Crawl/Idle/Jump/Run/Walk clips
  (glTF animation indices 0–4, sorted by name). `src/player/animation.rs` cross-fades
  them by `Gait` and pitches the body face-down while crawling (the Crawl clip is posed
  standing). Edit motion in the script and regenerate rather than hand-editing the GLB.
- **Sky & lighting** (`src/world/mod.rs`) use Bevy's physical `Atmosphere` with a `SunDisk`
  lit at `lux::RAW_SUNLIGHT`; the camera runs HDR at `Exposure { ev100: 13.0 }`. Tune light
  and emissive values in physical units for that exposure, and steer the look with `SUN_DIRECTION`
  and `HAZE` rather than adding ambient light. The sun is also a
  `VolumetricLight` for the low fog's directional scattering.

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
