use bevy::window::{PrimaryWindow, WindowResized};
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use super::*;
use crate::{SplitScreenPlugin, SplitScreenSystems};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Activate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Deactivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Tick;

#[derive(Resource, Default)]
struct MessageCounts {
    layouts: usize,
    modes: usize,
    assignments: usize,
}

fn tracking_app(config: crate::SplitScreenConfig) -> App {
    let mut app = App::new();
    app.init_schedule(Activate);
    app.init_schedule(Deactivate);
    app.init_schedule(Tick);
    app.add_plugins(MinimalPlugins);
    app.add_message::<WindowResized>();
    app.insert_resource(config);
    app.insert_resource(MessageCounts::default());
    app.add_plugins(SplitScreenPlugin::new(Activate, Deactivate, Tick));
    app.add_systems(Tick, track_messages.after(SplitScreenSystems::Debug));
    spawn_primary_window(app.world_mut(), 1280, 720);
    app
}

fn track_messages(
    mut counts: ResMut<MessageCounts>,
    mut layouts: MessageReader<crate::SplitScreenLayoutChanged>,
    mut modes: MessageReader<crate::SplitScreenModeChanged>,
    mut assignments: MessageReader<crate::SplitScreenPlayerViewAssigned>,
) {
    counts.layouts += layouts.read().count();
    counts.modes += modes.read().count();
    counts.assignments += assignments.read().count();
}

fn spawn_primary_window(world: &mut World, width: u32, height: u32) -> Entity {
    let mut window = Window::default();
    window
        .resolution
        .set_physical_resolution(width.max(1), height.max(1));
    window.resolution.set_scale_factor_override(Some(1.0));
    world.spawn((PrimaryWindow, window)).id()
}

fn spawn_target(world: &mut World, slot: u8, translation: Vec3) -> Entity {
    world
        .spawn((
            Name::new(format!("Target {}", slot + 1)),
            LocalPlayerSlot(slot),
            crate::SplitScreenTarget::default(),
            Transform::from_translation(translation),
            GlobalTransform::from_translation(translation),
        ))
        .id()
}

fn spawn_camera(world: &mut World, slot: u8, order: isize, area_weight: f32) -> Entity {
    world
        .spawn((
            Name::new(format!("Camera {}", slot + 1)),
            LocalPlayerSlot(slot),
            Camera2d,
            Camera { order, ..default() },
            crate::SplitScreenCamera::default(),
            crate::SplitScreenView { area_weight },
            Transform::from_xyz(0.0, 0.0, 999.0),
            GlobalTransform::from_xyz(0.0, 0.0, 999.0),
        ))
        .id()
}

fn spawn_ui_root(world: &mut World, slot: u8) -> Entity {
    world
        .spawn((
            Name::new(format!("UI Root {}", slot + 1)),
            LocalPlayerSlot(slot),
            crate::SplitScreenUiRoot,
        ))
        .id()
}

#[test]
fn shared_mode_routes_both_huds_to_the_merged_owner_camera() {
    let mut app = tracking_app(crate::SplitScreenConfig {
        two_player: crate::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 50.0,
            merge_outer_distance: 80.0,
            ..default()
        },
        ..default()
    });

    spawn_target(app.world_mut(), 0, Vec3::new(-16.0, 0.0, 0.0));
    spawn_target(app.world_mut(), 1, Vec3::new(16.0, 0.0, 0.0));
    let camera_a = spawn_camera(app.world_mut(), 0, 0, 1.0);
    let camera_b = spawn_camera(app.world_mut(), 1, 1, 2.0);
    let ui_a = spawn_ui_root(app.world_mut(), 0);
    let ui_b = spawn_ui_root(app.world_mut(), 1);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);

    assert!(!app.world().get::<Camera>(camera_a).unwrap().is_active);
    assert!(app.world().get::<Camera>(camera_b).unwrap().is_active);
    assert_eq!(app.world().get::<UiTargetCamera>(ui_a).unwrap().0, camera_b);
    assert_eq!(app.world().get::<UiTargetCamera>(ui_b).unwrap().0, camera_b);
}

#[test]
fn split_mode_applies_distinct_viewports_and_only_emits_messages_once_when_unchanged() {
    let mut app = tracking_app(crate::SplitScreenConfig {
        mode: crate::SplitScreenMode::DynamicOnly,
        two_player: crate::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 40.0,
            merge_outer_distance: 80.0,
            ..default()
        },
        ..default()
    });

    spawn_target(app.world_mut(), 0, Vec3::new(-220.0, -140.0, 0.0));
    spawn_target(app.world_mut(), 1, Vec3::new(220.0, 140.0, 0.0));
    let camera_a = spawn_camera(app.world_mut(), 0, 0, 1.0);
    let camera_b = spawn_camera(app.world_mut(), 1, 1, 1.0);
    spawn_ui_root(app.world_mut(), 0);
    spawn_ui_root(app.world_mut(), 1);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);

    let first_counts = app.world().resource::<MessageCounts>();
    assert_eq!(first_counts.layouts, 1);
    assert_eq!(first_counts.assignments, 2);

    let viewport_a = app
        .world()
        .get::<Camera>(camera_a)
        .unwrap()
        .viewport
        .clone()
        .unwrap();
    let viewport_b = app
        .world()
        .get::<Camera>(camera_b)
        .unwrap()
        .viewport
        .clone()
        .unwrap();
    assert_ne!(viewport_a.physical_position, viewport_b.physical_position);
    assert!(viewport_a.physical_size.x > 0);
    assert!(viewport_b.physical_size.x > 0);

    app.world_mut().run_schedule(Tick);

    let second_counts = app.world().resource::<MessageCounts>();
    assert_eq!(second_counts.layouts, 1);
    assert_eq!(second_counts.assignments, 2);
}

#[test]
fn resize_messages_update_runtime_and_viewports() {
    let mut app = tracking_app(crate::SplitScreenConfig {
        resize_debounce_frames: 0,
        mode: crate::SplitScreenMode::FixedOnly,
        ..default()
    });

    let window_entity = {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
        query.single(world).expect("primary window")
    };
    spawn_target(app.world_mut(), 0, Vec3::new(-200.0, 0.0, 0.0));
    spawn_target(app.world_mut(), 1, Vec3::new(200.0, 0.0, 0.0));
    let camera_a = spawn_camera(app.world_mut(), 0, 0, 1.0);
    spawn_camera(app.world_mut(), 1, 1, 1.0);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);
    let before = app
        .world()
        .get::<Camera>(camera_a)
        .unwrap()
        .viewport
        .clone()
        .unwrap()
        .physical_size;

    {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows.single_mut(world).expect("primary window");
        window.resolution.set_physical_resolution(900, 600);
        world.write_message(WindowResized {
            window: window_entity,
            width: 900.0,
            height: 600.0,
        });
    }

    app.world_mut().run_schedule(Tick);

    let after = app
        .world()
        .get::<Camera>(camera_a)
        .unwrap()
        .viewport
        .clone()
        .unwrap()
        .physical_size;
    assert_ne!(before, after);
    assert_eq!(
        app.world()
            .resource::<crate::SplitScreenRuntime>()
            .last_resize_window,
        Some(window_entity)
    );
}
