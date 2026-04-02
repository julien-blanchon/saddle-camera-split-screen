use bevy::prelude::*;

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenMode {
    #[default]
    Auto,
    SharedOnly,
    FixedOnly,
    DynamicOnly,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenProjectionPlane {
    #[default]
    Xy,
    Xz,
    Yz,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenAspectPolicy {
    #[default]
    MatchWindow,
    PreferVertical,
    PreferHorizontal,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenBalancePolicy {
    #[default]
    Uniform,
    Weighted,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenMultiPlayerStrategy {
    #[default]
    BalancedFixed,
    Hybrid,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenTwoPlayerLayout {
    #[default]
    Auto,
    Vertical,
    Horizontal,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenThreePlayerLayout {
    #[default]
    Auto,
    WideTop,
    WideBottom,
    WideLeft,
    WideRight,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitScreenFourPlayerLayout {
    #[default]
    Auto,
    Grid,
    VerticalStrip,
    HorizontalStrip,
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenPadding {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

impl Default for SplitScreenPadding {
    fn default() -> Self {
        Self {
            left: 16,
            right: 16,
            top: 16,
            bottom: 16,
        }
    }
}

#[derive(Reflect, Debug, Clone, PartialEq)]
pub struct SplitScreenDividerStyle {
    pub width: f32,
    pub feather: f32,
    pub color: Color,
    pub show_seam: bool,
}

impl Default for SplitScreenDividerStyle {
    fn default() -> Self {
        Self {
            width: 6.0,
            feather: 18.0,
            color: Color::srgba(0.97, 0.98, 1.0, 0.88),
            show_seam: true,
        }
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenTwoPlayerConfig {
    pub fixed_layout: SplitScreenTwoPlayerLayout,
    pub merge_inner_distance: f32,
    pub merge_outer_distance: f32,
    pub axis_hysteresis: f32,
}

impl Default for SplitScreenTwoPlayerConfig {
    fn default() -> Self {
        Self {
            fixed_layout: SplitScreenTwoPlayerLayout::Auto,
            merge_inner_distance: 5.0,
            merge_outer_distance: 9.0,
            axis_hysteresis: 0.18,
        }
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenThreePlayerConfig {
    pub layout: SplitScreenThreePlayerLayout,
    pub strategy: SplitScreenMultiPlayerStrategy,
}

impl Default for SplitScreenThreePlayerConfig {
    fn default() -> Self {
        Self {
            layout: SplitScreenThreePlayerLayout::Auto,
            strategy: SplitScreenMultiPlayerStrategy::Hybrid,
        }
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenFourPlayerConfig {
    pub layout: SplitScreenFourPlayerLayout,
    pub strategy: SplitScreenMultiPlayerStrategy,
}

impl Default for SplitScreenFourPlayerConfig {
    fn default() -> Self {
        Self {
            layout: SplitScreenFourPlayerLayout::Auto,
            strategy: SplitScreenMultiPlayerStrategy::Hybrid,
        }
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
pub struct SplitScreenDebugConfig {
    pub log_changes: bool,
    pub expose_snapshot: bool,
    pub draw_divider: bool,
    pub draw_viewport_bounds: bool,
    pub draw_targets: bool,
}

impl Default for SplitScreenDebugConfig {
    fn default() -> Self {
        Self {
            log_changes: false,
            expose_snapshot: true,
            draw_divider: true,
            draw_viewport_bounds: false,
            draw_targets: false,
        }
    }
}

#[derive(Resource, Reflect, Debug, Clone, PartialEq)]
#[reflect(Resource)]
pub struct SplitScreenConfig {
    pub max_players: u8,
    pub min_players: u8,
    pub mode: SplitScreenMode,
    pub default_projection: SplitScreenProjectionPlane,
    pub aspect_policy: SplitScreenAspectPolicy,
    pub balance_policy: SplitScreenBalancePolicy,
    pub two_player: SplitScreenTwoPlayerConfig,
    pub three_player: SplitScreenThreePlayerConfig,
    pub four_player: SplitScreenFourPlayerConfig,
    pub divider: SplitScreenDividerStyle,
    pub safe_area_padding: SplitScreenPadding,
    pub minimum_viewport_size: UVec2,
    pub resize_debounce_frames: u8,
    pub debug: SplitScreenDebugConfig,
}

impl Default for SplitScreenConfig {
    fn default() -> Self {
        Self {
            max_players: 4,
            min_players: 2,
            mode: SplitScreenMode::Auto,
            default_projection: SplitScreenProjectionPlane::Xy,
            aspect_policy: SplitScreenAspectPolicy::MatchWindow,
            balance_policy: SplitScreenBalancePolicy::Weighted,
            two_player: SplitScreenTwoPlayerConfig::default(),
            three_player: SplitScreenThreePlayerConfig::default(),
            four_player: SplitScreenFourPlayerConfig::default(),
            divider: SplitScreenDividerStyle::default(),
            safe_area_padding: SplitScreenPadding::default(),
            minimum_viewport_size: UVec2::new(220, 140),
            resize_debounce_frames: 2,
            debug: SplitScreenDebugConfig::default(),
        }
    }
}
