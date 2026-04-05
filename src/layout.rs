use bevy::prelude::*;

use crate::{
    LocalPlayerSlot, SplitScreenAspectPolicy, SplitScreenBalancePolicy, SplitScreenConfig,
    SplitScreenFourPlayerLayout, SplitScreenMode, SplitScreenMultiPlayerStrategy,
    SplitScreenPadding, SplitScreenThreePlayerLayout, SplitScreenTwoPlayerLayout, math,
};

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Default)]
pub struct NormalizedRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl NormalizedRect {
    pub const fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub const fn full() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::ONE,
        }
    }

    pub const fn zero() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        }
    }

    pub fn width(self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalRect {
    pub position: UVec2,
    pub size: UVec2,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenLayoutMode {
    #[default]
    Shared,
    DynamicTwoPlayer,
    FixedTwoVertical,
    FixedTwoHorizontal,
    FixedThreeWideTop,
    FixedThreeWideBottom,
    FixedThreeWideLeft,
    FixedThreeWideRight,
    FixedFourGrid,
    FixedFourVerticalStrip,
    FixedFourHorizontalStrip,
}

#[derive(Reflect, Debug, Clone, PartialEq)]
pub struct SplitScreenViewSnapshot {
    pub slot: LocalPlayerSlot,
    pub active: bool,
    pub normalized: NormalizedRect,
    pub physical: PhysicalRect,
    pub area_weight: f32,
    pub letterboxed_physical: Option<PhysicalRect>,
    pub border_color: Option<Color>,
    pub border_width: f32,
}

#[derive(Reflect, Debug, Clone, PartialEq)]
pub struct SplitScreenDividerSnapshot {
    pub normalized_start: Vec2,
    pub normalized_end: Vec2,
    pub physical_start: Vec2,
    pub physical_end: Vec2,
    pub thickness: f32,
    pub feather: f32,
    pub color: Color,
}

#[derive(Reflect, Debug, Clone, PartialEq)]
pub struct SplitScreenRenderTargetInfo {
    pub window: Option<Entity>,
    pub physical_size: UVec2,
}

#[derive(Reflect, Debug, Clone, PartialEq)]
pub struct SplitScreenLayoutSnapshot {
    pub target: SplitScreenRenderTargetInfo,
    pub mode: SplitScreenLayoutMode,
    pub merged_owner: Option<LocalPlayerSlot>,
    pub transition_alpha: f32,
    pub views: Vec<SplitScreenViewSnapshot>,
    pub divider: Option<SplitScreenDividerSnapshot>,
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub struct SplitScreenRuntime {
    pub active: bool,
    pub frame_serial: u64,
    pub last_resize_window: Option<Entity>,
    pub snapshots: Vec<SplitScreenLayoutSnapshot>,
    pub transition_progress: f32,
    pub transition_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutParticipant {
    pub slot: LocalPlayerSlot,
    pub position: Vec2,
    pub area_weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutAxis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutContext<'a> {
    pub target_window: Option<Entity>,
    pub target_size: UVec2,
    pub participants: &'a [LayoutParticipant],
    pub previous_mode: Option<SplitScreenLayoutMode>,
    pub config: &'a SplitScreenConfig,
}

#[derive(Debug, Clone, PartialEq)]
struct ViewPlan {
    slot: LocalPlayerSlot,
    active: bool,
    rect: NormalizedRect,
    area_weight: f32,
}

pub(crate) fn build_layout_snapshot(context: LayoutContext<'_>) -> SplitScreenLayoutSnapshot {
    let clamped_player_count = context
        .participants
        .len()
        .min(context.config.max_players as usize);
    let participants = &context.participants[..clamped_player_count];

    if participants.is_empty() {
        return SplitScreenLayoutSnapshot {
            target: SplitScreenRenderTargetInfo {
                window: context.target_window,
                physical_size: context.target_size,
            },
            mode: SplitScreenLayoutMode::Shared,
            merged_owner: None,
            transition_alpha: 0.0,
            views: Vec::new(),
            divider: None,
        };
    }

    let owner = choose_shared_owner(participants);
    let (mode, plans, divider, merged_owner, alpha) = match context.config.mode {
        SplitScreenMode::SharedOnly => (
            SplitScreenLayoutMode::Shared,
            shared_view_plans(participants, owner),
            None,
            Some(owner),
            0.0,
        ),
        SplitScreenMode::FixedOnly => fixed_layout_snapshot(context, owner),
        SplitScreenMode::DynamicOnly => dynamic_or_fixed_snapshot(context, owner, true),
        SplitScreenMode::Auto => dynamic_or_fixed_snapshot(context, owner, false),
    };

    let views = plans
        .into_iter()
        .map(|plan| {
            let physical = math::normalized_to_physical(
                plan.rect,
                context.target_size,
                context.config.safe_area_padding,
            );
            let letterboxed_physical = context
                .config
                .letterbox
                .policy
                .target_aspect_ratio()
                .map(|target_ratio| math::letterbox_physical(physical, target_ratio));
            let border_color = if context.config.border.enabled {
                Some(context.config.border.color_for_slot(plan.slot.index()))
            } else {
                None
            };
            let border_width = if context.config.border.enabled {
                context.config.border.width
            } else {
                0.0
            };
            SplitScreenViewSnapshot {
                physical,
                slot: plan.slot,
                active: plan.active,
                normalized: plan.rect,
                area_weight: plan.area_weight,
                letterboxed_physical,
                border_color,
                border_width,
            }
        })
        .collect();

    SplitScreenLayoutSnapshot {
        target: SplitScreenRenderTargetInfo {
            window: context.target_window,
            physical_size: context.target_size,
        },
        mode,
        merged_owner,
        transition_alpha: alpha,
        views,
        divider,
    }
}

pub(crate) fn layout_materially_changed(
    previous: Option<&SplitScreenLayoutSnapshot>,
    next: &SplitScreenLayoutSnapshot,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    if previous.mode != next.mode
        || previous.merged_owner != next.merged_owner
        || previous.views.len() != next.views.len()
        || previous.target.physical_size != next.target.physical_size
    {
        return true;
    }

    previous.views.iter().zip(&next.views).any(|(left, right)| {
        left.slot != right.slot
            || left.active != right.active
            || left.physical != right.physical
            || (left.area_weight - right.area_weight).abs() > 0.001
    })
}

fn dynamic_or_fixed_snapshot(
    context: LayoutContext<'_>,
    owner: LocalPlayerSlot,
    force_dynamic: bool,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    let participants = context.participants;
    if participants.len() <= 1 || participants.len() < context.config.min_players as usize {
        return (
            SplitScreenLayoutMode::Shared,
            shared_view_plans(participants, owner),
            None,
            Some(owner),
            0.0,
        );
    }

    if participants.len() == 2 {
        if !force_dynamic && matches!(context.config.mode, SplitScreenMode::FixedOnly) {
            return fixed_two_player_snapshot(context);
        }
        return dynamic_two_player_snapshot(context, owner);
    }

    fixed_layout_snapshot(context, owner)
}

fn fixed_layout_snapshot(
    context: LayoutContext<'_>,
    owner: LocalPlayerSlot,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    match context.participants.len() {
        0 => (SplitScreenLayoutMode::Shared, Vec::new(), None, None, 0.0),
        1 => (
            SplitScreenLayoutMode::Shared,
            shared_view_plans(context.participants, owner),
            None,
            Some(owner),
            0.0,
        ),
        2 => fixed_two_player_snapshot(context),
        3 => fixed_three_player_snapshot(context),
        _ => fixed_four_player_snapshot(context),
    }
}

fn fixed_two_player_snapshot(
    context: LayoutContext<'_>,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    let participants = context.participants;
    let axis = match context.config.two_player.fixed_layout {
        SplitScreenTwoPlayerLayout::Auto => choose_axis_from_aspect(context),
        SplitScreenTwoPlayerLayout::Vertical => LayoutAxis::Vertical,
        SplitScreenTwoPlayerLayout::Horizontal => LayoutAxis::Horizontal,
    };
    let plans = split_two_fixed(
        participants,
        axis,
        context.target_size,
        context.config.safe_area_padding,
        context.config.minimum_viewport_size,
    );
    let mode = match axis {
        LayoutAxis::Vertical => SplitScreenLayoutMode::FixedTwoVertical,
        LayoutAxis::Horizontal => SplitScreenLayoutMode::FixedTwoHorizontal,
    };
    let divider = fixed_divider_snapshot(context, axis);
    (mode, plans, divider, None, 1.0)
}

fn fixed_three_player_snapshot(
    context: LayoutContext<'_>,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    let participants = context.participants;
    let layout = match context.config.three_player.layout {
        SplitScreenThreePlayerLayout::Auto => {
            if context.target_size.x >= context.target_size.y {
                SplitScreenThreePlayerLayout::WideTop
            } else {
                SplitScreenThreePlayerLayout::WideLeft
            }
        }
        layout => layout,
    };

    let plans = split_three_fixed(
        participants,
        layout,
        context.target_size,
        context.config.safe_area_padding,
        context.config.balance_policy,
        context.config.minimum_viewport_size,
        context.config.three_player.strategy,
    );
    let mode = match layout {
        SplitScreenThreePlayerLayout::Auto | SplitScreenThreePlayerLayout::WideTop => {
            SplitScreenLayoutMode::FixedThreeWideTop
        }
        SplitScreenThreePlayerLayout::WideBottom => SplitScreenLayoutMode::FixedThreeWideBottom,
        SplitScreenThreePlayerLayout::WideLeft => SplitScreenLayoutMode::FixedThreeWideLeft,
        SplitScreenThreePlayerLayout::WideRight => SplitScreenLayoutMode::FixedThreeWideRight,
    };
    (mode, plans, None, None, 1.0)
}

fn fixed_four_player_snapshot(
    context: LayoutContext<'_>,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    let participants = context.participants;
    let layout = match context.config.four_player.layout {
        SplitScreenFourPlayerLayout::Auto => SplitScreenFourPlayerLayout::Grid,
        layout => layout,
    };
    let plans = split_four_fixed(
        participants,
        layout,
        context.target_size,
        context.config.safe_area_padding,
        context.config.balance_policy,
        context.config.minimum_viewport_size,
        context.config.four_player.strategy,
    );
    let mode = match layout {
        SplitScreenFourPlayerLayout::Auto | SplitScreenFourPlayerLayout::Grid => {
            SplitScreenLayoutMode::FixedFourGrid
        }
        SplitScreenFourPlayerLayout::VerticalStrip => SplitScreenLayoutMode::FixedFourVerticalStrip,
        SplitScreenFourPlayerLayout::HorizontalStrip => {
            SplitScreenLayoutMode::FixedFourHorizontalStrip
        }
    };
    (mode, plans, None, None, 1.0)
}

fn dynamic_two_player_snapshot(
    context: LayoutContext<'_>,
    owner: LocalPlayerSlot,
) -> (
    SplitScreenLayoutMode,
    Vec<ViewPlan>,
    Option<SplitScreenDividerSnapshot>,
    Option<LocalPlayerSlot>,
    f32,
) {
    let participants = context.participants;
    let delta = participants[1].position - participants[0].position;
    let distance = delta.length();
    let keep_split = matches!(
        context.previous_mode,
        Some(SplitScreenLayoutMode::DynamicTwoPlayer)
    );

    if !keep_split && distance <= context.config.two_player.merge_outer_distance {
        return (
            SplitScreenLayoutMode::Shared,
            shared_view_plans(participants, owner),
            None,
            Some(owner),
            0.0,
        );
    }

    if keep_split && distance < context.config.two_player.merge_inner_distance {
        return (
            SplitScreenLayoutMode::Shared,
            shared_view_plans(participants, owner),
            None,
            Some(owner),
            0.0,
        );
    }

    let axis = match context.config.two_player.fixed_layout {
        SplitScreenTwoPlayerLayout::Vertical => LayoutAxis::Vertical,
        SplitScreenTwoPlayerLayout::Horizontal => LayoutAxis::Horizontal,
        SplitScreenTwoPlayerLayout::Auto => choose_dynamic_axis(context, delta),
    };

    let alpha = math::smoothstep(
        (distance - context.config.two_player.merge_inner_distance)
            / (context.config.two_player.merge_outer_distance
                - context.config.two_player.merge_inner_distance)
                .max(0.001),
    );

    let (plans, ratio) = split_two_dynamic(
        participants,
        axis,
        owner,
        alpha,
        context.target_size,
        context.config.safe_area_padding,
        context.config.minimum_viewport_size,
    );
    let divider = dynamic_divider_snapshot(context, axis, ratio, delta, alpha);

    (
        SplitScreenLayoutMode::DynamicTwoPlayer,
        plans,
        divider,
        Some(owner),
        alpha,
    )
}

fn shared_view_plans(participants: &[LayoutParticipant], owner: LocalPlayerSlot) -> Vec<ViewPlan> {
    let full = NormalizedRect::full();
    participants
        .iter()
        .copied()
        .map(|participant| ViewPlan {
            slot: participant.slot,
            active: participant.slot == owner,
            rect: if participant.slot == owner {
                full
            } else {
                NormalizedRect::zero()
            },
            area_weight: participant.area_weight,
        })
        .collect()
}

fn choose_shared_owner(participants: &[LayoutParticipant]) -> LocalPlayerSlot {
    participants
        .iter()
        .copied()
        .max_by(|left, right| {
            left.area_weight
                .total_cmp(&right.area_weight)
                .then(left.slot.cmp(&right.slot).reverse())
        })
        .map(|participant| participant.slot)
        .unwrap_or(LocalPlayerSlot(0))
}

fn choose_axis_from_aspect(context: LayoutContext<'_>) -> LayoutAxis {
    match context.config.aspect_policy {
        SplitScreenAspectPolicy::PreferVertical => LayoutAxis::Vertical,
        SplitScreenAspectPolicy::PreferHorizontal => LayoutAxis::Horizontal,
        SplitScreenAspectPolicy::MatchWindow => {
            if context.target_size.x >= context.target_size.y {
                LayoutAxis::Vertical
            } else {
                LayoutAxis::Horizontal
            }
        }
    }
}

fn choose_dynamic_axis(context: LayoutContext<'_>, delta: Vec2) -> LayoutAxis {
    let vertical_score = delta.x.abs();
    let horizontal_score = delta.y.abs();
    let hysteresis = context.config.two_player.axis_hysteresis.clamp(0.0, 0.49);

    let previous_axis = match context.previous_mode {
        Some(SplitScreenLayoutMode::FixedTwoVertical)
        | Some(SplitScreenLayoutMode::DynamicTwoPlayer)
            if vertical_score >= horizontal_score =>
        {
            Some(LayoutAxis::Vertical)
        }
        Some(SplitScreenLayoutMode::FixedTwoHorizontal) if horizontal_score >= vertical_score => {
            Some(LayoutAxis::Horizontal)
        }
        _ => None,
    };

    match (vertical_score, horizontal_score, previous_axis) {
        (v, h, Some(LayoutAxis::Vertical)) if v + hysteresis >= h => LayoutAxis::Vertical,
        (v, h, Some(LayoutAxis::Horizontal)) if h + hysteresis >= v => LayoutAxis::Horizontal,
        (v, h, _) => {
            if adjusted_vertical_score(v, context.config.aspect_policy)
                >= adjusted_horizontal_score(h, context.config.aspect_policy)
            {
                LayoutAxis::Vertical
            } else {
                LayoutAxis::Horizontal
            }
        }
    }
}

fn adjusted_vertical_score(score: f32, policy: SplitScreenAspectPolicy) -> f32 {
    match policy {
        SplitScreenAspectPolicy::PreferVertical => score * 1.2,
        SplitScreenAspectPolicy::PreferHorizontal => score * 0.8,
        SplitScreenAspectPolicy::MatchWindow => score,
    }
}

fn adjusted_horizontal_score(score: f32, policy: SplitScreenAspectPolicy) -> f32 {
    match policy {
        SplitScreenAspectPolicy::PreferHorizontal => score * 1.2,
        SplitScreenAspectPolicy::PreferVertical => score * 0.8,
        SplitScreenAspectPolicy::MatchWindow => score,
    }
}

fn split_two_fixed(
    participants: &[LayoutParticipant],
    axis: LayoutAxis,
    target_size: UVec2,
    padding: SplitScreenPadding,
    minimum_viewport_size: UVec2,
) -> Vec<ViewPlan> {
    let usable = math::usable_target_size(target_size, padding);
    let (first, second) = order_two_participants(participants, axis);
    let min_fraction = viewport_min_fraction(axis, minimum_viewport_size, usable, 2);
    let ratio = 0.5_f32.clamp(min_fraction, 1.0 - min_fraction);
    plans_from_two_split(first, second, axis, ratio)
}

fn split_two_dynamic(
    participants: &[LayoutParticipant],
    axis: LayoutAxis,
    owner: LocalPlayerSlot,
    alpha: f32,
    target_size: UVec2,
    padding: SplitScreenPadding,
    minimum_viewport_size: UVec2,
) -> (Vec<ViewPlan>, f32) {
    let usable = math::usable_target_size(target_size, padding);
    let (first, second) = order_two_participants(participants, axis);
    let total_weight = (first.area_weight + second.area_weight).max(0.001);
    let min_fraction = viewport_min_fraction(axis, minimum_viewport_size, usable, 2);
    let desired_ratio = (first.area_weight / total_weight).clamp(min_fraction, 1.0 - min_fraction);
    let start_ratio = if first.slot == owner { 1.0 } else { 0.0 };
    let ratio = start_ratio + (desired_ratio - start_ratio) * alpha;
    (plans_from_two_split(first, second, axis, ratio), ratio)
}

fn plans_from_two_split(
    first: LayoutParticipant,
    second: LayoutParticipant,
    axis: LayoutAxis,
    ratio: f32,
) -> Vec<ViewPlan> {
    let (first_rect, second_rect) = match axis {
        LayoutAxis::Vertical => (
            NormalizedRect::from_min_max(Vec2::ZERO, Vec2::new(ratio, 1.0)),
            NormalizedRect::from_min_max(Vec2::new(ratio, 0.0), Vec2::ONE),
        ),
        LayoutAxis::Horizontal => (
            NormalizedRect::from_min_max(Vec2::ZERO, Vec2::new(1.0, ratio)),
            NormalizedRect::from_min_max(Vec2::new(0.0, ratio), Vec2::ONE),
        ),
    };
    vec![
        ViewPlan {
            slot: first.slot,
            active: first_rect.width() > 0.0 && first_rect.height() > 0.0,
            rect: first_rect,
            area_weight: first.area_weight,
        },
        ViewPlan {
            slot: second.slot,
            active: second_rect.width() > 0.0 && second_rect.height() > 0.0,
            rect: second_rect,
            area_weight: second.area_weight,
        },
    ]
}

fn order_two_participants(
    participants: &[LayoutParticipant],
    axis: LayoutAxis,
) -> (LayoutParticipant, LayoutParticipant) {
    let mut ordered = participants.to_vec();
    match axis {
        LayoutAxis::Vertical => ordered.sort_by(|left, right| {
            left.position
                .x
                .total_cmp(&right.position.x)
                .then(left.slot.cmp(&right.slot))
        }),
        LayoutAxis::Horizontal => ordered.sort_by(|left, right| {
            right
                .position
                .y
                .total_cmp(&left.position.y)
                .then(left.slot.cmp(&right.slot))
        }),
    }
    (ordered[0], ordered[1])
}

fn split_three_fixed(
    participants: &[LayoutParticipant],
    layout: SplitScreenThreePlayerLayout,
    target_size: UVec2,
    padding: SplitScreenPadding,
    balance_policy: SplitScreenBalancePolicy,
    minimum_viewport_size: UVec2,
    strategy: SplitScreenMultiPlayerStrategy,
) -> Vec<ViewPlan> {
    let mut ordered = participants.to_vec();
    ordered.sort_by_key(|left| left.slot);
    let primary_index = choose_primary_index(&ordered, balance_policy);
    let primary = ordered.remove(primary_index);
    let usable = math::usable_target_size(target_size, padding);

    match layout {
        SplitScreenThreePlayerLayout::Auto | SplitScreenThreePlayerLayout::WideTop => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    left.position
                        .x
                        .total_cmp(&right.position.x)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            let top_min =
                viewport_min_fraction(LayoutAxis::Horizontal, minimum_viewport_size, usable, 2);
            let top_fraction = (primary.area_weight / total_area_weight(participants))
                .clamp(top_min, 1.0 - top_min);
            let bottom_split = secondary_split_fraction(
                balance_policy,
                ordered[0].area_weight,
                ordered[1].area_weight,
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2),
            );
            vec![
                ViewPlan {
                    slot: primary.slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(Vec2::ZERO, Vec2::new(1.0, top_fraction)),
                    area_weight: primary.area_weight,
                },
                ViewPlan {
                    slot: ordered[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(0.0, top_fraction),
                        Vec2::new(bottom_split, 1.0),
                    ),
                    area_weight: ordered[0].area_weight,
                },
                ViewPlan {
                    slot: ordered[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(bottom_split, top_fraction),
                        Vec2::ONE,
                    ),
                    area_weight: ordered[1].area_weight,
                },
            ]
        }
        SplitScreenThreePlayerLayout::WideBottom => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    left.position
                        .x
                        .total_cmp(&right.position.x)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            let top_min =
                viewport_min_fraction(LayoutAxis::Horizontal, minimum_viewport_size, usable, 2);
            let top_fraction = (1.0 - primary.area_weight / total_area_weight(participants))
                .clamp(top_min, 1.0 - top_min);
            let top_split = secondary_split_fraction(
                balance_policy,
                ordered[0].area_weight,
                ordered[1].area_weight,
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2),
            );
            vec![
                ViewPlan {
                    slot: ordered[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::ZERO,
                        Vec2::new(top_split, top_fraction),
                    ),
                    area_weight: ordered[0].area_weight,
                },
                ViewPlan {
                    slot: ordered[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(top_split, 0.0),
                        Vec2::new(1.0, top_fraction),
                    ),
                    area_weight: ordered[1].area_weight,
                },
                ViewPlan {
                    slot: primary.slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(Vec2::new(0.0, top_fraction), Vec2::ONE),
                    area_weight: primary.area_weight,
                },
            ]
        }
        SplitScreenThreePlayerLayout::WideLeft => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    right
                        .position
                        .y
                        .total_cmp(&left.position.y)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            let left_min =
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2);
            let left_fraction = (primary.area_weight / total_area_weight(participants))
                .clamp(left_min, 1.0 - left_min);
            let right_split = secondary_split_fraction(
                balance_policy,
                ordered[0].area_weight,
                ordered[1].area_weight,
                viewport_min_fraction(LayoutAxis::Horizontal, minimum_viewport_size, usable, 2),
            );
            vec![
                ViewPlan {
                    slot: primary.slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(Vec2::ZERO, Vec2::new(left_fraction, 1.0)),
                    area_weight: primary.area_weight,
                },
                ViewPlan {
                    slot: ordered[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(left_fraction, 0.0),
                        Vec2::new(1.0, right_split),
                    ),
                    area_weight: ordered[0].area_weight,
                },
                ViewPlan {
                    slot: ordered[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(left_fraction, right_split),
                        Vec2::ONE,
                    ),
                    area_weight: ordered[1].area_weight,
                },
            ]
        }
        SplitScreenThreePlayerLayout::WideRight => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    right
                        .position
                        .y
                        .total_cmp(&left.position.y)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            let left_min =
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2);
            let left_fraction = (1.0 - primary.area_weight / total_area_weight(participants))
                .clamp(left_min, 1.0 - left_min);
            let left_split = secondary_split_fraction(
                balance_policy,
                ordered[0].area_weight,
                ordered[1].area_weight,
                viewport_min_fraction(LayoutAxis::Horizontal, minimum_viewport_size, usable, 2),
            );
            vec![
                ViewPlan {
                    slot: ordered[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::ZERO,
                        Vec2::new(left_fraction, left_split),
                    ),
                    area_weight: ordered[0].area_weight,
                },
                ViewPlan {
                    slot: ordered[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(0.0, left_split),
                        Vec2::new(left_fraction, 1.0),
                    ),
                    area_weight: ordered[1].area_weight,
                },
                ViewPlan {
                    slot: primary.slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(Vec2::new(left_fraction, 0.0), Vec2::ONE),
                    area_weight: primary.area_weight,
                },
            ]
        }
    }
}

