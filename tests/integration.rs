use bevy::window::{PrimaryWindow, WindowResized};
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use split_screen::{
    LocalPlayerSlot, SplitScreenConfig, SplitScreenLayoutMode, SplitScreenPlugin,
    SplitScreenRuntime, SplitScreenTarget, SplitScreenUiRoot, SplitScreenView,
};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Activate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Deactivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Tick;

fn integration_app(config: SplitScreenConfig) -> App {
    let mut app = App::new();
    app.init_schedule(Activate);
    app.init_schedule(Deactivate);
    app.init_schedule(Tick);
    app.add_plugins(MinimalPlugins);
    app.add_message::<WindowResized>();
    app.insert_resource(config);
    app.add_plugins(SplitScreenPlugin::new(Activate, Deactivate, Tick));
    app.configure_sets(Tick, split_screen::SplitScreenSystems::Debug);

    let mut window = Window::default();
    window.resolution.set_physical_resolution(1280, 720);
    window.resolution.set_scale_factor_override(Some(1.0));
    app.world_mut().spawn((PrimaryWindow, window));
    app
}

fn spawn_slot_target(app: &mut App, slot: u8, translation: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(format!("Target {}", slot + 1)),
            LocalPlayerSlot(slot),
            SplitScreenTarget::default(),
            Transform::from_translation(translation),
            GlobalTransform::from_translation(translation),
        ))
        .id()
}

fn spawn_slot_camera(app: &mut App, slot: u8, order: isize, area_weight: f32) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(format!("Camera {}", slot + 1)),
            LocalPlayerSlot(slot),
            Camera2d,
            Camera { order, ..default() },
            split_screen::SplitScreenCamera::default(),
            SplitScreenView { area_weight },
            Transform::from_xyz(0.0, 0.0, 999.0),
            GlobalTransform::from_xyz(0.0, 0.0, 999.0),
        ))
        .id()
}

fn spawn_ui_root(app: &mut App, slot: u8) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(format!("UI Root {}", slot + 1)),
            LocalPlayerSlot(slot),
            SplitScreenUiRoot,
        ))
        .id()
}

#[test]
fn plugin_builds_with_custom_schedules_and_applies_a_layout_on_the_first_tick() {
    let mut app = integration_app(SplitScreenConfig {
        mode: split_screen::SplitScreenMode::FixedOnly,
        ..default()
    });
    spawn_slot_target(&mut app, 0, Vec3::new(-220.0, 0.0, 0.0));
    spawn_slot_target(&mut app, 1, Vec3::new(220.0, 0.0, 0.0));
    let camera_a = spawn_slot_camera(&mut app, 0, 0, 1.0);
    let camera_b = spawn_slot_camera(&mut app, 1, 1, 1.0);
    spawn_ui_root(&mut app, 0);
    spawn_ui_root(&mut app, 1);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);

    let viewport_a = app
        .world()
        .get::<Camera>(camera_a)
        .unwrap()
        .viewport
        .clone();
    let viewport_b = app
        .world()
        .get::<Camera>(camera_b)
        .unwrap()
        .viewport
        .clone();
    assert!(viewport_a.is_some());
    assert!(viewport_b.is_some());
    assert!(app.world().resource::<SplitScreenRuntime>().active);
}

