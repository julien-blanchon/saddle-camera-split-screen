# Saddle Camera Split Screen

Dynamic split-screen orchestration for 2-4 local players in Bevy.

The crate manages local-player slot ownership, viewport layout, merged-versus-split transitions, and per-player UI camera targeting. It does not spawn players, move cameras, or impose an input schema. Consumers bring their own controllers and attach this crate's slot and target components where needed.

## Quick Start

```toml
[dependencies]
split_screen = { path = "shared/camera/split_screen" }
bevy = "0.18"
```

```rust,no_run
use bevy::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenPlugin, SplitScreenTarget, SplitScreenView,
};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Gameplay,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SplitScreenPlugin::new(
            OnEnter(DemoState::Gameplay),
            OnExit(DemoState::Gameplay),
            Update,
        )))
        .init_state::<DemoState>()
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Player 0 Target"),
        LocalPlayerSlot(0),
        SplitScreenTarget::default(),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Player 0 Camera"),
        LocalPlayerSlot(0),
        Camera3d::default(),
        SplitScreenCamera::default(),
        SplitScreenView::default(),
        Transform::from_xyz(0.0, 6.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

For always-on examples and debug tools, `SplitScreenPlugin::always_on(Update)` is the short constructor.

## What It Does

- keeps slot identity stable with `LocalPlayerSlot`
- tracks world-space influence anchors through `SplitScreenTarget`
- partitions a render target into per-slot viewports
- collapses two-player sessions into a shared view when the tracked targets are close
- reassigns `UiTargetCamera` on `SplitScreenUiRoot` nodes so each HUD follows the correct view
- exposes runtime snapshots and messages so other systems can inspect the current layout

## What It Does Not Do

- spawn players or cameras for you
- move or blend the managed cameras
- decide how your game joins players or binds input devices
- render a custom compositor shader for true non-rectangular view masking

The current production path is a viewport-first hybrid. Two-player mode computes a dynamic divider direction and smooth merge/split transition, then applies the closest practical rectangular layout while exposing divider metadata for overlays or custom compositors. Three- and four-player layouts favor stable readable regions with weighted balancing over aggressive experimental partitioning.

## Public API

| Type | Purpose |
| --- | --- |
| `SplitScreenPlugin` | Registers the runtime with injectable activate, deactivate, and update schedules |
| `SplitScreenSystems` | Public ordering hooks: `CollectTargets`, `ComputeLayout`, `ApplyViewports`, `SyncUi`, `Debug` |
| `LocalPlayerSlot` | Stable slot identity shared across targets, cameras, and UI roots |
| `SplitScreenTarget` | Marks an entity whose position influences layout decisions |
| `SplitScreenView` | Per-slot screen-area weighting metadata |
| `SplitScreenCamera` | Marks a camera managed by the crate; supports UI-anchor selection |
| `SplitScreenUiRoot` | Marks a UI root that should retarget to the selected managed camera for its slot |
| `SplitScreenConfig` | Central tuning resource for modes, merge thresholds, layouts, padding, divider style, and debug hooks |
| `SplitScreenRuntime` | Readable runtime resource exposing the latest snapshots and resize bookkeeping |
| `SplitScreenLayoutSnapshot` | Current logical and physical regions for one render target |
| Messages | `SplitScreenLayoutChanged`, `SplitScreenPlayerViewAssigned`, `SplitScreenModeChanged` |

## Configuration Summary

`SplitScreenConfig` keeps the main tuning surface in one place:

- `mode`: `Auto`, `SharedOnly`, `FixedOnly`, or `DynamicOnly`
- `two_player`: merge hysteresis, fixed-layout preference, and axis hysteresis
- `three_player` / `four_player`: fixed-layout families plus hybrid balancing strategy
- `safe_area_padding`: physical padding inside the render target before viewports are assigned
- `minimum_viewport_size`: floor used when computing split fractions
- `divider`: seam thickness, feathering, tint, and whether seam metadata is exposed
- `balance_policy`: uniform or weighted area balancing using `SplitScreenView::area_weight`
- `default_projection`: how `SplitScreenTarget` positions collapse from `Vec3` to layout space when the component does not override it
- `resize_debounce_frames`: coalesces resize-driven layout-change messages during rapid window changes

## Integration Notes

### Managed Cameras

- Tag every managed camera with `LocalPlayerSlot` plus `SplitScreenCamera`.
- Add `SplitScreenView` to any entity that should contribute area weight for that slot. Placing it on the camera is the simplest pattern.
- Multiple cameras may share the same slot and will receive the same viewport. Use `SplitScreenCamera { ui_anchor: true }` on the one that should drive the slot's HUD targeting.

### Local Player Slots

- Slots are completely generic metadata. Your own input layer can map controllers, action contexts, or save data to the same `LocalPlayerSlot`.
- The crate sorts by slot value, not spawn order, so join/leave cycles do not scramble layout ownership.

### `UiTargetCamera`

- Mark per-player HUD roots with `SplitScreenUiRoot` and the matching `LocalPlayerSlot`.
- The crate inserts or refreshes `UiTargetCamera` automatically.
- When a two-player session merges into a shared view, both roots target the merged owner's camera so HUDs stay visible.

### `RenderLayers`

- This crate does not assign `RenderLayers` itself.
- If a project needs slot-specific world content, pair `LocalPlayerSlot` with your own `RenderLayers` routing or camera-filter logic. The layout runtime does not interfere with that setup.

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal two-player fixed split with viewport ownership | `cargo run -p split_screen --example basic` |
| `dynamic_two_player` | Merge/split transitions with a dynamic divider overlay | `cargo run -p split_screen --example dynamic_two_player` |
| `four_player` | Four-player grid and strip-ready layout path | `cargo run -p split_screen --example four_player` |
| `per_player_ui` | Slot-targeted HUD roots using automatic `UiTargetCamera` retargeting | `cargo run -p split_screen --example per_player_ui` |

## Workspace Lab

The richer verification app lives inside the crate at `shared/camera/split_screen/examples/lab`:

```bash
cargo run -p split_screen_lab
```

With E2E enabled:

```bash
cargo run -p split_screen_lab --features e2e -- split_screen_smoke
```

For live inspection over BRP, use the crate-local lab README:

```bash
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras diagnostics
```

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
