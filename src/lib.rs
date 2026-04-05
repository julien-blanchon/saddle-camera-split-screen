mod components;
mod config;
mod layout;
mod math;
mod messages;
mod systems;

pub use components::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenTarget, SplitScreenUiRoot, SplitScreenView,
};
pub use config::{
    SplitScreenAspectPolicy, SplitScreenBalancePolicy, SplitScreenBorderConfig, SplitScreenConfig,
    SplitScreenDebugConfig, SplitScreenDividerStyle, SplitScreenFourPlayerConfig,
    SplitScreenFourPlayerLayout, SplitScreenLetterboxConfig, SplitScreenLetterboxPolicy,
    SplitScreenMode, SplitScreenMultiPlayerStrategy, SplitScreenPadding,
    SplitScreenProjectionPlane, SplitScreenThreePlayerConfig, SplitScreenThreePlayerLayout,
    SplitScreenTransitionConfig, SplitScreenTransitionEasing, SplitScreenTwoPlayerConfig,
    SplitScreenTwoPlayerLayout,
};
pub use layout::{
    NormalizedRect, PhysicalRect, SplitScreenDividerSnapshot, SplitScreenLayoutMode,
    SplitScreenLayoutSnapshot, SplitScreenRenderTargetInfo, SplitScreenRuntime,
    SplitScreenViewSnapshot,
};
pub use messages::{
    SplitScreenLayoutChanged, SplitScreenModeChanged, SplitScreenPlayerViewAssigned,
};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SplitScreenSystems {
    CollectTargets,
    ComputeLayout,
    ApplyViewports,
    SyncUi,
    Debug,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

#[derive(Resource, Default)]
struct SplitScreenRuntimeActive(bool);

pub struct SplitScreenPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl SplitScreenPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for SplitScreenPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for SplitScreenPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_resource::<SplitScreenRuntimeActive>()
            .init_resource::<SplitScreenConfig>()
            .init_resource::<SplitScreenRuntime>()
            .init_resource::<systems::SplitScreenInternalState>()
            .add_message::<SplitScreenLayoutChanged>()
            .add_message::<SplitScreenPlayerViewAssigned>()
            .add_message::<SplitScreenModeChanged>()
            .register_type::<LocalPlayerSlot>()
            .register_type::<NormalizedRect>()
            .register_type::<PhysicalRect>()
            .register_type::<SplitScreenAspectPolicy>()
            .register_type::<SplitScreenBalancePolicy>()
            .register_type::<SplitScreenBorderConfig>()
            .register_type::<SplitScreenLetterboxConfig>()
            .register_type::<SplitScreenLetterboxPolicy>()
            .register_type::<SplitScreenTransitionConfig>()
            .register_type::<SplitScreenTransitionEasing>()
            .register_type::<SplitScreenCamera>()
            .register_type::<SplitScreenConfig>()
            .register_type::<SplitScreenDebugConfig>()
            .register_type::<SplitScreenDividerSnapshot>()
            .register_type::<SplitScreenDividerStyle>()
            .register_type::<SplitScreenFourPlayerConfig>()
            .register_type::<SplitScreenFourPlayerLayout>()
            .register_type::<SplitScreenLayoutMode>()
            .register_type::<SplitScreenLayoutSnapshot>()
            .register_type::<SplitScreenMode>()
            .register_type::<SplitScreenMultiPlayerStrategy>()
            .register_type::<SplitScreenPadding>()
            .register_type::<SplitScreenProjectionPlane>()
            .register_type::<SplitScreenRenderTargetInfo>()
            .register_type::<SplitScreenRuntime>()
            .register_type::<SplitScreenTarget>()
            .register_type::<SplitScreenThreePlayerConfig>()
            .register_type::<SplitScreenThreePlayerLayout>()
            .register_type::<SplitScreenTwoPlayerConfig>()
            .register_type::<SplitScreenTwoPlayerLayout>()
            .register_type::<SplitScreenUiRoot>()
            .register_type::<SplitScreenView>()
            .register_type::<SplitScreenViewSnapshot>()
            .add_systems(self.activate_schedule, activate_runtime)
            .add_systems(
                self.deactivate_schedule,
                (
                    systems::restore_managed_cameras,
                    systems::clear_internal_state,
                    deactivate_runtime,
                )
                    .chain(),
            )
            .configure_sets(
                self.update_schedule,
                (
                    SplitScreenSystems::CollectTargets,
                    SplitScreenSystems::ComputeLayout,
                    SplitScreenSystems::ApplyViewports,
                    SplitScreenSystems::SyncUi,
                    SplitScreenSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::collect_scene_state.in_set(SplitScreenSystems::CollectTargets),
                    systems::compute_layouts.in_set(SplitScreenSystems::ComputeLayout),
                    systems::apply_camera_plans.in_set(SplitScreenSystems::ApplyViewports),
                    systems::sync_ui_roots.in_set(SplitScreenSystems::SyncUi),
                    systems::emit_debug_messages.in_set(SplitScreenSystems::Debug),
                )
                    .run_if(runtime_is_active),
            );
    }
}

fn activate_runtime(mut runtime: ResMut<SplitScreenRuntimeActive>) {
    runtime.0 = true;
}

fn deactivate_runtime(mut runtime: ResMut<SplitScreenRuntimeActive>) {
    runtime.0 = false;
}

fn runtime_is_active(runtime: Res<SplitScreenRuntimeActive>) -> bool {
    runtime.0
}
