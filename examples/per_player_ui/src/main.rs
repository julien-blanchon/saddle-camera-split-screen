use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{LocalPlayerSlot, SplitScreenConfig, SplitScreenPlugin};

#[derive(Component)]
struct UiFocusMotion {
    radius: f32,
    speed: f32,
    phase: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen per_player_ui")),
        SplitScreenPlugin::default(),
    ));
    app.insert_resource(SplitScreenConfig {
        two_player: split_screen::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 110.0,
            merge_outer_distance: 180.0,
            ..default()
        },
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_targets,
            common::follow_cameras,
            common::update_hud_text,
            common::update_divider_overlay,
            common::update_debug_overlay,
        ),
    );
    app.run();
}

fn setup(mut commands: Commands) {
    common::spawn_arena(&mut commands);

    let first = common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(0),
        Vec2::new(-120.0, -20.0),
        Vec2::new(64.0, 64.0),
    );
    commands.entity(first).insert(UiFocusMotion {
        radius: 90.0,
        speed: 0.48,
        phase: 0.0,
    });

    let second = common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(1),
        Vec2::new(120.0, 20.0),
        Vec2::new(64.0, 64.0),
    );
    commands.entity(second).insert(UiFocusMotion {
        radius: 90.0,
        speed: 0.48,
        phase: std::f32::consts::PI,
    });

    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(0), 0, 1.0);
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(1), 1, 1.0);

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(0), "HUD A");
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(1), "HUD B");
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Each HUD root is a normal Bevy UI tree.\nThe crate only retargets UiTargetCamera.",
    );
}

fn animate_targets(time: Res<Time>, mut targets: Query<(&UiFocusMotion, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (motion, mut transform) in &mut targets {
        let angle = t * motion.speed + motion.phase;
        transform.translation.x = motion.radius * angle.cos();
        transform.translation.y = motion.radius * 0.55 * angle.sin();
    }
}