fn split_four_fixed(
    participants: &[LayoutParticipant],
    layout: SplitScreenFourPlayerLayout,
    target_size: UVec2,
    padding: SplitScreenPadding,
    balance_policy: SplitScreenBalancePolicy,
    minimum_viewport_size: UVec2,
    strategy: SplitScreenMultiPlayerStrategy,
) -> Vec<ViewPlan> {
    let mut ordered = participants.to_vec();
    if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
        ordered.sort_by(|left, right| {
            right
                .position
                .y
                .total_cmp(&left.position.y)
                .then(left.position.x.total_cmp(&right.position.x))
                .then(left.slot.cmp(&right.slot))
        });
    }
    let usable = math::usable_target_size(target_size, padding);

    match layout {
        SplitScreenFourPlayerLayout::Auto | SplitScreenFourPlayerLayout::Grid => {
            let top = &ordered[..2];
            let bottom = &ordered[2..];
            let row_split = secondary_split_fraction(
                balance_policy,
                top[0].area_weight + top[1].area_weight,
                bottom[0].area_weight + bottom[1].area_weight,
                viewport_min_fraction(LayoutAxis::Horizontal, minimum_viewport_size, usable, 2),
            );
            let top_split = secondary_split_fraction(
                balance_policy,
                top[0].area_weight,
                top[1].area_weight,
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2),
            );
            let bottom_split = secondary_split_fraction(
                balance_policy,
                bottom[0].area_weight,
                bottom[1].area_weight,
                viewport_min_fraction(LayoutAxis::Vertical, minimum_viewport_size, usable, 2),
            );
            vec![
                ViewPlan {
                    slot: top[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(Vec2::ZERO, Vec2::new(top_split, row_split)),
                    area_weight: top[0].area_weight,
                },
                ViewPlan {
                    slot: top[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(top_split, 0.0),
                        Vec2::new(1.0, row_split),
                    ),
                    area_weight: top[1].area_weight,
                },
                ViewPlan {
                    slot: bottom[0].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(0.0, row_split),
                        Vec2::new(bottom_split, 1.0),
                    ),
                    area_weight: bottom[0].area_weight,
                },
                ViewPlan {
                    slot: bottom[1].slot,
                    active: true,
                    rect: NormalizedRect::from_min_max(
                        Vec2::new(bottom_split, row_split),
                        Vec2::ONE,
                    ),
                    area_weight: bottom[1].area_weight,
                },
            ]
        }
        SplitScreenFourPlayerLayout::VerticalStrip => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    left.position
                        .x
                        .total_cmp(&right.position.x)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            strip_plans(
                ordered,
                LayoutAxis::Vertical,
                usable,
                balance_policy,
                minimum_viewport_size,
            )
        }
        SplitScreenFourPlayerLayout::HorizontalStrip => {
            if matches!(strategy, SplitScreenMultiPlayerStrategy::Hybrid) {
                ordered.sort_by(|left, right| {
                    right
                        .position
                        .y
                        .total_cmp(&left.position.y)
                        .then(left.slot.cmp(&right.slot))
                });
            }
            strip_plans(
                ordered,
                LayoutAxis::Horizontal,
                usable,
                balance_policy,
                minimum_viewport_size,
            )
        }
    }
}

