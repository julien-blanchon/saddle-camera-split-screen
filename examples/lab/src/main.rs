use split_screen_example_common as common;
#[cfg(feature = "e2e")]
mod e2e;

#[cfg(feature = "brp")]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::{input::ButtonInput, prelude::*};
#[cfg(feature = "brp")]
use bevy_brp_extras::BrpExtrasPlugin;
use split_screen::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenConfig, SplitScreenMode, SplitScreenPlugin,
    SplitScreenTarget, SplitScreenUiRoot, SplitScreenView,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
enum LabPresentation {
    Merge,
    SlantedSplit,
    WeightedSplit,
    FourPlayer,
    PerPlayerUi,
}

#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
struct LabPresentationState {
    presentation: LabPresentation,
}

impl Default for LabPresentationState {
    fn default() -> Self {
        Self {
            presentation: LabPresentation::Merge,
        }
    }
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
struct LabEntities {
    targets: [Entity; 4],
    cameras: [Entity; 4],
    hud_roots: [Entity; 4],
    overlay_camera: Entity,
}

#[derive(Component)]
struct LabStatusText;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(common::demo_window_plugin("split_screen_lab")),
        SplitScreenPlugin::default(),
    ));
    common::add_debug_pane(&mut app);
    #[cfg(feature = "brp")]
    app.add_plugins((
        RemotePlugin::default(),
        BrpExtrasPlugin::with_http_plugin(RemoteHttpPlugin::default()),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::SplitScreenLabE2EPlugin);

    app.register_type::<LabPresentationState>()
        .register_type::<LabEntities>()
        .insert_resource(SplitScreenConfig {
            two_player: split_screen::SplitScreenTwoPlayerConfig {
                merge_inner_distance: 90.0,
                merge_outer_distance: 170.0,
                ..default()
            },
            ..default()
        })
        .init_resource::<LabPresentationState>()
        .add_systems(Startup, (setup, initialize_presentation).chain())
        .add_systems(
            Update,
            (
                handle_hotkeys,
                common::follow_cameras,
                common::update_hud_text,
                common::update_divider_overlay,
                common::update_debug_overlay,
                update_status_text,
            ),
        );
    app.run();
}

fn setup(mut commands: Commands) {
    common::spawn_arena(&mut commands);

    let targets = [
        common::spawn_demo_target(
            &mut commands,
            LocalPlayerSlot(0),
            Vec2::new(-40.0, -20.0),
            Vec2::new(72.0, 72.0),
        ),
        common::spawn_demo_target(
            &mut commands,
            LocalPlayerSlot(1),
            Vec2::new(40.0, 20.0),
            Vec2::new(72.0, 72.0),
        ),
        common::spawn_demo_target(
            &mut commands,
            LocalPlayerSlot(2),
            Vec2::new(-320.0, -220.0),
            Vec2::new(60.0, 60.0),
        ),
        common::spawn_demo_target(
            &mut commands,
            LocalPlayerSlot(3),
            Vec2::new(320.0, -220.0),
            Vec2::new(60.0, 60.0),
        ),
    ];

    let cameras = [
        common::spawn_managed_camera(&mut commands, LocalPlayerSlot(0), 0, 1.0),
        common::spawn_managed_camera(&mut commands, LocalPlayerSlot(1), 1, 1.0),
        common::spawn_managed_camera(&mut commands, LocalPlayerSlot(2), 2, 1.0),
        common::spawn_managed_camera(&mut commands, LocalPlayerSlot(3), 3, 1.0),
    ];

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    let hud_roots = [
        common::spawn_slot_hud(
            &mut commands,
            overlay_camera,
            LocalPlayerSlot(0),
            "Player 1",
        ),
        common::spawn_slot_hud(
            &mut commands,
            overlay_camera,
            LocalPlayerSlot(1),
            "Player 2",
        ),
        common::spawn_slot_hud(
            &mut commands,
            overlay_camera,
            LocalPlayerSlot(2),
            "Player 3",
        ),
        common::spawn_slot_hud(
            &mut commands,
            overlay_camera,
            LocalPlayerSlot(3),
            "Player 4",
        ),
    ];

    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Split-screen lab\nBRP and crate-local E2E live here.",
    );
    commands.spawn((
        Name::new("Lab Status"),
        LabStatusText,
        bevy::ui::UiTargetCamera(overlay_camera),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(18.0),
            bottom: Val::Px(18.0),
            width: Val::Px(360.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.78)),
        Text::default(),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    commands.insert_resource(LabEntities {
        targets,
        cameras,
        hud_roots,
        overlay_camera,
    });
}

fn initialize_presentation(
    entities: Res<LabEntities>,
    presentation_state: Res<LabPresentationState>,
    mut commands: Commands,
) {
    apply_presentation(&mut commands, &entities, presentation_state.presentation);
}

