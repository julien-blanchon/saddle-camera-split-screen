use std::collections::{BTreeMap, HashMap};

use bevy::{
    camera::{NormalizedRenderTarget, RenderTarget},
    prelude::*,
    ui::UiTargetCamera,
    window::{PrimaryWindow, WindowResized},
};

use crate::{
    LocalPlayerSlot, NormalizedRect, SplitScreenCamera, SplitScreenConfig,
    SplitScreenLayoutChanged, SplitScreenLayoutMode, SplitScreenModeChanged,
    SplitScreenPlayerViewAssigned, SplitScreenProjectionPlane, SplitScreenRuntime,
    SplitScreenTarget, SplitScreenUiRoot, SplitScreenView, SplitScreenViewSnapshot, layout, math,
};

#[derive(Resource, Default)]
pub(crate) struct SplitScreenInternalState {
    groups: Vec<CollectedGroup>,
    snapshots: BTreeMap<NormalizedRenderTarget, crate::SplitScreenLayoutSnapshot>,
    last_emitted_snapshots: BTreeMap<NormalizedRenderTarget, crate::SplitScreenLayoutSnapshot>,
    last_ui_targets: BTreeMap<LocalPlayerSlot, Entity>,
    resize_hold: HashMap<Entity, u8>,
    camera_plans: HashMap<Entity, CameraPlan>,
    ui_targets: HashMap<LocalPlayerSlot, Entity>,
    transition_from: HashMap<LocalPlayerSlot, NormalizedRect>,
    transition_to: HashMap<LocalPlayerSlot, NormalizedRect>,
    transition_progress: f32,
    transition_active: bool,
}

#[derive(Debug, Clone)]
struct CollectedGroup {
    target: NormalizedRenderTarget,
    target_window: Option<Entity>,
    target_size: UVec2,
    participants: Vec<CollectedParticipant>,
}

#[derive(Debug, Clone)]
struct CollectedParticipant {
    slot: LocalPlayerSlot,
    position: Vec2,
    area_weight: f32,
    primary_ui_camera: Option<Entity>,
    cameras: Vec<Entity>,
}

#[derive(Debug, Clone, Copy)]
struct CameraBinding {
    entity: Entity,
    order: isize,
    ui_anchor: bool,
}

#[derive(Debug, Clone)]
struct CameraPlan {
    active: bool,
    viewport: Option<bevy::camera::Viewport>,
}

pub(crate) fn clear_internal_state(
    mut internal: ResMut<SplitScreenInternalState>,
    mut runtime: ResMut<SplitScreenRuntime>,
) {
    internal.groups.clear();
    internal.snapshots.clear();
    internal.last_emitted_snapshots.clear();
    internal.last_ui_targets.clear();
    internal.resize_hold.clear();
    internal.camera_plans.clear();
    internal.ui_targets.clear();
    internal.transition_from.clear();
    internal.transition_to.clear();
    internal.transition_progress = 0.0;
    internal.transition_active = false;
    runtime.active = false;
    runtime.snapshots.clear();
    runtime.transition_progress = 0.0;
    runtime.transition_active = false;
}

pub(crate) fn restore_managed_cameras(mut cameras: Query<&mut Camera, With<SplitScreenCamera>>) {
    for mut camera in &mut cameras {
        camera.viewport = None;
        camera.is_active = true;
    }
}