fn strip_plans(
    participants: Vec<LayoutParticipant>,
    axis: LayoutAxis,
    usable: UVec2,
    balance_policy: SplitScreenBalancePolicy,
    minimum_viewport_size: UVec2,
) -> Vec<ViewPlan> {
    let min_fraction =
        viewport_min_fraction(axis, minimum_viewport_size, usable, participants.len());
    let weights: Vec<f32> = participants
        .iter()
        .map(|participant| match balance_policy {
            SplitScreenBalancePolicy::Uniform => 1.0,
            SplitScreenBalancePolicy::Weighted => participant.area_weight.max(0.1),
        })
        .collect();
    let fractions = allocate_strip_fractions(&weights, min_fraction);
    let mut cursor = 0.0;
    participants
        .into_iter()
        .zip(fractions)
        .map(|(participant, fraction)| {
            let start = cursor;
            cursor = (cursor + fraction).min(1.0);
            let rect = match axis {
                LayoutAxis::Vertical => {
                    NormalizedRect::from_min_max(Vec2::new(start, 0.0), Vec2::new(cursor, 1.0))
                }
                LayoutAxis::Horizontal => {
                    NormalizedRect::from_min_max(Vec2::new(0.0, start), Vec2::new(1.0, cursor))
                }
            };
            ViewPlan {
                slot: participant.slot,
                active: true,
                rect,
                area_weight: participant.area_weight,
            }
        })
        .collect()
}

