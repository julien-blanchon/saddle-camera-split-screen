use bevy::prelude::*;

use crate::{LocalPlayerSlot, SplitScreenLayoutMode, SplitScreenLayoutSnapshot};

#[derive(Message, Debug, Clone)]
pub struct SplitScreenLayoutChanged {
    pub snapshot: SplitScreenLayoutSnapshot,
}

#[derive(Message, Debug, Clone)]
pub struct SplitScreenPlayerViewAssigned {
    pub slot: LocalPlayerSlot,
    pub camera: Entity,
    pub window: Option<Entity>,
}

#[derive(Message, Debug, Clone)]
pub struct SplitScreenModeChanged {
    pub window: Option<Entity>,
    pub previous: SplitScreenLayoutMode,
    pub current: SplitScreenLayoutMode,
}
