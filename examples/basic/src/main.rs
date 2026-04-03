use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenConfig, SplitScreenMode, SplitScreenPlugin,
    SplitScreenTwoPlayerConfig, SplitScreenTwoPlayerLayout,
};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen basic")),
        SplitScreenPlugin::default(),
    ));
    common::add_debug_pane(&mut app);
    app.insert_resource(SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        two_player: SplitScreenTwoPlayerConfig {
            fixed_layout: SplitScreenTwoPlayerLayout::Vertical,
            ..default()
        },
        ..default()
    });
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
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

    common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(0),
        Vec2::new(-280.0, -40.0),
        Vec2::new(64.0, 64.0),
    );
    common::spawn_demo_target(
        &mut commands,
        LocalPlayerSlot(1),
        Vec2::new(280.0, 80.0),
        Vec2::new(64.0, 64.0),
    );
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(0), 0, 1.0);
    common::spawn_managed_camera(&mut commands, LocalPlayerSlot(1), 1, 1.0);

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_slot_hud(
        &mut commands,
        overlay_camera,
        LocalPlayerSlot(0),
        "Player 1",
    );
    common::spawn_slot_hud(
        &mut commands,
        overlay_camera,
        LocalPlayerSlot(1),
        "Player 2",
    );
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Fixed two-player split\nThe crate only owns viewports and UI targeting.",
    );
}
