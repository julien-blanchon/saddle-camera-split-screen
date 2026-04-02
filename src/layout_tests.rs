use std::collections::BTreeMap;

use bevy::prelude::*;

use super::*;
use crate::{
    SplitScreenConfig, SplitScreenFourPlayerLayout, SplitScreenMode, SplitScreenPadding,
    SplitScreenThreePlayerLayout, SplitScreenTwoPlayerConfig, SplitScreenTwoPlayerLayout,
};

fn participant(slot: u8, position: Vec2, area_weight: f32) -> LayoutParticipant {
    LayoutParticipant {
        slot: LocalPlayerSlot(slot),
        position,
        area_weight,
    }
}

fn snapshot_with(
    config: SplitScreenConfig,
    participants: Vec<LayoutParticipant>,
    previous_mode: Option<SplitScreenLayoutMode>,
    target_size: UVec2,
) -> SplitScreenLayoutSnapshot {
    build_layout_snapshot(LayoutContext {
        target_window: None,
        target_size,
        participants: &participants,
        previous_mode,
        config: &config,
    })
}

fn view(snapshot: &SplitScreenLayoutSnapshot, slot: u8) -> &SplitScreenViewSnapshot {
    snapshot
        .views
        .iter()
        .find(|view| view.slot == LocalPlayerSlot(slot))
        .expect("view for slot should exist")
}

fn assert_snapshot_is_valid(snapshot: &SplitScreenLayoutSnapshot) {
    for view in &snapshot.views {
        assert!(view.normalized.min.is_finite());
        assert!(view.normalized.max.is_finite());
        assert!(view.normalized.min.x <= view.normalized.max.x);
        assert!(view.normalized.min.y <= view.normalized.max.y);
        assert!(view.physical.size.x <= snapshot.target.physical_size.x);
        assert!(view.physical.size.y <= snapshot.target.physical_size.y);
    }

    if let Some(divider) = snapshot.divider.as_ref() {
        assert!(divider.normalized_start.is_finite());
        assert!(divider.normalized_end.is_finite());
        assert!(divider.physical_start.is_finite());
        assert!(divider.physical_end.is_finite());
    }
}

fn rects_by_slot(snapshot: &SplitScreenLayoutSnapshot) -> BTreeMap<LocalPlayerSlot, PhysicalRect> {
    snapshot
        .views
        .iter()
        .map(|view| (view.slot, view.physical))
        .collect()
}