pub(crate) fn collect_scene_state(
    config: Res<SplitScreenConfig>,
    mut resized: MessageReader<WindowResized>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
    target_query: Query<(&LocalPlayerSlot, &SplitScreenTarget, &GlobalTransform)>,
    view_query: Query<(&LocalPlayerSlot, &SplitScreenView)>,
    camera_query: Query<(
        Entity,
        &LocalPlayerSlot,
        &SplitScreenCamera,
        &Camera,
        &RenderTarget,
        &GlobalTransform,
    )>,
    mut internal: ResMut<SplitScreenInternalState>,
    mut runtime: ResMut<SplitScreenRuntime>,
) {
    let primary_window = primary_window.iter().next();
    for event in resized.read() {
        if config.resize_debounce_frames > 0 {
            internal
                .resize_hold
                .insert(event.window, config.resize_debounce_frames);
        } else {
            internal.resize_hold.remove(&event.window);
        }
        runtime.last_resize_window = Some(event.window);
    }
    for hold in internal.resize_hold.values_mut() {
        *hold = hold.saturating_sub(1);
    }

    let mut target_positions = HashMap::<LocalPlayerSlot, (Vec2, f32)>::new();
    for (slot, target, transform) in &target_query {
        let world = transform.translation() + target.anchor_offset;
        let projected = project_vec3(
            world,
            target.projection.unwrap_or(config.default_projection),
        );
        let weight = target.weight.max(0.01);
        let entry = target_positions.entry(*slot).or_insert((Vec2::ZERO, 0.0));
        entry.0 += projected * weight;
        entry.1 += weight;
    }

    let mut area_weights = HashMap::<LocalPlayerSlot, f32>::new();
    for (slot, view) in &view_query {
        area_weights
            .entry(*slot)
            .and_modify(|current| *current = current.max(view.area_weight.max(0.1)))
            .or_insert(view.area_weight.max(0.1));
    }

    #[derive(Default)]
    struct ParticipantBuilder {
        fallback_position: Vec2,
        area_weight: f32,
        primary_ui_camera: Option<CameraBinding>,
        cameras: Vec<CameraBinding>,
    }

    #[derive(Default)]
    struct GroupBuilder {
        target_window: Option<Entity>,
        target_size: UVec2,
        participants: BTreeMap<LocalPlayerSlot, ParticipantBuilder>,
    }

    let mut groups = BTreeMap::<NormalizedRenderTarget, GroupBuilder>::new();
    for (entity, slot, split_camera, camera, render_target, transform) in &camera_query {
        let Some(target) = render_target.normalize(primary_window) else {
            continue;
        };
        let target_size = target_window_size(&target, &windows, camera).unwrap_or(UVec2::ZERO);
        let group = groups.entry(target.clone()).or_default();
        group.target_window = normalized_target_window(&target);
        if target_size != UVec2::ZERO {
            group.target_size = target_size;
        }

        let participant = group.participants.entry(*slot).or_default();
        participant.fallback_position =
            project_vec3(transform.translation(), config.default_projection);
        participant.area_weight = area_weights.get(slot).copied().unwrap_or(1.0);
        let binding = CameraBinding {
            entity,
            order: camera.order,
            ui_anchor: split_camera.ui_anchor,
        };
        participant.cameras.push(binding);
        let current = participant.primary_ui_camera;
        participant.primary_ui_camera = Some(match current {
            Some(existing)
                if existing.ui_anchor && !binding.ui_anchor
                    || (existing.ui_anchor == binding.ui_anchor
                        && existing.order <= binding.order) =>
            {
                existing
            }
            _ => binding,
        });
    }

    internal.groups = groups
        .into_iter()
        .map(|(target, builder)| CollectedGroup {
            target,
            target_window: builder.target_window,
            target_size: builder.target_size,
            participants: builder
                .participants
                .into_iter()
                .map(|(slot, participant)| {
                    let position = target_positions
                        .get(&slot)
                        .map(|(sum, total)| *sum / total.max(0.001))
                        .unwrap_or(participant.fallback_position);
                    CollectedParticipant {
                        slot,
                        position,
                        area_weight: participant.area_weight.max(0.1),
                        primary_ui_camera: participant
                            .primary_ui_camera
                            .map(|binding| binding.entity),
                        cameras: participant
                            .cameras
                            .into_iter()
                            .map(|binding| binding.entity)
                            .collect(),
                    }
                })
                .collect(),
        })
        .collect();
}

