use bevy::prelude::*;

use crate::SplitScreenProjectionPlane;

#[derive(Component, Reflect, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component, Debug, PartialEq, Hash, Clone)]
pub struct LocalPlayerSlot(pub u8);

impl LocalPlayerSlot {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Component, Reflect, Debug, Clone, PartialEq)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SplitScreenTarget {
    pub weight: f32,
    pub anchor_offset: Vec3,
    pub projection: Option<SplitScreenProjectionPlane>,
}

impl Default for SplitScreenTarget {
    fn default() -> Self {
        Self {
            weight: 1.0,
            anchor_offset: Vec3::ZERO,
            projection: None,
        }
    }
}

#[derive(Component, Reflect, Debug, Clone, PartialEq)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SplitScreenView {
    pub area_weight: f32,
}

impl Default for SplitScreenView {
    fn default() -> Self {
        Self { area_weight: 1.0 }
    }
}

#[derive(Component, Reflect, Debug, Clone, PartialEq)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SplitScreenCamera {
    pub ui_anchor: bool,
}

impl Default for SplitScreenCamera {
    fn default() -> Self {
        Self { ui_anchor: true }
    }
}

#[derive(Component, Reflect, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SplitScreenUiRoot;
