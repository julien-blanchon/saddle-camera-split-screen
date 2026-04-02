# `split_screen_lab`

Crate-local verification harness for [`split_screen`](../../README.md).

## Purpose

- exercise merged and split two-player layouts
- verify four-player viewport ownership
- inspect per-player UI retargeting
- debug runtime snapshots over BRP
- run crate-local E2E scenarios without depending on `crates/sandboxes`

## Run

```bash
cargo run -p split_screen_lab
```

Hotkeys:

- `1`: merged two-player framing
- `2`: diagonal two-player split
- `3`: four-player layout
- `4`: per-player UI emphasis
- `A`: `SplitScreenMode::Auto`
- `D`: `SplitScreenMode::DynamicOnly`
- `F`: `SplitScreenMode::FixedOnly`
- `S`: `SplitScreenMode::SharedOnly`

## BRP

The default feature enables BRP extras.

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch split_screen_lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/split_screen_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```

Useful resources and components to inspect:

- `split_screen::SplitScreenRuntime`
- `split_screen::SplitScreenConfig`
- `bevy_ecs::name::Name`
- `split_screen::SplitScreenCamera`
- `split_screen::SplitScreenUiRoot`

## E2E

```bash
cargo run -p split_screen_lab --features e2e -- split_screen_smoke
cargo run -p split_screen_lab --features e2e -- split_screen_two_player_merge
cargo run -p split_screen_lab --features e2e -- split_screen_two_player_slanted_split
cargo run -p split_screen_lab --features e2e -- split_screen_resize
cargo run -p split_screen_lab --features e2e -- split_screen_four_player
cargo run -p split_screen_lab --features e2e -- split_screen_per_player_ui
```