fn handle_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut presentation_state: ResMut<LabPresentationState>,
    mut config: ResMut<SplitScreenConfig>,
    entities: Res<LabEntities>,
    mut commands: Commands,
) {
    let mut next_presentation = None;
    if keys.just_pressed(KeyCode::Digit1) {
        next_presentation = Some(LabPresentation::Merge);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        next_presentation = Some(LabPresentation::SlantedSplit);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        next_presentation = Some(LabPresentation::WeightedSplit);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        next_presentation = Some(LabPresentation::FourPlayer);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        next_presentation = Some(LabPresentation::PerPlayerUi);
    }

    if keys.just_pressed(KeyCode::KeyA) {
        config.mode = SplitScreenMode::Auto;
    }
    if keys.just_pressed(KeyCode::KeyD) {
        config.mode = SplitScreenMode::DynamicOnly;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        config.mode = SplitScreenMode::FixedOnly;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        config.mode = SplitScreenMode::SharedOnly;
    }

    if let Some(next) = next_presentation {
        set_presentation(&mut commands, &entities, presentation_state.as_mut(), next);
    }
}

fn update_status_text(
    presentation: Res<LabPresentationState>,
    config: Res<SplitScreenConfig>,
    entities: Res<LabEntities>,
    mut texts: Query<&mut Text, With<LabStatusText>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };

    text.0 = format!(
        "scene: {:?}\nmode override: {:?}\nslots: [1,2,3,4] entities ready\nkeys: 1 merge 2 slanted 3 weighted 4 four 5 ui | A auto D dynamic F fixed S shared\noverlay camera: {:?}",
        presentation.presentation, config.mode, entities.overlay_camera,
    );
}

pub(crate) fn set_presentation(
    commands: &mut Commands,
    entities: &LabEntities,
    presentation_state: &mut LabPresentationState,
    next: LabPresentation,
) {
    presentation_state.presentation = next;
    apply_presentation(commands, entities, next);
}

pub(crate) fn apply_presentation(
    commands: &mut Commands,
    entities: &LabEntities,
    next: LabPresentation,
) {
    let definitions = match next {
        LabPresentation::Merge => [
            Some((Vec2::new(-40.0, -20.0), 1.0)),
            Some((Vec2::new(40.0, 20.0), 1.0)),
            None,
            None,
        ],
        LabPresentation::SlantedSplit => [
            Some((Vec2::new(-260.0, -180.0), 1.0)),
            Some((Vec2::new(260.0, 180.0), 1.0)),
            None,
            None,
        ],
        LabPresentation::WeightedSplit => [
            Some((Vec2::new(-320.0, -140.0), 1.7)),
            Some((Vec2::new(240.0, 140.0), 0.7)),
            None,
            None,
        ],
        LabPresentation::FourPlayer => [
            Some((Vec2::new(-320.0, 220.0), 1.6)),
            Some((Vec2::new(320.0, 220.0), 1.0)),
            Some((Vec2::new(-320.0, -220.0), 1.0)),
            Some((Vec2::new(320.0, -220.0), 1.0)),
        ],
        LabPresentation::PerPlayerUi => [
            Some((Vec2::new(-220.0, 0.0), 1.0)),
            Some((Vec2::new(220.0, 0.0), 1.0)),
            None,
            None,
        ],
    };

    for (index, definition) in definitions.into_iter().enumerate() {
        let slot = LocalPlayerSlot(index as u8);
        let target = entities.targets[index];
        let camera = entities.cameras[index];
        let hud_root = entities.hud_roots[index];

        match definition {
            Some((position, area_weight)) => {
                commands.entity(target).insert((
                    SplitScreenTarget::default(),
                    Transform::from_xyz(position.x, position.y, 5.0),
                    Visibility::Visible,
                ));
                commands.entity(camera).insert((
                    Camera {
                        order: index as isize,
                        is_active: true,
                        viewport: None,
                        ..default()
                    },
                    SplitScreenCamera::default(),
                    SplitScreenView { area_weight },
                    Transform::from_xyz(position.x, position.y, 999.0),
                    Visibility::Visible,
                ));
                commands
                    .entity(hud_root)
                    .insert((SplitScreenUiRoot, Visibility::Visible, slot));
            }
            None => {
                commands.entity(target).remove::<SplitScreenTarget>();
                commands.entity(target).insert(Visibility::Hidden);

                commands
                    .entity(camera)
                    .remove::<SplitScreenCamera>()
                    .remove::<SplitScreenView>()
                    .insert((
                        Camera {
                            order: index as isize,
                            viewport: None,
                            is_active: false,
                            ..default()
                        },
                        Visibility::Hidden,
                    ));

                commands
                    .entity(hud_root)
                    .remove::<SplitScreenUiRoot>()
                    .remove::<bevy::ui::UiTargetCamera>()
                    .insert(Visibility::Hidden);
            }
        }
    }
}
