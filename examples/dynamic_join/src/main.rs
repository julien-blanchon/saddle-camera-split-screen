use split_screen_example_common as common;

use bevy::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenConfig, SplitScreenMode, SplitScreenPlugin,
    SplitScreenTarget, SplitScreenTransitionConfig, SplitScreenTransitionEasing, SplitScreenView,
};

const MAX_PLAYERS: u8 = 4;

const SPAWN_POSITIONS: [Vec2; 4] = [
    Vec2::new(-200.0, 100.0),
    Vec2::new(200.0, 100.0),
    Vec2::new(-200.0, -100.0),
    Vec2::new(200.0, -100.0),
];

const PLAYER_NAMES: [&str; 4] = ["Player 1", "Player 2", "Player 3", "Player 4"];

#[derive(Component)]
struct PlayerMarker(u8);

#[derive(Component)]
struct PlayerCameraMarker(u8);

#[derive(Resource)]
struct ActivePlayers {
    active: [bool; 4],
}

impl Default for ActivePlayers {
    fn default() -> Self {
        Self {
            active: [true, true, false, false],
        }
    }
}

#[derive(Component)]
struct InstructionsText;

#[derive(Component)]
struct DriftMotion {
    base: Vec2,
    phase: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen dynamic_join")),
        SplitScreenPlugin::default(),
    ));
    common::add_debug_pane(&mut app);
    app.insert_resource(SplitScreenConfig {
        mode: SplitScreenMode::FixedOnly,
        min_players: 1,
        transition: SplitScreenTransitionConfig {
            enabled: true,
            duration_seconds: 0.4,
            easing: SplitScreenTransitionEasing::EaseOutCubic,
        },
        ..default()
    });
    app.init_resource::<ActivePlayers>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            handle_join_leave_input,
            animate_targets,
            common::follow_cameras,
            common::update_hud_text,
            common::update_divider_overlay,
            common::update_debug_overlay,
        ),
    );
    app.run();
}

fn setup(mut commands: Commands, players: Res<ActivePlayers>) {
    common::spawn_arena(&mut commands);

    for index in 0..MAX_PLAYERS {
        let slot = LocalPlayerSlot(index);
        let position = SPAWN_POSITIONS[index as usize];

        let target =
            common::spawn_demo_target(&mut commands, slot, position, Vec2::new(58.0, 58.0));
        commands.entity(target).insert((
            PlayerMarker(index),
            DriftMotion {
                base: position,
                phase: index as f32 * 1.2,
            },
        ));

        let camera = commands
            .spawn((
                Name::new(format!("Split Camera {}", index + 1)),
                slot,
                Camera2d,
                Camera {
                    order: index as isize,
                    is_active: players.active[index as usize],
                    ..default()
                },
                SplitScreenCamera::default(),
                SplitScreenView { area_weight: 1.0 },
                PlayerCameraMarker(index),
                Transform::from_xyz(0.0, 0.0, 999.0),
            ))
            .id();

        if !players.active[index as usize] {
            commands.entity(target).remove::<SplitScreenTarget>();
            commands.entity(camera).remove::<SplitScreenCamera>();
        }
    }

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    for index in 0..MAX_PLAYERS {
        let slot = LocalPlayerSlot(index);
        common::spawn_slot_hud(
            &mut commands,
            overlay_camera,
            slot,
            PLAYER_NAMES[index as usize],
        );
    }
    common::spawn_divider_overlay(&mut commands, overlay_camera);

    commands.spawn((
        Name::new("Instructions"),
        InstructionsText,
        UiTargetCamera(overlay_camera),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            bottom: Val::Px(18.0),
            width: Val::Px(520.0),
            padding: UiRect::all(Val::Px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.85)),
        Text::new(instructions_text(&players)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn handle_join_leave_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut players: ResMut<ActivePlayers>,
    mut targets: Query<(Entity, &PlayerMarker), Without<PlayerCameraMarker>>,
    mut cameras: Query<(Entity, &PlayerCameraMarker), Without<PlayerMarker>>,
    mut instructions: Query<&mut Text, With<InstructionsText>>,
    mut commands: Commands,
) {
    let toggle_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ];
    let mut changed = false;

    for (index, key) in toggle_keys.iter().enumerate() {
        if keys.just_pressed(*key) {
            let was_active = players.active[index];
            players.active[index] = !was_active;
            changed = true;

            if was_active {
                for (entity, marker) in targets.iter() {
                    if marker.0 == index as u8 {
                        commands.entity(entity).remove::<SplitScreenTarget>();
                    }
                }
                for (entity, marker) in cameras.iter() {
                    if marker.0 == index as u8 {
                        commands.entity(entity).remove::<SplitScreenCamera>();
                    }
                }
            } else {
                for (entity, marker) in targets.iter_mut() {
                    if marker.0 == index as u8 {
                        commands.entity(entity).insert(SplitScreenTarget::default());
                    }
                }
                for (entity, marker) in cameras.iter_mut() {
                    if marker.0 == index as u8 {
                        commands.entity(entity).insert(SplitScreenCamera::default());
                    }
                }
            }
        }
    }

    if changed {
        if let Ok(mut text) = instructions.single_mut() {
            text.0 = instructions_text(&players);
        }
    }
}

fn animate_targets(time: Res<Time>, mut targets: Query<(&DriftMotion, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (motion, mut transform) in &mut targets {
        transform.translation.x = motion.base.x + 60.0 * (t * 0.5 + motion.phase).sin();
        transform.translation.y = motion.base.y + 40.0 * (t * 0.35 + motion.phase).cos();
    }
}

fn instructions_text(players: &ActivePlayers) -> String {
    let mut text = String::from("DYNAMIC JOIN/LEAVE DEMO\n\n");
    text.push_str("Press 1-4 to toggle players on/off\n");
    text.push_str("Viewports animate smoothly when layout changes\n\n");
    for index in 0..MAX_PLAYERS {
        let status = if players.active[index as usize] {
            "ACTIVE"
        } else {
            "inactive"
        };
        let color_name = match index {
            0 => "Red",
            1 => "Blue",
            2 => "Yellow",
            3 => "Green",
            _ => "?",
        };
        text.push_str(&format!(
            "[{}] {} ({}): {}\n",
            index + 1,
            PLAYER_NAMES[index as usize],
            color_name,
            status,
        ));
    }
    text
}
