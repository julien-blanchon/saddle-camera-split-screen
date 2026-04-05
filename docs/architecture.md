# Architecture

`split_screen` separates pure layout reasoning from ECS glue.

## Data Flow

1. `SplitScreenTarget` entities publish the world-space anchors that matter for framing.
2. `SplitScreenView` contributes slot-level area weight.
3. `SplitScreenCamera` groups cameras by render target and slot.
4. The runtime projects target anchors into a 2D layout space, computes logical regions in normalized coordinates, and stores the result as `SplitScreenLayoutSnapshot`.
5. Camera viewports are updated from the physical rectangles derived from that snapshot.
6. `SplitScreenUiRoot` nodes are retargeted to the chosen camera for their slot.
7. Messages publish materially changed snapshots, mode transitions, and slot-to-camera assignments.

## Merge And Split Logic

Two-player mode uses a hybrid strategy:

- If the tracked anchors stay within `merge_outer_distance`, the session remains merged into one shared view.
- Once already split, it only re-merges below `merge_inner_distance`.
- Between those distances, the runtime computes a smooth `transition_alpha` that can drive seam or post-process overlays.

The shipping viewport path stays rectangular because Bevy `Viewport` is rectangular by design. The runtime still computes a divider direction from the players' relative positions and exposes it in `SplitScreenDividerSnapshot`, so consumers can render a slanted seam or add a custom compositor later without rewriting the slot and layout model.

When the balance policy is weighted, the runtime now also tracks the actual split ratio used for the rectangular layout. Divider metadata is emitted from that true ratio instead of assuming a centered seam, which keeps debug overlays and custom compositors aligned with weighted view ownership.

## Fixed And Hybrid Layouts

Three- and four-player layouts favor readability and deterministic ownership:

- 3 players: one dominant panel plus two secondary regions
- 4 players: weighted grid by default, with strip layouts available for edge cases

`SplitScreenView::area_weight` biases how much space a slot receives when the selected layout supports balancing. The current hybrid path reorders fixed regions using player positions and weights instead of attempting a full arbitrary Voronoi compositor for 3-4 players.

## UI Binding

`SplitScreenUiRoot` is intentionally small. The crate does not build HUD widgets; it only keeps the root node pointed at the correct camera through `UiTargetCamera`.

- In split mode, each slot points at its chosen UI-anchor camera.
- In merged shared mode, all slot HUD roots target the merged owner's camera so UI stays visible even though the other cameras are inactive.

## Animated Layout Transitions

When a layout change is detected (different mode, different number of views, or significantly different normalized rects), the runtime starts a transition animation:

1. The previous normalized rects for each slot are stored as "from" values.
2. The new computed rects become "to" values.
3. Each frame, the runtime advances a progress counter based on delta time and the configured duration.
4. The eased progress is used to lerp between from/to rects.
5. The interpolated rects are converted to physical viewports and applied to cameras.

For newly joining players, the "from" rect defaults to zero (collapsed), so the viewport slides in from nothing. For departing players, the remaining viewports smoothly expand to fill the vacated space.

The transition system respects the existing merge/split hysteresis — merge transitions use the dynamic alpha, while layout transitions use the configurable easing curve.

## Letterboxing and Borders

After computing (and optionally interpolating) each view's normalized rect:

- **Letterboxing**: if a `SplitScreenLetterboxPolicy` is active, each view's physical rect is shrunk to maintain the target aspect ratio, centered within the allocated region. The `letterboxed_physical` field on each `SplitScreenViewSnapshot` contains the result, and the camera viewport uses this adjusted rect.
- **Borders**: if `SplitScreenBorderConfig` is enabled, each view snapshot includes `border_color` and `border_width` metadata. The crate does not render borders itself — this is presentation metadata for overlay systems or consumers.

## Performance Notes

Layout computation is cheap:

- at most 4 participants are considered
- normalized rect math is pure scalar work
- the runtime does not allocate render targets or textures

The expensive part is rendering multiple cameras. That cost is real and outside the scope of the crate. This is why the runtime:

- exposes merged shared mode for the common two-player "stay close" case
- keeps three- and four-player defaults stable and readable instead of constantly reshaping the screen
- leaves optional render-scaling or per-camera world filtering to the consuming game

## Why This Crate Uses A Viewport-First Hybrid

True Voronoi split-screen compositing usually means rendering full views, then compositing them with a shader or post-process mask. That is powerful, but it couples layout, render targets, and compositing much more tightly.

This crate keeps the reusable shared surface focused on:

- slot identity
- target collection
- robust region computation
- camera viewport ownership
- UI routing
- debug inspection

That keeps the public API small and project-agnostic while still leaving a clean seam for games that want a custom slanted or fully Voronoi compositor later.