#[test]
fn two_player_shared_mode_prefers_weighted_owner_when_targets_stay_close() {
    let config = SplitScreenConfig {
        two_player: SplitScreenTwoPlayerConfig {
            merge_inner_distance: 40.0,
            merge_outer_distance: 80.0,
            ..default()
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-12.0, 0.0), 1.0),
            participant(1, Vec2::new(12.0, 0.0), 2.5),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(snapshot.mode, SplitScreenLayoutMode::Shared);
    assert_eq!(snapshot.merged_owner, Some(LocalPlayerSlot(1)));
    assert!(view(&snapshot, 1).active);
    assert_eq!(view(&snapshot, 0).physical.size, UVec2::ZERO);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn diagonal_two_player_split_exposes_a_slanted_divider() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::DynamicOnly,
        two_player: SplitScreenTwoPlayerConfig {
            merge_inner_distance: 40.0,
            merge_outer_distance: 80.0,
            ..default()
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-180.0, -120.0), 1.0),
            participant(1, Vec2::new(180.0, 120.0), 1.0),
        ],
        Some(SplitScreenLayoutMode::Shared),
        UVec2::new(1280, 720),
    );

    assert_eq!(snapshot.mode, SplitScreenLayoutMode::DynamicTwoPlayer);
    let divider = snapshot
        .divider
        .clone()
        .expect("dynamic split should expose a divider");
    assert_ne!(divider.normalized_start.y, divider.normalized_end.y);
    assert!(view(&snapshot, 0).active);
    assert!(view(&snapshot, 1).active);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn minimum_viewport_size_clamps_weighted_two_player_regions() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::DynamicOnly,
        minimum_viewport_size: UVec2::new(500, 140),
        safe_area_padding: SplitScreenPadding {
            left: 16,
            right: 16,
            top: 16,
            bottom: 16,
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-280.0, 0.0), 1.0),
            participant(1, Vec2::new(280.0, 0.0), 3.0),
        ],
        Some(SplitScreenLayoutMode::DynamicTwoPlayer),
        UVec2::new(1280, 720),
    );

    assert!(view(&snapshot, 0).physical.size.x >= 500);
    assert!(view(&snapshot, 1).physical.size.x >= 500);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn coincident_two_player_targets_do_not_generate_invalid_values() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::DynamicOnly,
        two_player: SplitScreenTwoPlayerConfig {
            merge_inner_distance: 0.0,
            merge_outer_distance: 0.0,
            ..default()
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::ZERO, 1.0),
            participant(1, Vec2::ZERO, 1.0),
        ],
        Some(SplitScreenLayoutMode::DynamicTwoPlayer),
        UVec2::new(960, 540),
    );

    assert_eq!(snapshot.mode, SplitScreenLayoutMode::DynamicTwoPlayer);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn weighted_three_player_layout_gives_the_primary_panel_to_the_heaviest_slot() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        three_player: crate::SplitScreenThreePlayerConfig {
            layout: SplitScreenThreePlayerLayout::WideLeft,
            ..default()
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-220.0, 140.0), 1.0),
            participant(1, Vec2::new(220.0, 160.0), 1.0),
            participant(2, Vec2::new(0.0, -180.0), 3.5),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(snapshot.mode, SplitScreenLayoutMode::FixedThreeWideLeft);
    assert!(view(&snapshot, 2).physical.size.x > view(&snapshot, 0).physical.size.x);
    assert!(view(&snapshot, 2).physical.size.x > view(&snapshot, 1).physical.size.x);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn four_player_strip_layout_balances_weights_without_invalid_rectangles() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        four_player: crate::SplitScreenFourPlayerConfig {
            layout: SplitScreenFourPlayerLayout::VerticalStrip,
            ..default()
        },
        minimum_viewport_size: UVec2::new(180, 120),
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-320.0, 0.0), 3.0),
            participant(1, Vec2::new(-80.0, 0.0), 1.0),
            participant(2, Vec2::new(120.0, 0.0), 1.0),
            participant(3, Vec2::new(320.0, 0.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(snapshot.mode, SplitScreenLayoutMode::FixedFourVerticalStrip);
    let leader_width = view(&snapshot, 0).physical.size.x;
    assert!(leader_width > view(&snapshot, 1).physical.size.x);
    assert!(leader_width > view(&snapshot, 2).physical.size.x);
    assert!(leader_width > view(&snapshot, 3).physical.size.x);
    assert_snapshot_is_valid(&snapshot);
}

#[test]
fn three_player_balanced_fixed_strategy_keeps_slot_order_stable_across_position_swaps() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        three_player: crate::SplitScreenThreePlayerConfig {
            layout: SplitScreenThreePlayerLayout::WideTop,
            strategy: crate::SplitScreenMultiPlayerStrategy::BalancedFixed,
            ..default()
        },
        ..default()
    };

    let first = snapshot_with(
        config.clone(),
        vec![
            participant(0, Vec2::new(-340.0, -120.0), 1.0),
            participant(1, Vec2::new(120.0, 240.0), 1.0),
            participant(2, Vec2::new(10.0, -20.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    let second = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(500.0, 500.0), 1.0),
            participant(1, Vec2::new(-420.0, -220.0), 1.0),
            participant(2, Vec2::new(3.0, 420.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(rects_by_slot(&first), rects_by_slot(&second));
}

#[test]
fn four_player_balanced_fixed_strategy_ignores_position_for_fixed_ordering() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        four_player: crate::SplitScreenFourPlayerConfig {
            layout: SplitScreenFourPlayerLayout::Grid,
            strategy: crate::SplitScreenMultiPlayerStrategy::BalancedFixed,
            ..default()
        },
        ..default()
    };

    let first = snapshot_with(
        config.clone(),
        vec![
            participant(0, Vec2::new(-340.0, 260.0), 1.0),
            participant(1, Vec2::new(120.0, -40.0), 1.0),
            participant(2, Vec2::new(340.0, 80.0), 1.0),
            participant(3, Vec2::new(-90.0, 260.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    let second = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(2000.0, 10.0), 1.0),
            participant(1, Vec2::new(-100.0, 20.0), 1.0),
            participant(2, Vec2::new(40.0, -420.0), 1.0),
            participant(3, Vec2::new(-30.0, 520.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(rects_by_slot(&first), rects_by_slot(&second));
}

#[test]
fn slot_identity_is_stable_even_when_participants_arrive_in_different_orders() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        two_player: SplitScreenTwoPlayerConfig {
            fixed_layout: SplitScreenTwoPlayerLayout::Vertical,
            ..default()
        },
        ..default()
    };
    let first = snapshot_with(
        config.clone(),
        vec![
            participant(0, Vec2::new(-200.0, 0.0), 1.0),
            participant(1, Vec2::new(200.0, 0.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );
    let second = snapshot_with(
        config,
        vec![
            participant(1, Vec2::new(200.0, 0.0), 1.0),
            participant(0, Vec2::new(-200.0, 0.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    assert_eq!(rects_by_slot(&first), rects_by_slot(&second));
}

#[test]
fn physical_rectangles_respect_safe_area_padding() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        safe_area_padding: SplitScreenPadding {
            left: 32,
            right: 48,
            top: 24,
            bottom: 40,
        },
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-200.0, 0.0), 1.0),
            participant(1, Vec2::new(200.0, 0.0), 1.0),
        ],
        None,
        UVec2::new(1280, 720),
    );

    for view in &snapshot.views {
        if view.active {
            assert!(view.physical.position.x >= 32);
            assert!(view.physical.position.y >= 24);
        }
    }
}

#[test]
fn extreme_aspect_ratios_keep_regions_valid() {
    let config = SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        four_player: crate::SplitScreenFourPlayerConfig {
            layout: SplitScreenFourPlayerLayout::HorizontalStrip,
            ..default()
        },
        minimum_viewport_size: UVec2::new(120, 60),
        ..default()
    };

    let snapshot = snapshot_with(
        config,
        vec![
            participant(0, Vec2::new(-200.0, 300.0), 1.0),
            participant(1, Vec2::new(0.0, 100.0), 1.0),
            participant(2, Vec2::new(0.0, -100.0), 1.0),
            participant(3, Vec2::new(200.0, -300.0), 1.0),
        ],
        None,
        UVec2::new(4000, 300),
    );

    assert_snapshot_is_valid(&snapshot);
    assert!(snapshot.views.iter().all(|view| view.physical.size.y >= 1));
}
