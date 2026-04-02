use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{LocalPlayerSlot, SplitScreenConfig, SplitScreenPlugin};

#[derive(Component)]
struct DynamicPair;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin(
            "split_screen dynamic_two_player",
        )),
        SplitScreenPlugin::default(),
    ));
    app.insert_resource(SplitScreenConfig {
        two_player: split_screen::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 120.0,
            merge_outer_distance: 210.0,
            ..default()
        },
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_pair,
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
        Vec2::new(-90.0, -50.0),
        Vec2::new(72.0, 72.0),
    );
    commands.entity(first).insert(DynamicPair);
    let second = common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(1),
        Vec2::new(90.0, 50.0),
        Vec2::new(72.0, 72.0),
    );
    commands.entity(second).insert(DynamicPair);
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(0), 0, 1.0);
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(1), 1, 1.0);

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(0), "Scout");
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(1), "Anchor");
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Dynamic merge and split\nTargets drift between shared and diagonal separation.",
    );
}

fn animate_pair(
    time: Res<Time>,
    mut targets: Query<(&LocalPlayerSlot, &mut Transform), With<DynamicPair>>,
) {
    let t = time.elapsed_secs();
    let separation = 60.0 + 220.0 * ((t * 0.42).sin() * 0.5 + 0.5);
    let slant = 0.55 + 0.25 * (t * 0.27).cos();

    for (slot, mut transform) in &mut targets {
        let direction = if slot.0 == 0 { -1.0 } else { 1.0 };
        transform.translation.x = direction * separation;
        transform.translation.y = direction * separation * slant;
    }
}