fn choose_primary_index(
    participants: &[LayoutParticipant],
    balance_policy: SplitScreenBalancePolicy,
) -> usize {
    match balance_policy {
        SplitScreenBalancePolicy::Uniform => 0,
        SplitScreenBalancePolicy::Weighted => participants
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.area_weight
                    .total_cmp(&right.area_weight)
                    .then(left.slot.cmp(&right.slot).reverse())
            })
            .map(|(index, _)| index)
            .unwrap_or(0),
    }
}

fn total_area_weight(participants: &[LayoutParticipant]) -> f32 {
    participants
        .iter()
        .map(|participant| participant.area_weight.max(0.1))
        .sum::<f32>()
        .max(0.001)
}

fn weighted_split_fraction(first: f32, second: f32, min_fraction: f32) -> f32 {
    let total = (first + second).max(0.001);
    (first / total).clamp(min_fraction, 1.0 - min_fraction)
}

fn secondary_split_fraction(
    balance_policy: SplitScreenBalancePolicy,
    first: f32,
    second: f32,
    min_fraction: f32,
) -> f32 {
    match balance_policy {
        SplitScreenBalancePolicy::Uniform => 0.5_f32.clamp(min_fraction, 1.0 - min_fraction),
        SplitScreenBalancePolicy::Weighted => weighted_split_fraction(first, second, min_fraction),
    }
}

