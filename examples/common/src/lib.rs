use std::fmt::Write as _;

use bevy::{camera::ClearColorConfig, prelude::*, ui::UiTargetCamera};
use saddle_pane::prelude::*;
use split_screen::{
    LocalPlayerSlot, SplitScreenCamera, SplitScreenDividerSnapshot, SplitScreenLayoutMode,
    SplitScreenLayoutSnapshot, SplitScreenRuntime, SplitScreenTarget, SplitScreenUiRoot,
    SplitScreenView,
};

pub const SLOT_COLORS: [Color; 4] = [
    Color::srgb(0.91, 0.36, 0.28),
    Color::srgb(0.24, 0.63, 0.94),
    Color::srgb(0.95, 0.78, 0.26),
    Color::srgb(0.34, 0.84, 0.52),
];

const CAMERA_Z: f32 = 999.0;
const HUD_POSITIONS: [(f32, f32); 4] = [(18.0, 18.0), (980.0, 18.0), (18.0, 566.0), (980.0, 566.0)];

#[derive(Component, Clone, Copy)]
pub struct SlotHudText(pub LocalPlayerSlot);

#[derive(Component)]
pub struct DividerOverlay;

#[derive(Component)]
pub struct DividerOverlayLabel;

#[derive(Component)]
pub struct DebugOverlayText;

#[derive(Component)]
pub struct DemoActor;

#[derive(Resource, Pane)]
#[pane(title = "Split Screen", position = "top-right")]
pub struct SplitScreenPane {
    #[pane(tab = "Layout", slider, min = 40.0, max = 220.0, step = 5.0)]
    pub merge_inner_distance: f32,
    #[pane(tab = "Layout", slider, min = 60.0, max = 320.0, step = 5.0)]
    pub merge_outer_distance: f32,
    #[pane(tab = "Layout", slider, min = 120.0, max = 720.0, step = 10.0)]
    pub minimum_viewport_width: f32,
    #[pane(tab = "Divider", slider, min = 0.0, max = 24.0, step = 0.5)]
    pub divider_width: f32,
    #[pane(tab = "Divider", slider, min = 0.0, max = 18.0, step = 0.5)]
    pub divider_feather: f32,
    #[pane(tab = "Runtime", monitor)]
    pub mode: String,
    #[pane(tab = "Runtime", monitor)]
    pub player_count: usize,
}

impl Default for SplitScreenPane {
    fn default() -> Self {
        let config = split_screen::SplitScreenConfig::default();
        Self {
            merge_inner_distance: config.two_player.merge_inner_distance,
            merge_outer_distance: config.two_player.merge_outer_distance,
            minimum_viewport_width: config.minimum_viewport_size.x as f32,
            divider_width: config.divider.width,
            divider_feather: config.divider.feather,
            mode: "Shared".into(),
            player_count: 0,
        }
    }
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn add_debug_pane(app: &mut App) {
    app.add_plugins(pane_plugins())
        .register_pane::<SplitScreenPane>()
        .add_systems(Update, sync_split_screen_pane);
}

pub fn demo_window_plugin(title: &str) -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1280, 720).into(),
            resizable: true,
            ..default()
        }),
        ..default()
    }
}

pub fn spawn_arena(commands: &mut Commands) {
    commands.spawn((
        Name::new("Demo Arena"),
        Sprite {
            color: Color::srgb(0.08, 0.09, 0.12),
            custom_size: Some(Vec2::new(2200.0, 1400.0)),
            ..default()
        },
    ));

    for index in -10..=10 {
        let x = index as f32 * 120.0;
        commands.spawn((
            Name::new(format!("Arena Grid Vertical {index}")),
            Sprite {
                color: Color::srgba(0.20, 0.23, 0.29, 0.55),
                custom_size: Some(Vec2::new(4.0, 1400.0)),
                ..default()
            },
            Transform::from_xyz(x, 0.0, -1.0),
        ));
    }

    for index in -6..=6 {
        let y = index as f32 * 120.0;
        commands.spawn((
            Name::new(format!("Arena Grid Horizontal {index}")),
            Sprite {
                color: Color::srgba(0.20, 0.23, 0.29, 0.55),
                custom_size: Some(Vec2::new(2200.0, 4.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, -1.0),
        ));
    }

    commands.spawn((
        Name::new("Arena Center Marker"),
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.18),
            custom_size: Some(Vec2::new(48.0, 48.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
}

pub fn spawn_demo_target(
    commands: &mut Commands,
    slot: LocalPlayerSlot,
    position: Vec2,
    size: Vec2,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("Demo Target {}", slot.0 + 1)),
            DemoActor,
            slot,
            SplitScreenTarget::default(),
            Sprite {
                color: slot_color(slot),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, 5.0),
        ))
        .id()
}

pub fn spawn_managed_camera(
    commands: &mut Commands,
    slot: LocalPlayerSlot,
    order: isize,
    area_weight: f32,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("Split Camera {}", slot.0 + 1)),
            slot,
            Camera2d,
            Camera { order, ..default() },
            SplitScreenCamera::default(),
            SplitScreenView { area_weight },
            Transform::from_xyz(0.0, 0.0, CAMERA_Z),
        ))
        .id()
}