pub(crate) fn compute_layouts(
    config: Res<SplitScreenConfig>,
    time: Res<Time>,
    mut internal: ResMut<SplitScreenInternalState>,
    mut runtime: ResMut<SplitScreenRuntime>,
) {
    internal.snapshots.clear();
    internal.camera_plans.clear();
    internal.ui_targets.clear();
    runtime.active = true;
    runtime.frame_serial = runtime.frame_serial.saturating_add(1);
    runtime.snapshots.clear();

    let groups = internal.groups.clone();
    let previous_snapshots = internal.last_emitted_snapshots.clone();

    if internal.transition_active && config.transition.enabled {
        let dt = time.delta_secs();
        let duration = config.transition.duration_seconds.max(0.01);
        internal.transition_progress = (internal.transition_progress + dt / duration).min(1.0);
        if internal.transition_progress >= 1.0 {
            internal.transition_active = false;
            internal.transition_from.clear();
            internal.transition_to.clear();
        }
    }

    for group in &groups {
        let layout_participants: Vec<_> = group
            .participants
            .iter()
            .map(|participant| layout::LayoutParticipant {
                slot: participant.slot,
                position: participant.position,
                area_weight: participant.area_weight,
            })
            .collect();
        let previous_mode = previous_snapshots
            .get(&group.target)
            .map(|snapshot| snapshot.mode);
        let mut snapshot = layout::build_layout_snapshot(layout::LayoutContext {
            target_window: group.target_window,
            target_size: group.target_size,
            participants: &layout_participants,
            previous_mode,
            config: &config,
        });

        let prev_views = previous_snapshots.get(&group.target);
        let layout_changed = prev_views.is_some_and(|prev| {
            prev.views.len() != snapshot.views.len()
                || prev.mode != snapshot.mode
                || prev.views.iter().zip(&snapshot.views).any(|(left, right)| {
                    left.slot != right.slot
                        || (left.normalized.min - right.normalized.min).length() > 0.01
                        || (left.normalized.max - right.normalized.max).length() > 0.01
                })
        });

        if layout_changed && config.transition.enabled && !internal.transition_active {
            internal.transition_from.clear();
            internal.transition_to.clear();
            if let Some(prev) = prev_views {
                for view in &prev.views {
                    internal.transition_from.insert(view.slot, view.normalized);
                }
            }
            for view in &snapshot.views {
                internal.transition_to.insert(view.slot, view.normalized);
            }
            internal.transition_progress = 0.0;
            internal.transition_active = true;
        }

        if internal.transition_active && config.transition.enabled {
            let eased_t = math::ease(internal.transition_progress, config.transition.easing);
            for view in &mut snapshot.views {
                let from = internal
                    .transition_from
                    .get(&view.slot)
                    .copied()
                    .unwrap_or(NormalizedRect::zero());
                let to = internal
                    .transition_to
                    .get(&view.slot)
                    .copied()
                    .unwrap_or(view.normalized);
                let lerped = math::lerp_normalized_rect(from, to, eased_t);
                view.normalized = lerped;
                view.physical = math::normalized_to_physical(
                    lerped,
                    group.target_size,
                    config.safe_area_padding,
                );
                if let Some(target_ratio) = config.letterbox.policy.target_aspect_ratio() {
                    view.letterboxed_physical =
                        Some(math::letterbox_physical(view.physical, target_ratio));
                }
                view.active = lerped.width() > 0.01 && lerped.height() > 0.01;
            }
            snapshot.transition_alpha = eased_t;
        }

        runtime.transition_progress = internal.transition_progress;
        runtime.transition_active = internal.transition_active;

        let merged_target = snapshot.merged_owner.and_then(|slot| {
            group
                .participants
                .iter()
                .find(|participant| participant.slot == slot)
                .and_then(|participant| participant.primary_ui_camera)
        });

        for participant in &group.participants {
            let Some(view_snapshot) = snapshot
                .views
                .iter()
                .find(|view| view.slot == participant.slot)
            else {
                continue;
            };

            let ui_camera = if snapshot.mode == SplitScreenLayoutMode::Shared {
                merged_target
            } else {
                participant.primary_ui_camera
            };
            if let Some(camera) = ui_camera {
                internal.ui_targets.insert(participant.slot, camera);
            }

            let effective_view = if view_snapshot.letterboxed_physical.is_some() {
                let mut modified = view_snapshot.clone();
                if let Some(lb) = modified.letterboxed_physical {
                    modified.physical = lb;
                }
                modified
            } else {
                view_snapshot.clone()
            };
            let viewport = viewport_from_snapshot(&effective_view);
            for camera in &participant.cameras {
                internal.camera_plans.insert(
                    *camera,
                    CameraPlan {
                        active: effective_view.active,
                        viewport: viewport.clone(),
                    },
                );
            }
        }

        internal
            .snapshots
            .insert(group.target.clone(), snapshot.clone());
        if config.debug.expose_snapshot {
            runtime.snapshots.push(snapshot);
        }
    }
}

