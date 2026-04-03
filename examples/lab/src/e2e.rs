use bevy::{prelude::*, window::PrimaryWindow};
use saddle_bevy_e2e::{
    action::Action, actions::assertions, init_scenario, scenario::Scenario, E2EPlugin, E2ESet,
};
use split_screen::{LocalPlayerSlot, SplitScreenLayoutMode, SplitScreenRuntime, SplitScreenSystems};

use crate::{apply_presentation, LabEntities, LabPresentation, LabPresentationState};

pub struct SplitScreenLabE2EPlugin;

impl Plugin for SplitScreenLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(SplitScreenSystems::CollectTargets));

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                init_scenario(app, scenario);
            } else {
                error!(
                    "[split_screen_lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

fn list_scenarios() -> Vec<&'static str> {
    vec![
        "split_screen_smoke",
        "split_screen_two_player_merge",
        "split_screen_two_player_slanted_split",
        "split_screen_weighted_dynamic",
        "split_screen_resize",
        "split_screen_four_player",
        "split_screen_per_player_ui",
    ]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "split_screen_smoke" => Some(build_smoke()),
        "split_screen_two_player_merge" => Some(build_two_player_merge()),
        "split_screen_two_player_slanted_split" => Some(build_slanted_split()),
        "split_screen_weighted_dynamic" => Some(build_weighted_dynamic()),
        "split_screen_resize" => Some(build_resize()),
        "split_screen_four_player" => Some(build_four_player()),
        "split_screen_per_player_ui" => Some(build_per_player_ui()),
        _ => None,
    }
}

fn snapshot(world: &World) -> split_screen::SplitScreenLayoutSnapshot {
    world
        .resource::<SplitScreenRuntime>()
        .snapshots
        .first()
        .expect("split-screen snapshot should exist")
        .clone()
}

fn set_lab_presentation(world: &mut World, presentation: LabPresentation) {
    let entities = world.resource::<LabEntities>().clone();
    {
        let mut state = world.resource_mut::<LabPresentationState>();
        state.presentation = presentation;
    }
    let mut commands = world.commands();
    apply_presentation(&mut commands, &entities, presentation);
}

fn ui_target_for_slot(world: &World, slot: u8) -> Option<Entity> {
    let entities = world.resource::<LabEntities>();
    let entity = entities.hud_roots[usize::from(slot)];
    world
        .get::<bevy::ui::UiTargetCamera>(entity)
        .map(|target| target.0)
}