pub fn spawn_overlay_camera(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("Overlay Camera"),
            Camera2d,
            Camera {
                order: 100,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id()
}

pub fn spawn_slot_hud(
    commands: &mut Commands,
    overlay_camera: Entity,
    slot: LocalPlayerSlot,
    title: &str,
) -> Entity {
    let (left, top) = HUD_POSITIONS[slot.index().min(HUD_POSITIONS.len() - 1)];

    commands
        .spawn((
            Name::new(format!("HUD Root {}", slot.0 + 1)),
            slot,
            SplitScreenUiRoot,
            UiTargetCamera(overlay_camera),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new(format!("HUD Panel {}", slot.0 + 1)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        top: Val::Px(top),
                        width: Val::Px(280.0),
                        min_height: Val::Px(118.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(slot_color(slot).with_alpha(0.18)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Name::new(format!("HUD Label {}", slot.0 + 1)),
                        Text::new(title),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(slot_color(slot)),
                    ));
                    panel.spawn((
                        Name::new(format!("HUD Text {}", slot.0 + 1)),
                        SlotHudText(slot),
                        Text::default(),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        })
        .id()
}

pub fn spawn_divider_overlay(commands: &mut Commands, overlay_camera: Entity) {
    commands
        .spawn((
            Name::new("Divider Overlay Root"),
            UiTargetCamera(overlay_camera),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Transform::default(),
            Visibility::Visible,
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Divider Overlay"),
                DividerOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(1.0),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                Visibility::Hidden,
                Transform::default(),
            ));
            parent.spawn((
                Name::new("Divider Overlay Label"),
                DividerOverlayLabel,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(18.0),
                    top: Val::Px(18.0),
                    width: Val::Px(320.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.75)),
                Text::default(),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn spawn_debug_overlay(commands: &mut Commands, overlay_camera: Entity, title: &str) {
    commands.spawn((
        Name::new("Debug Overlay"),
        DebugOverlayText,
        UiTargetCamera(overlay_camera),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            bottom: Val::Px(18.0),
            width: Val::Px(430.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.80)),
        Text::new(title),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

pub fn follow_cameras(
    targets: Query<(&LocalPlayerSlot, &GlobalTransform), With<SplitScreenTarget>>,
    mut cameras: Query<(&LocalPlayerSlot, &mut Transform), With<SplitScreenCamera>>,
) {
    for (slot, mut transform) in &mut cameras {
        if let Some((_, target_transform)) =
            targets.iter().find(|(target_slot, _)| *target_slot == slot)
        {
            transform.translation.x = target_transform.translation().x;
            transform.translation.y = target_transform.translation().y;
            transform.translation.z = CAMERA_Z;
        }
    }
}

pub fn update_hud_text(
    runtime: Res<SplitScreenRuntime>,
    ui_roots: Query<(&LocalPlayerSlot, Option<&UiTargetCamera>), With<SplitScreenUiRoot>>,
    mut texts: Query<(&SlotHudText, &mut Text)>,
) {
    let snapshot = primary_snapshot(&runtime);

    for (slot_text, mut text) in &mut texts {
        let assigned_camera = ui_roots
            .iter()
            .find(|(slot, _)| *slot == &slot_text.0)
            .and_then(|(_, target)| target.map(|target| target.0));
        let view = snapshot.and_then(|snapshot| view_for_slot(snapshot, slot_text.0));
        let mode = snapshot
            .map(|snapshot| snapshot.mode)
            .unwrap_or(SplitScreenLayoutMode::Shared);

        text.0 = format!(
            "slot: {}\nmode: {:?}\nactive: {}\nrect: {}\nui camera: {}",
            slot_text.0 .0 + 1,
            mode,
            view.is_some_and(|view| view.active),
            view.map(rect_summary).unwrap_or_else(|| "n/a".into()),
            assigned_camera
                .map(|entity| format!("{entity:?}"))
                .unwrap_or_else(|| "none".into()),
        );
    }
}

fn sync_split_screen_pane(
    mut pane: ResMut<SplitScreenPane>,
    runtime: Res<SplitScreenRuntime>,
    mut config: ResMut<split_screen::SplitScreenConfig>,
) {
    let pane_added = pane.is_added();

    if pane_added {
        let pane = pane.bypass_change_detection();
        pane.merge_inner_distance = config.two_player.merge_inner_distance;
        pane.merge_outer_distance = config.two_player.merge_outer_distance;
        pane.minimum_viewport_width = config.minimum_viewport_size.x as f32;
        pane.divider_width = config.divider.width;
        pane.divider_feather = config.divider.feather;
    }

    if pane.is_changed() && !pane_added {
        config.two_player.merge_inner_distance = pane.merge_inner_distance;
        config.two_player.merge_outer_distance = pane
            .merge_outer_distance
            .max(config.two_player.merge_inner_distance);
        config.minimum_viewport_size.x = pane.minimum_viewport_width.max(1.0) as u32;
        config.divider.width = pane.divider_width;
        config.divider.feather = pane.divider_feather;
    }

    let pane = pane.bypass_change_detection();
    if let Some(snapshot) = primary_snapshot(&runtime) {
        pane.mode = format!("{:?}", snapshot.mode);
        pane.player_count = snapshot.views.iter().filter(|view| view.active).count();
    } else {
        pane.mode = "Shared".into();
        pane.player_count = 0;
    }
}

pub fn update_divider_overlay(
    runtime: Res<SplitScreenRuntime>,
    mut divider_query: Query<
        (
            &mut Node,
            &mut BackgroundColor,
            &mut Transform,
            &mut Visibility,
        ),
        With<DividerOverlay>,
    >,
    mut label_query: Query<&mut Text, (With<DividerOverlayLabel>, Without<DividerOverlay>)>,
) {
    let snapshot = primary_snapshot(&runtime);
    let divider = snapshot.and_then(|snapshot| snapshot.divider.as_ref());

    if let Ok((mut node, mut color, mut transform, mut visibility)) = divider_query.single_mut() {
        if let Some(divider) = divider {
            apply_divider_style(&mut node, &mut color, &mut transform, divider);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    if let Ok(mut text) = label_query.single_mut() {
        text.0 = if let Some(snapshot) = snapshot {
            divider_summary(snapshot)
        } else {
            "divider: none".into()
        };
    }
}

pub fn update_debug_overlay(
    runtime: Res<SplitScreenRuntime>,
    mut texts: Query<&mut Text, With<DebugOverlayText>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };

    let mut output = String::new();
    if let Some(snapshot) = primary_snapshot(&runtime) {
        let _ = writeln!(
            output,
            "mode: {:?}\nmerged owner: {:?}\nalpha: {:.2}\nviews:",
            snapshot.mode,
            snapshot.merged_owner.map(|slot| slot.0 + 1),
            snapshot.transition_alpha,
        );
        for view in &snapshot.views {
            let _ = writeln!(
                output,
                "  p{} active={} {}",
                view.slot.0 + 1,
                view.active,
                rect_summary(view),
            );
        }
    } else {
        output.push_str("mode: no snapshot\n");
    }

    text.0 = output;
}

pub fn slot_color(slot: LocalPlayerSlot) -> Color {
    SLOT_COLORS[slot.index().min(SLOT_COLORS.len() - 1)]
}

pub fn primary_snapshot(runtime: &SplitScreenRuntime) -> Option<&SplitScreenLayoutSnapshot> {
    runtime.snapshots.first()
}

pub fn view_for_slot(
    snapshot: &SplitScreenLayoutSnapshot,
    slot: LocalPlayerSlot,
) -> Option<&split_screen::SplitScreenViewSnapshot> {
    snapshot.views.iter().find(|view| view.slot == slot)
}

fn rect_summary(view: &split_screen::SplitScreenViewSnapshot) -> String {
    format!(
        "{}x{} @ {},{}",
        view.physical.size.x,
        view.physical.size.y,
        view.physical.position.x,
        view.physical.position.y,
    )
}

fn divider_summary(snapshot: &SplitScreenLayoutSnapshot) -> String {
    let Some(divider) = snapshot.divider.as_ref() else {
        return "divider: none".into();
    };

    format!(
        "divider: ({:.0}, {:.0}) -> ({:.0}, {:.0})\nalpha: {:.2}",
        divider.physical_start.x,
        divider.physical_start.y,
        divider.physical_end.x,
        divider.physical_end.y,
        snapshot.transition_alpha,
    )
}

fn apply_divider_style(
    node: &mut Node,
    color: &mut BackgroundColor,
    transform: &mut Transform,
    divider: &SplitScreenDividerSnapshot,
) {
    let delta = divider.physical_end - divider.physical_start;
    let length = delta.length().max(1.0);
    let midpoint = (divider.physical_start + divider.physical_end) * 0.5;

    node.left = Val::Px(midpoint.x - length * 0.5);
    node.top = Val::Px(midpoint.y - divider.thickness.max(1.0) * 0.5);
    node.width = Val::Px(length);
    node.height = Val::Px(divider.thickness.max(1.0));
    *color = BackgroundColor(divider.color);
    *transform = Transform::from_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x)));
}
