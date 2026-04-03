use bevy::{prelude::*, window::WindowPlugin};
use saddle_camera_third_person_camera::{
    ShoulderSide, ThirdPersonCamera, ThirdPersonCameraMode, ThirdPersonCameraPlugin,
    ThirdPersonCameraRuntime, ThirdPersonCameraSettings, ThirdPersonCameraTarget,
};
use saddle_pane::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenConfig, SplitScreenPlugin,
    SplitScreenProjectionPlane, SplitScreenSystems, SplitScreenTarget, SplitScreenView,
};
use split_screen_example_common as common;

#[derive(Component, Clone, Copy)]
struct CoopHeroPath {
    midpoint: Vec3,
    lane_offset: f32,
    max_separation: f32,
    travel_depth: f32,
    speed: f32,
    phase: f32,
}

#[derive(Component)]
struct CoopHero;

#[derive(Component)]
struct SceneStatusText;

#[derive(Resource, Pane)]
#[pane(title = "Third Person Cameras", position = "bottom-right")]
struct ThirdPersonCoopPane {
    #[pane(tab = "Framing", slider, min = 3.0, max = 8.0, step = 0.1)]
    distance: f32,
    #[pane(tab = "Framing", slider, min = 0.0, max = 1.5, step = 0.05)]
    shoulder_offset: f32,
    #[pane(tab = "Framing", slider, min = 0.6, max = 2.2, step = 0.05)]
    framing_height: f32,
    #[pane(tab = "Runtime", monitor)]
    corrected_distance: f32,
    #[pane(tab = "Runtime", monitor)]
    obstruction_active: bool,
}

impl Default for ThirdPersonCoopPane {
    fn default() -> Self {
        let settings = ThirdPersonCameraSettings::default();
        Self {
            distance: settings.zoom.default_distance,
            shoulder_offset: settings.framing.shoulder_offset,
            framing_height: settings.framing.shoulder_height,
            corrected_distance: settings.zoom.default_distance,
            obstruction_active: false,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "saddle-camera-split-screen third_person_coop".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }),
        SplitScreenPlugin::default(),
        ThirdPersonCameraPlugin::default(),
    ));
    common::add_debug_pane(&mut app);
    app.insert_resource(SplitScreenConfig {
        default_projection: SplitScreenProjectionPlane::Xz,
        two_player: split_screen::SplitScreenTwoPlayerConfig {
            merge_inner_distance: 7.0,
            merge_outer_distance: 15.0,
            axis_hysteresis: 0.12,
            ..default()
        },
        ..default()
    })
    .init_resource::<ThirdPersonCoopPane>()
    .register_pane::<ThirdPersonCoopPane>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            animate_heroes.before(SplitScreenSystems::CollectTargets),
            sync_camera_pane,
            common::update_hud_text,
            common::update_divider_overlay,
            common::update_debug_overlay,
            update_scene_status,
        ),
    );
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_world(&mut commands, &mut meshes, &mut materials);

    let overlay_camera = common::spawn_overlay_camera(&mut commands);
    common::spawn_divider_overlay(&mut commands, overlay_camera);
    common::spawn_debug_overlay(
        &mut commands,
        overlay_camera,
        "Third-person split-screen coop\nWeighted viewports widen Player 1 and collapse back to shared view when the heroes regroup.",
    );
    commands.spawn((
        Name::new("Scene Status"),
        SceneStatusText,
        bevy::ui::UiTargetCamera(overlay_camera),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(18.0),
            bottom: Val::Px(18.0),
            width: Val::Px(340.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.06, 0.10, 0.78)),
        Text::default(),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    let left_hero = spawn_hero(
        &mut commands,
        &mut meshes,
        &mut materials,
        LocalPlayerSlot(0),
        "Vanguard",
        Color::srgb(0.91, 0.36, 0.28),
        CoopHeroPath {
            midpoint: Vec3::new(0.0, 0.0, -18.0),
            lane_offset: -2.6,
            max_separation: 8.0,
            travel_depth: 9.0,
            speed: 0.45,
            phase: 0.0,
        },
    );
    let right_hero = spawn_hero(
        &mut commands,
        &mut meshes,
        &mut materials,
        LocalPlayerSlot(1),
        "Scout",
        Color::srgb(0.24, 0.63, 0.94),
        CoopHeroPath {
            midpoint: Vec3::new(0.0, 0.0, -18.0),
            lane_offset: 2.6,
            max_separation: 8.0,
            travel_depth: 9.0,
            speed: 0.45,
            phase: std::f32::consts::PI,
        },
    );

    spawn_camera(
        &mut commands,
        LocalPlayerSlot(0),
        left_hero,
        0,
        1.35,
        ShoulderSide::Right,
    );
    spawn_camera(
        &mut commands,
        LocalPlayerSlot(1),
        right_hero,
        1,
        0.85,
        ShoulderSide::Left,
    );

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
}

