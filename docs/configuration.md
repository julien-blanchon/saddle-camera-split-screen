# Configuration

`SplitScreenConfig` is the main tuning resource. All values are public and can be edited at runtime.

## `SplitScreenConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `max_players` | `u8` | `4` | `1..=4` | Hard cap for how many slots are considered per render target |
| `min_players` | `u8` | `2` | `1..=4` | Minimum slot count before the runtime considers a non-trivial split; below this it collapses to a shared view |
| `mode` | `SplitScreenMode` | `Auto` | enum | Chooses auto, shared-only, fixed-only, or dynamic-only behavior |
| `default_projection` | `SplitScreenProjectionPlane` | `Xy` | enum | Fallback `Vec3 -> Vec2` projection for targets that do not override their projection |
| `aspect_policy` | `SplitScreenAspectPolicy` | `MatchWindow` | enum | Biases vertical vs horizontal orientation when auto-choosing a split family |
| `balance_policy` | `SplitScreenBalancePolicy` | `Weighted` | enum | Controls whether slot weights influence region sizes |
| `two_player` | `SplitScreenTwoPlayerConfig` | see below | n/a | Two-player merge/split behavior |
| `three_player` | `SplitScreenThreePlayerConfig` | see below | n/a | Three-player layout family and hybrid strategy |
| `four_player` | `SplitScreenFourPlayerConfig` | see below | n/a | Four-player layout family and hybrid strategy |
| `divider` | `SplitScreenDividerStyle` | see below | n/a | Seam styling metadata published in snapshots |
| `safe_area_padding` | `SplitScreenPadding` | `16px` on each edge | `>= 0` | Insets the usable render area before viewports are generated |
| `minimum_viewport_size` | `UVec2` | `220 x 140` | `>= 1` | Lower bound used while solving split fractions |
| `resize_debounce_frames` | `u8` | `2` | `0..=255` | Number of update ticks to suppress resize-driven layout-change messages after a resize event |
| `transition` | `SplitScreenTransitionConfig` | see below | n/a | Animated viewport transition settings |
| `letterbox` | `SplitScreenLetterboxConfig` | see below | n/a | Per-viewport aspect ratio enforcement |
| `border` | `SplitScreenBorderConfig` | see below | n/a | Per-viewport border decoration metadata |
| `debug` | `SplitScreenDebugConfig` | see below | n/a | Logging and debug-surface toggles |

## `SplitScreenTwoPlayerConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `fixed_layout` | `SplitScreenTwoPlayerLayout` | `Auto` | enum | Fixed orientation preference, and the tie-breaker for dynamic mode |
| `merge_inner_distance` | `f32` | `5.0` | `>= 0` | Distance below which an already-split session can merge |
| `merge_outer_distance` | `f32` | `9.0` | `> merge_inner_distance` | Distance above which a shared session will definitely split |
| `axis_hysteresis` | `f32` | `0.18` | `0..=0.49` | Stabilizes auto axis selection near diagonal separations |

Interactions:

- keep `merge_outer_distance` above `merge_inner_distance` or the hysteresis band collapses
- `fixed_layout = Vertical` or `Horizontal` effectively locks the split family while preserving merge behavior
- when `balance_policy = Weighted`, the actual divider position is also biased by each slot's `SplitScreenView::area_weight`

## `SplitScreenThreePlayerConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `layout` | `SplitScreenThreePlayerLayout` | `Auto` | enum | Chooses the dominant-panel arrangement |
| `strategy` | `SplitScreenMultiPlayerStrategy` | `Hybrid` | enum | `BalancedFixed` keeps authored arrangements; `Hybrid` also reorders and rebalances by player position/weight |

## `SplitScreenFourPlayerConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `layout` | `SplitScreenFourPlayerLayout` | `Auto` | enum | Chooses grid or strip layouts |
| `strategy` | `SplitScreenMultiPlayerStrategy` | `Hybrid` | enum | `Hybrid` keeps the layout family but rebalances row and column splits using slot weights |

## `SplitScreenDividerStyle`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `width` | `f32` | `6.0` | `>= 0` | Suggested seam thickness in pixels |
| `feather` | `f32` | `18.0` | `>= 0` | Suggested soft edge width for overlay or compositor consumers |
| `color` | `Color` | translucent near-white | any color | Suggested seam tint |
| `show_seam` | `bool` | `true` | `true/false` | Enables divider metadata in snapshots |

## `SplitScreenPadding`

All values are physical pixels.

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `left` | `u32` | `16` | Insets the left edge of the usable layout area |
| `right` | `u32` | `16` | Insets the right edge of the usable layout area |
| `top` | `u32` | `16` | Insets the top edge of the usable layout area |
| `bottom` | `u32` | `16` | Insets the bottom edge of the usable layout area |

Use larger padding for UI-heavy HUDs that need breathing room inside each viewport.

## `SplitScreenTransitionConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `true/false` | Enables or disables animated layout transitions |
| `duration_seconds` | `f32` | `0.35` | `> 0` | Duration of a viewport transition animation in seconds |
| `easing` | `SplitScreenTransitionEasing` | `SmoothStep` | enum | Easing curve: `Linear`, `SmoothStep`, or `EaseOutCubic` |

When a layout change is detected (player join/leave, mode switch, etc.), viewports smoothly interpolate from their previous normalized rects to their new targets over `duration_seconds`. Set `enabled = false` to snap immediately.

## `SplitScreenLetterboxConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `policy` | `SplitScreenLetterboxPolicy` | `None` | enum | Aspect ratio enforcement: `None`, `Maintain16x9`, `Maintain4x3`, or `Custom(f32)` |
| `fill_color` | `Color` | `Color::BLACK` | any color | Color of the letterbox/pillarbox bars (rendering is up to the consumer or overlay system) |

When a non-`None` policy is active, the physical viewport of each player is shrunk to fit the target aspect ratio, centered within the allocated region. The `letterboxed_physical` field on `SplitScreenViewSnapshot` contains the adjusted rect, and is used for camera viewport assignment.

## `SplitScreenBorderConfig`

| Field | Type | Default | Valid Range | Effect |
| --- | --- | --- | --- | --- |
| `enabled` | `bool` | `false` | `true/false` | Enables per-viewport border metadata in snapshots |
| `width` | `f32` | `3.0` | `>= 0` | Border thickness in pixels |
| `color` | `Color` | translucent gray | any color | Default border color for all viewports |
| `per_slot_colors` | `Vec<Color>` | empty | up to 4 entries | Optional per-slot color override (falls back to `color` if no entry for a slot) |

Border metadata is exposed on each `SplitScreenViewSnapshot` via `border_color` and `border_width`. The crate does not render borders itself; the consumer or an overlay system uses this metadata to draw visual decorations.

## `SplitScreenDebugConfig`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `log_changes` | `bool` | `false` | Emits an `info!` line whenever a snapshot materially changes |
| `expose_snapshot` | `bool` | `true` | Keeps runtime snapshots populated for inspection and BRP |
| `draw_divider` | `bool` | `true` | Intended for crate-local labs or consumers that render the divider metadata |
| `draw_viewport_bounds` | `bool` | `false` | Intended for external debug overlays |
| `draw_targets` | `bool` | `false` | Intended for external debug overlays |
