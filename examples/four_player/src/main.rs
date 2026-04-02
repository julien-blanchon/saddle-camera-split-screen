use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenConfig, SplitScreenFourPlayerConfig, SplitScreenFourPlayerLayout,
    SplitScreenMode, SplitScreenPlugin,
};

#[derive(Component)]
struct CornerMotion {
    base: Vec2,
    phase: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen four_player")),
        SplitScreenPlugin::default(),
    ));
    app.insert_resource(SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        four_player: SplitScreenFourPlayerConfig {
            layout: SplitScreenFourPlayerLayout::Grid,
            ..default()
        },
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_corners,
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

    let slots = [
        (LocalPlayerSlot(0), Vec2::new(-340.0, 220.0), 1.6),
        (LocalPlayerSlot(1), Vec2::new(340.0, 220.0), 1.0),
        (LocalPlayerSlot(2), Vec2::new(-340.0, -220.0), 1.0),
        (LocalPlayerSlot(3), Vec2::new(340.0, -220.0), 1.0),
    ];

    for (slot, position, area_weight) in slots {
        let target =
            common::spawn_demo_target(&mut commands, slot, position, Vec2::new(62.0, 62.0));
        commands.entity(target).insert(CornerMotion {
            base: position,
            phase: slot.0 as f32 * 0.6,
        });
        common::spawn_managed_camera(&mut commands, slot, slot.0 as isize, area_weight);
    }

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(0), "Leader");
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(1), "Flank");
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(2), "Scout");
    common::spawn_slot_hud(&mut commands, overlay_camera, LocalPlayerSlot(3), "Support");
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Four-player fixed layout\nSlot 1 carries extra weight to claim more area.",
    );
}

fn animate_corners(time: Res<Time>, mut targets: Query<(&CornerMotion, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (motion, mut transform) in &mut targets {
        transform.translation.x = motion.base.x + 34.0 * (t * 0.75 + motion.phase).sin();
        transform.translation.y = motion.base.y + 24.0 * (t * 0.55 + motion.phase).cos();
    }
}