pub(crate) fn apply_camera_plans(
    mut cameras: Query<(Entity, &mut Camera), With<SplitScreenCamera>>,
    internal: Res<SplitScreenInternalState>,
) {
    for (entity, mut camera) in &mut cameras {
        let Some(plan) = internal.camera_plans.get(&entity).cloned() else {
            camera.is_active = true;
            camera.viewport = None;
            continue;
        };
        camera.is_active = plan.active;
        camera.viewport = plan.viewport;
    }
}

pub(crate) fn sync_ui_roots(
    ui_roots: Query<(Entity, &LocalPlayerSlot), With<SplitScreenUiRoot>>,
    internal: Res<SplitScreenInternalState>,
    mut commands: Commands,
) {
    for (entity, slot) in &ui_roots {
        match internal.ui_targets.get(slot).copied() {
            Some(target_camera) => {
                commands
                    .entity(entity)
                    .insert(UiTargetCamera(target_camera));
            }
            None => {
                commands.entity(entity).remove::<UiTargetCamera>();
            }
        }
    }
}

pub(crate) fn emit_debug_messages(
    config: Res<SplitScreenConfig>,
    mut internal: ResMut<SplitScreenInternalState>,
    mut layout_writer: MessageWriter<SplitScreenLayoutChanged>,
    mut assignment_writer: MessageWriter<SplitScreenPlayerViewAssigned>,
    mut mode_writer: MessageWriter<SplitScreenModeChanged>,
) {
    for (target, snapshot) in &internal.snapshots {
        let hold_active = snapshot
            .target
            .window
            .and_then(|window| internal.resize_hold.get(&window))
            .copied()
            .unwrap_or_default()
            > 0;

        let previous = internal.last_emitted_snapshots.get(target);
        if !hold_active && layout::layout_materially_changed(previous, snapshot) {
            if config.debug.log_changes {
                info!(
                    "split_screen layout changed: mode={:?} views={} target={:?}",
                    snapshot.mode,
                    snapshot.views.len(),
                    snapshot.target.window
                );
            }
            layout_writer.write(SplitScreenLayoutChanged {
                snapshot: snapshot.clone(),
            });
            if let Some(previous) = previous {
                if previous.mode != snapshot.mode {
                    mode_writer.write(SplitScreenModeChanged {
                        window: snapshot.target.window,
                        previous: previous.mode,
                        current: snapshot.mode,
                    });
                }
            }
        }
    }

    for (slot, camera) in &internal.ui_targets {
        if internal.last_ui_targets.get(slot).copied() != Some(*camera) {
            let window = internal
                .snapshots
                .values()
                .find(|snapshot| snapshot.views.iter().any(|view| view.slot == *slot))
                .and_then(|snapshot| snapshot.target.window);
            assignment_writer.write(SplitScreenPlayerViewAssigned {
                slot: *slot,
                camera: *camera,
                window,
            });
        }
    }

    internal.last_emitted_snapshots = internal.snapshots.clone();
    internal.last_ui_targets = internal
        .ui_targets
        .iter()
        .map(|(slot, camera)| (*slot, *camera))
        .collect();
}

fn viewport_from_snapshot(view: &SplitScreenViewSnapshot) -> Option<bevy::camera::Viewport> {
    if !view.active {
        return None;
    }
    Some(bevy::camera::Viewport {
        physical_position: view.physical.position,
        physical_size: view.physical.size,
        ..default()
    })
}

fn project_vec3(value: Vec3, projection: SplitScreenProjectionPlane) -> Vec2 {
    match projection {
        SplitScreenProjectionPlane::Xy => value.xy(),
        SplitScreenProjectionPlane::Xz => Vec2::new(value.x, value.z),
        SplitScreenProjectionPlane::Yz => Vec2::new(value.y, value.z),
    }
}

fn normalized_target_window(target: &NormalizedRenderTarget) -> Option<Entity> {
    match target {
        NormalizedRenderTarget::Window(window) => Some(window.entity()),
        _ => None,
    }
}

fn target_window_size(
    target: &NormalizedRenderTarget,
    windows: &Query<&Window>,
    camera: &Camera,
) -> Option<UVec2> {
    match target {
        NormalizedRenderTarget::Window(window) => {
            windows.get(window.entity()).ok().map(Window::physical_size)
        }
        _ => camera.physical_target_size(),
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod systems_tests;