fn viewport_min_fraction(
    axis: LayoutAxis,
    minimum_viewport_size: UVec2,
    usable: UVec2,
    segments: usize,
) -> f32 {
    if segments == 0 {
        return 0.0;
    }

    let total_pixels = match axis {
        LayoutAxis::Vertical => usable.x,
        LayoutAxis::Horizontal => usable.y,
    };
    if total_pixels == 0 {
        return 0.0;
    }

    let requested = match axis {
        LayoutAxis::Vertical => {
            math::fraction_from_min_size(minimum_viewport_size.x.max(1), total_pixels)
        }
        LayoutAxis::Horizontal => {
            math::fraction_from_min_size(minimum_viewport_size.y.max(1), total_pixels)
        }
    };
    let even = 1.0 / segments as f32;

    if requested * segments as f32 >= 1.0 {
        even
    } else {
        requested.clamp(0.0, even)
    }
}

fn allocate_strip_fractions(weights: &[f32], min_fraction: f32) -> Vec<f32> {
    if weights.is_empty() {
        return Vec::new();
    }

    let count = weights.len();
    let even = 1.0 / count as f32;
    let safe_min = min_fraction.min(even);
    let base_total = safe_min * count as f32;
    if base_total >= 1.0 - 0.0001 {
        return vec![even; count];
    }

    let remaining = 1.0 - base_total;
    let total_weight = weights.iter().map(|weight| weight.max(0.0)).sum::<f32>();
    if total_weight <= 0.0001 {
        return vec![even; count];
    }

    weights
        .iter()
        .map(|weight| safe_min + remaining * weight.max(0.0) / total_weight)
        .collect()
}

