use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{LocalPlayerSlot, SplitScreenConfig, SplitScreenPlugin, SplitScreenView};

#[derive(Component)]
struct WeightedDuelist {
    center: Vec2,
    radius: f32,
    speed: f32,
    phase: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen weighted_dynamic")),
        SplitScreenPlugin::default(),
    ));
    common::add_debug_pane(&mut app);
    app.insert_resource(SplitScreenConfig {
        two_player: split_screen::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 120.0,
            merge_outer_distance: 220.0,
            fixed_layout: split_screen::SplitScreenTwoPlayerLayout::Vertical,
            ..default()
        },
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_duelists,
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

    let lead = common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(0),
        Vec2::new(-220.0, -80.0),
        Vec2::new(74.0, 74.0),
    );
    commands.entity(lead).insert(WeightedDuelist {
        center: Vec2::new(-180.0, -60.0),
        radius: 90.0,
        speed: 0.42,
        phase: 0.0,
    });

    let scout = common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(1),
        Vec2::new(220.0, 80.0),
        Vec2::new(64.0, 64.0),
    );
    commands.entity(scout).insert(WeightedDuelist {
        center: Vec2::new(180.0, 60.0),
        radius: 90.0,
        speed: 0.42,
        phase: std::f32::consts::PI,
    });

    let lead_camera = common::spawn_managed_camera(&mut commands, LocalPlayerSlot(0), 0, 3.0);
    commands
        .entity(lead_camera)
        .insert(SplitScreenView { area_weight: 3.0 });
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(1), 1, 1.0);

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_slot_hud(
        &mut commands,
        overlay_camera,
        LocalPlayerSlot(0),
        "Lead Camera",
    );
    common::spawn_slot_hud(
        &mut commands,
        overlay_camera,
        LocalPlayerSlot(1),
        "Scout Camera",
    );
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Weighted dynamic split\nSlot 1 carries triple the area weight, so the divider slides off center.",
    );
}

fn animate_duelists(time: Res<Time>, mut targets: Query<(&WeightedDuelist, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (motion, mut transform) in &mut targets {
        let angle = t * motion.speed + motion.phase;
        transform.translation.x = motion.center.x + motion.radius * angle.cos();
        transform.translation.y = motion.center.y + motion.radius * 0.55 * angle.sin();
    }
}