fn build_smoke() -> Scenario {
    Scenario::builder("split_screen_smoke")
        .description(
            "Boot the split-screen lab in its merged state, assert that a runtime snapshot exists, and capture a readable baseline frame.",
        )
        .then(Action::WaitFrames(30))
        .then(assertions::custom("runtime snapshot exists", |world| {
            !world.resource::<SplitScreenRuntime>().snapshots.is_empty()
        }))
        .then(assertions::custom("lab starts in merge presentation", |world| {
            world.resource::<LabPresentationState>().presentation == LabPresentation::Merge
        }))
        .then(assertions::log_summary("split_screen_smoke summary"))
        .then(Action::Screenshot("split_screen_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_two_player_merge() -> Scenario {
    Scenario::builder("split_screen_two_player_merge")
        .description(
            "Start from a diagonal split, capture it, collapse back to a merged shared view, assert both HUD roots target the same camera, and capture the merged result.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::SlantedSplit);
        })))
        .then(Action::WaitFrames(10))
        .then(Action::Screenshot("two_player_merge_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::Merge);
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("layout collapses to shared mode", |world| {
            let snapshot = snapshot(world);
            snapshot.mode == SplitScreenLayoutMode::Shared
                && snapshot.views.iter().filter(|view| view.active).count() == 1
        }))
        .then(assertions::custom("both HUD roots target the merged owner camera", |world| {
            let first = ui_target_for_slot(world, 0);
            let second = ui_target_for_slot(world, 1);
            first.is_some() && first == second
        }))
        .then(assertions::log_summary("split_screen_two_player_merge summary"))
        .then(Action::Screenshot("two_player_merge_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_slanted_split() -> Scenario {
    Scenario::builder("split_screen_two_player_slanted_split")
        .description(
            "Begin in merged mode, expand to a diagonal split, assert the divider metadata is slanted, and capture both checkpoints.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::Merge);
        })))
        .then(Action::WaitFrames(10))
        .then(Action::Screenshot("two_player_slanted_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::SlantedSplit);
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("layout enters diagonal dynamic split", |world| {
            let snapshot = snapshot(world);
            snapshot.mode == SplitScreenLayoutMode::DynamicTwoPlayer
                && snapshot
                    .divider
                    .as_ref()
                    .is_some_and(|divider| divider.normalized_start.y != divider.normalized_end.y)
        }))
        .then(assertions::log_summary("split_screen_two_player_slanted_split summary"))
        .then(Action::Screenshot("two_player_slanted_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_weighted_dynamic() -> Scenario {
    Scenario::builder("split_screen_weighted_dynamic")
        .description(
            "Switch to the weighted two-player presentation, assert the seam shifts off center and Player 1 receives more viewport area, then capture the result.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::WeightedSplit);
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("weighted split biases area toward player 1", |world| {
            let snapshot = snapshot(world);
            if snapshot.mode != SplitScreenLayoutMode::DynamicTwoPlayer {
                return false;
            }

            let Some(view_a) = snapshot.views.iter().find(|view| view.slot == LocalPlayerSlot(0)) else {
                return false;
            };
            let Some(view_b) = snapshot.views.iter().find(|view| view.slot == LocalPlayerSlot(1)) else {
                return false;
            };
            let area_a = view_a.physical.size.x * view_a.physical.size.y;
            let area_b = view_b.physical.size.x * view_b.physical.size.y;
            area_a > area_b
        }))
        .then(assertions::custom("divider midpoint is no longer centered", |world| {
            let snapshot = snapshot(world);
            let Some(divider) = snapshot.divider.as_ref() else {
                return false;
            };
            let midpoint_x = (divider.normalized_start.x + divider.normalized_end.x) * 0.5;
            (midpoint_x - 0.5).abs() > 0.04
        }))
        .then(assertions::log_summary("split_screen_weighted_dynamic summary"))
        .then(Action::Screenshot("split_screen_weighted_dynamic".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_resize() -> Scenario {
    Scenario::builder("split_screen_resize")
        .description(
            "Run the diagonal split layout, resize the primary window, assert the physical viewport dimensions change without invalid rectangles, and capture before/after frames.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::SlantedSplit);
        })))
        .then(Action::WaitFrames(10))
        .then(Action::Screenshot("split_screen_resize_before".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world| {
            let window_entity = world
                .query_filtered::<Entity, With<PrimaryWindow>>()
                .single(world)
                .expect("primary window");
            let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
            let mut window = windows.single_mut(world).expect("primary window");
            window.resolution.set_physical_resolution(960, 540);
            world.write_message(bevy::window::WindowResized {
                window: window_entity,
                width: 960.0,
                height: 540.0,
            });
        })))
        .then(Action::WaitFrames(5))
        .then(assertions::custom("resized layout stays valid", |world| {
            let snapshot = snapshot(world);
            snapshot.target.physical_size == UVec2::new(960, 540)
                && snapshot
                    .views
                    .iter()
                    .filter(|view| view.active)
                    .all(|view| view.physical.size.x > 0 && view.physical.size.y > 0)
        }))
        .then(assertions::log_summary("split_screen_resize summary"))
        .then(Action::Screenshot("split_screen_resize_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_four_player() -> Scenario {
    Scenario::builder("split_screen_four_player")
        .description(
            "Switch the lab into its four-player arrangement, assert that all four views are active, and capture the balanced layout.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::FourPlayer);
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("four views become active", |world| {
            let snapshot = snapshot(world);
            snapshot.views.iter().filter(|view| view.active).count() == 4
                && matches!(
                    snapshot.mode,
                    SplitScreenLayoutMode::FixedFourGrid
                        | SplitScreenLayoutMode::FixedFourVerticalStrip
                        | SplitScreenLayoutMode::FixedFourHorizontalStrip
                )
        }))
        .then(assertions::log_summary("split_screen_four_player summary"))
        .then(Action::Screenshot("split_screen_four_player".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_per_player_ui() -> Scenario {
    Scenario::builder("split_screen_per_player_ui")
        .description(
            "Switch to the UI-focused presentation, assert that each active HUD root resolves to a distinct camera, and capture the result.",
        )
        .then(Action::Custom(Box::new(|world| {
            set_lab_presentation(world, LabPresentation::PerPlayerUi);
        })))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("per-player HUDs target different cameras", |world| {
            let first = ui_target_for_slot(world, 0);
            let second = ui_target_for_slot(world, 1);
            first.is_some() && second.is_some() && first != second
        }))
        .then(assertions::custom("ui presentation stays split", |world| {
            let snapshot = snapshot(world);
            snapshot.mode != SplitScreenLayoutMode::Shared
        }))
        .then(assertions::log_summary("split_screen_per_player_ui summary"))
        .then(Action::Screenshot("split_screen_per_player_ui".into()))
        .then(Action::WaitFrames(1))
        .build()
}