fn fixed_divider_snapshot(
    context: LayoutContext<'_>,
    axis: LayoutAxis,
) -> Option<SplitScreenDividerSnapshot> {
    if !context.config.divider.show_seam {
        return None;
    }
    let (normalized_start, normalized_end) = match axis {
        LayoutAxis::Vertical => (Vec2::new(0.5, 0.0), Vec2::new(0.5, 1.0)),
        LayoutAxis::Horizontal => (Vec2::new(0.0, 0.5), Vec2::new(1.0, 0.5)),
    };
    Some(SplitScreenDividerSnapshot {
        physical_start: math::normalized_point_to_physical(
            normalized_start,
            context.target_size,
            context.config.safe_area_padding,
        ),
        physical_end: math::normalized_point_to_physical(
            normalized_end,
            context.target_size,
            context.config.safe_area_padding,
        ),
        normalized_start,
        normalized_end,
        thickness: context.config.divider.width,
        feather: context.config.divider.feather,
        color: context.config.divider.color,
    })
}

fn dynamic_divider_snapshot(
    context: LayoutContext<'_>,
    axis: LayoutAxis,
    ratio: f32,
    delta: Vec2,
    alpha: f32,
) -> Option<SplitScreenDividerSnapshot> {
    if !context.config.divider.show_seam || alpha <= 0.0 {
        return None;
    }

    let direction = if delta.length_squared() <= 0.0001 {
        match axis {
            LayoutAxis::Vertical => Vec2::Y,
            LayoutAxis::Horizontal => Vec2::X,
        }
    } else {
        Vec2::new(-delta.y, delta.x).normalize()
    };
    let center = match axis {
        LayoutAxis::Vertical => Vec2::new(ratio, 0.5),
        LayoutAxis::Horizontal => Vec2::new(0.5, ratio),
    };
    let (normalized_start, normalized_end) = clipped_unit_square_segment(center, direction)
        .unwrap_or_else(|| match axis {
            LayoutAxis::Vertical => (Vec2::new(ratio, 0.0), Vec2::new(ratio, 1.0)),
            LayoutAxis::Horizontal => (Vec2::new(0.0, ratio), Vec2::new(1.0, ratio)),
        });

    Some(SplitScreenDividerSnapshot {
        physical_start: math::normalized_point_to_physical(
            normalized_start,
            context.target_size,
            context.config.safe_area_padding,
        ),
        physical_end: math::normalized_point_to_physical(
            normalized_end,
            context.target_size,
            context.config.safe_area_padding,
        ),
        normalized_start,
        normalized_end,
        thickness: context.config.divider.width * alpha.max(0.2),
        feather: context.config.divider.feather,
        color: context.config.divider.color,
    })
}

fn clipped_unit_square_segment(origin: Vec2, direction: Vec2) -> Option<(Vec2, Vec2)> {
    let mut hits = Vec::new();
    let epsilon = 0.0001;

    if direction.x.abs() > epsilon {
        for boundary_x in [0.0, 1.0] {
            let t = (boundary_x - origin.x) / direction.x;
            let point = origin + direction * t;
            if (-epsilon..=1.0 + epsilon).contains(&point.y) {
                hits.push(point.clamp(Vec2::ZERO, Vec2::ONE));
            }
        }
    }
    if direction.y.abs() > epsilon {
        for boundary_y in [0.0, 1.0] {
            let t = (boundary_y - origin.y) / direction.y;
            let point = origin + direction * t;
            if (-epsilon..=1.0 + epsilon).contains(&point.x) {
                hits.push(point.clamp(Vec2::ZERO, Vec2::ONE));
            }
        }
    }

    hits.sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
    hits.dedup_by(|left, right| left.distance_squared(*right) <= 0.0001);

    if hits.len() >= 2 {
        Some((hits[0], *hits.last().unwrap()))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