fn spawn_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 26_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 18.0, 12.0).looking_at(Vec3::new(0.0, 0.0, -18.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Fill Light"),
        PointLight {
            intensity: 100_000.0,
            range: 55.0,
            ..default()
        },
        Transform::from_xyz(-8.0, 7.0, -8.0),
    ));
    commands.spawn((
        Name::new("Arena Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.07, 0.09, 0.12),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    for (index, z) in [-8.0_f32, -18.0, -28.0].into_iter().enumerate() {
        commands.spawn((
            Name::new(format!("Cross Lane {}", index + 1)),
            Mesh3d(meshes.add(Cuboid::new(12.0, 0.12, 1.6))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.14, 0.17, 0.22),
                metallic: 0.04,
                perceptual_roughness: 0.88,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.06, z),
        ));
    }

    for (name, translation, size, color) in [
        (
            "Cover Wall Left",
            Vec3::new(-5.5, 1.4, -16.0),
            Vec3::new(0.6, 2.8, 6.0),
            Color::srgb(0.32, 0.20, 0.18),
        ),
        (
            "Cover Wall Right",
            Vec3::new(5.5, 1.4, -20.0),
            Vec3::new(0.6, 2.8, 6.0),
            Color::srgb(0.16, 0.22, 0.34),
        ),
        (
            "Center Gate",
            Vec3::new(0.0, 2.0, -24.0),
            Vec3::new(4.0, 4.0, 0.8),
            Color::srgb(0.86, 0.44, 0.20),
        ),
    ] {
        commands.spawn((
            Name::new(name.to_string()),
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.86,
                ..default()
            })),
            Transform::from_translation(translation),
            saddle_camera_third_person_camera::ThirdPersonCameraObstacle::default(),
        ));
    }
}

fn spawn_hero(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    slot: LocalPlayerSlot,
    name: &str,
    color: Color,
    path: CoopHeroPath,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            CoopHero,
            slot,
            path,
            SplitScreenTarget {
                projection: Some(SplitScreenProjectionPlane::Xz),
                anchor_offset: Vec3::Y * 1.2,
                ..default()
            },
            Mesh3d(meshes.add(Capsule3d::new(0.46, 1.3).mesh().rings(10).latitudes(14))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.05,
                perceptual_roughness: 0.24,
                ..default()
            })),
            Transform::from_xyz(path.lane_offset, 1.0, -8.0),
        ))
        .id()
}

fn spawn_camera(
    commands: &mut Commands,
    slot: LocalPlayerSlot,
    target: Entity,
    order: isize,
    area_weight: f32,
    shoulder_side: ShoulderSide,
) {
    let mut settings = ThirdPersonCameraSettings::default();
    settings.zoom.default_distance = 4.8;
    settings.zoom.min_distance = 2.4;
    settings.zoom.max_distance = 7.2;
    settings.framing.shoulder_height = 1.35;

    let camera =
        ThirdPersonCamera::looking_at(Vec3::new(0.0, 1.2, -8.0), Vec3::new(1.2, 2.6, -2.0))
            .with_mode(ThirdPersonCameraMode::Shoulder)
            .with_shoulder_side(shoulder_side);

    commands.spawn((
        Name::new(format!("Coop Camera {}", slot.0 + 1)),
        slot,
        Camera { order, ..default() },
        SplitScreenCamera::default(),
        SplitScreenView { area_weight },
        camera,
        settings,
        ThirdPersonCameraTarget {
            target,
            offset: Vec3::Y * 1.2,
            follow_rotation: false,
            enabled: true,
            ignore_children: true,
            ignored_entities: vec![target],
            recenter_on_target_change: false,
        },
        Transform::from_xyz(0.0, 2.8, 4.8),
    ));
}

fn animate_heroes(
    time: Res<Time>,
    mut heroes: Query<(&CoopHeroPath, &mut Transform), With<CoopHero>>,
) {
    let t = time.elapsed_secs();
    for (path, mut transform) in &mut heroes {
        let wave = (t * path.speed + path.phase).sin();
        let depth_wave = (t * path.speed * 0.5 + path.phase).cos();
        let separation = wave * path.max_separation * 0.5;
        let translation = path.midpoint
            + Vec3::new(
                path.lane_offset + separation,
                1.0,
                depth_wave * path.travel_depth,
            );
        let previous = transform.translation;
        transform.translation = translation;
        let velocity = transform.translation - previous;
        if velocity.length_squared() > 0.0001 {
            transform.look_to(velocity.normalize_or_zero(), Vec3::Y);
        }
    }
}

fn sync_camera_pane(
    mut pane: ResMut<ThirdPersonCoopPane>,
    mut cameras: Query<(
        &mut ThirdPersonCamera,
        &mut ThirdPersonCameraSettings,
        &ThirdPersonCameraRuntime,
    )>,
) {
    let mut first_runtime = None;

    for (mut camera, mut settings, runtime) in &mut cameras {
        if pane.is_changed() && !pane.is_added() {
            settings.zoom.default_distance = pane.distance;
            settings.framing.shoulder_offset = pane.shoulder_offset;
            settings.framing.shoulder_height = pane.framing_height;
            camera.target_distance = pane.distance;
        }

        if first_runtime.is_none() {
            first_runtime = Some(*runtime);
        }
    }

    if let Some(runtime) = first_runtime {
        pane.corrected_distance = runtime.corrected_distance;
        pane.obstruction_active = runtime.obstruction_active;
    }
}

fn update_scene_status(
    heroes: Query<(&LocalPlayerSlot, &GlobalTransform), With<CoopHero>>,
    mut texts: Query<&mut Text, With<SceneStatusText>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };

    let mut positions = [Vec3::ZERO; 2];
    for (slot, transform) in &heroes {
        if slot.index() < positions.len() {
            positions[slot.index()] = transform.translation();
        }
    }
    let separation = positions[0].xz().distance(positions[1].xz());

    text.0 = format!(
        "Third-person coop scene\nP1 width bias: 1.35x\nP2 width bias: 0.85x\nhero separation: {:.2}\nmerge when close, split when far",
        separation,
    );
}