#[test]
fn join_and_leave_recompute_the_snapshot_and_ui_targeting() {
    let mut app = integration_app(SplitScreenConfig {
        two_player: split_screen::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 40.0,
            merge_outer_distance: 80.0,
            ..default()
        },
        ..default()
    });
    spawn_slot_target(&mut app, 0, Vec3::ZERO);
    let camera_a = spawn_slot_camera(&mut app, 0, 0, 1.0);
    let ui_a = spawn_ui_root(&mut app, 0);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);
    assert_eq!(
        app.world().resource::<SplitScreenRuntime>().snapshots[0].mode,
        SplitScreenLayoutMode::Shared
    );

    let target_b = spawn_slot_target(&mut app, 1, Vec3::new(240.0, 180.0, 0.0));
    let camera_b = spawn_slot_camera(&mut app, 1, 1, 1.0);
    let ui_b = spawn_ui_root(&mut app, 1);

    app.world_mut().run_schedule(Tick);
    assert_eq!(
        app.world().resource::<SplitScreenRuntime>().snapshots[0].mode,
        SplitScreenLayoutMode::DynamicTwoPlayer
    );
    assert!(app.world().get::<UiTargetCamera>(ui_a).is_some());
    assert!(app.world().get::<UiTargetCamera>(ui_b).is_some());

    app.world_mut().entity_mut(target_b).despawn();
    app.world_mut().entity_mut(camera_b).despawn();
    app.world_mut().entity_mut(ui_b).despawn();

    app.world_mut().run_schedule(Tick);

    let runtime = app.world().resource::<SplitScreenRuntime>();
    assert_eq!(runtime.snapshots[0].mode, SplitScreenLayoutMode::Shared);
    assert_eq!(app.world().get::<UiTargetCamera>(ui_a).unwrap().0, camera_a);
}

#[test]
fn deactivate_restores_managed_cameras_and_clears_runtime_state() {
    let mut app = integration_app(SplitScreenConfig {
        mode: split_screen::SplitScreenMode::FixedOnly,
        ..default()
    });
    spawn_slot_target(&mut app, 0, Vec3::new(-220.0, 0.0, 0.0));
    spawn_slot_target(&mut app, 1, Vec3::new(220.0, 0.0, 0.0));
    let camera_a = spawn_slot_camera(&mut app, 0, 0, 1.0);
    let camera_b = spawn_slot_camera(&mut app, 1, 1, 1.0);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);
    assert!(app.world().resource::<SplitScreenRuntime>().active);

    app.world_mut().run_schedule(Deactivate);

    assert!(!app.world().resource::<SplitScreenRuntime>().active);
    assert!(
        app.world()
            .resource::<SplitScreenRuntime>()
            .snapshots
            .is_empty()
    );
    assert!(
        app.world()
            .get::<Camera>(camera_a)
            .unwrap()
            .viewport
            .is_none()
    );
    assert!(
        app.world()
            .get::<Camera>(camera_b)
            .unwrap()
            .viewport
            .is_none()
    );
    assert!(app.world().get::<Camera>(camera_a).unwrap().is_active);
    assert!(app.world().get::<Camera>(camera_b).unwrap().is_active);
}

#[test]
fn resize_message_recomputes_the_layout_integration_path() {
    let mut app = integration_app(SplitScreenConfig {
        resize_debounce_frames: 0,
        mode: split_screen::SplitScreenMode::FixedOnly,
        ..default()
    });
    let window_entity = {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
        query.single(world).expect("primary window")
    };
    spawn_slot_target(&mut app, 0, Vec3::new(-220.0, 0.0, 0.0));
    spawn_slot_target(&mut app, 1, Vec3::new(220.0, 0.0, 0.0));
    let camera = spawn_slot_camera(&mut app, 0, 0, 1.0);
    spawn_slot_camera(&mut app, 1, 1, 1.0);

    app.world_mut().run_schedule(Activate);
    app.world_mut().run_schedule(Tick);
    let before = app
        .world()
        .get::<Camera>(camera)
        .unwrap()
        .viewport
        .clone()
        .unwrap()
        .physical_size;

    {
        let world = app.world_mut();
        let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let mut window = windows.single_mut(world).expect("primary window");
        window.resolution.set_physical_resolution(960, 540);
        world.write_message(WindowResized {
            window: window_entity,
            width: 960.0,
            height: 540.0,
        });
    }

    app.world_mut().run_schedule(Tick);

    let after = app
        .world()
        .get::<Camera>(camera)
        .unwrap()
        .viewport
        .clone()
        .unwrap()
        .physical_size;
    assert_ne!(before, after);
    assert_eq!(
        app.world()
            .resource::<SplitScreenRuntime>()
            .last_resize_window,
        Some(window_entity)
    );
}
