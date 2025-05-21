// src/game.rs
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::{
    horror::{HorrorSpawnTimer, MaxHorrors},
    echoing_soul::{EchoingSoul, EchoingSoulPlugin},
    survivor::{Survivor, SanityStrain},
    components::Health,
    upgrades::{UpgradePlugin, UpgradePool, OfferedUpgrades, UpgradeCard, UpgradeType},
    weapons::{CircleOfWarding, SwarmOfNightmares},
    audio::{PlaySoundEvent, SoundEffect},
    debug_menu::DebugMenuPlugin,
    items::{ItemId, ItemLibrary, AutomaticWeaponId, AutomaticWeaponLibrary, AutomaticWeaponDefinition},
    skills::{ActiveSkillInstance, SkillLibrary, SkillDefinition},
    automatic_projectiles::AutomaticProjectile,
    glyphs::{GlyphLibrary, GlyphDefinition, GlyphId},
};

pub const SCREEN_WIDTH: f32 = 1280.0;
pub const SCREEN_HEIGHT: f32 = 720.0;
const INITIAL_MAX_HORRORS: u32 = 20;
const INITIAL_SPAWN_INTERVAL_SECONDS: f32 = 2.0;
const DIFFICULTY_INCREASE_INTERVAL_SECONDS: f32 = 30.0;
const MAX_HORRORS_INCREMENT: u32 = 10;
const SPAWN_INTERVAL_DECREMENT_FACTOR: f32 = 0.9;
const MIN_SPAWN_INTERVAL_SECONDS: f32 = 0.3;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default] MainMenu,
    InGame,
    LevelUp,
    GameOver,
    DebugUpgradeMenu,
    GlyphScreen,
}

#[derive(Resource, Default)]
struct PreviousGameState(Option<AppState>);

#[derive(Resource)]
pub struct GameConfig { pub width: f32, pub height: f32, pub spawn_area_padding: f32, }
impl Default for GameConfig { fn default() -> Self { Self { width: SCREEN_WIDTH, height: SCREEN_HEIGHT, spawn_area_padding: 50.0 } } }
pub struct GamePlugin;
#[derive(Resource, Default)]
pub struct GameState { pub score: u32, pub cycle_number: u32, pub horror_count: u32, pub game_over_timer: Timer, pub game_timer: Timer, pub difficulty_timer: Timer, }
#[derive(Event)] pub struct UpgradeChosenEvent(pub UpgradeCard);
#[derive(Event)] pub struct ItemCollectedEvent(pub ItemId);

// --- Components for Glyph Screen UI ---
#[derive(Component)] struct MainMenuUI;
#[derive(Component)] struct LevelUpUI;
#[derive(Component)] struct UpgradeButton(UpgradeCard);
#[derive(Component)] struct GameOverUI;
#[derive(Component)] struct InGameUI;
#[derive(Component)] struct GlyphScreenUI;
#[derive(Component)] struct EnduranceText;
#[derive(Component)] struct InsightText;
#[derive(Component)] struct EchoesText;
#[derive(Component)] struct ScoreText;
#[derive(Component)] struct TimerText;
#[derive(Component)] struct CycleText;

#[derive(Component)]
struct GlyphInventoryButton(GlyphId);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum GlyphSocketTargetType {
    ActiveSkill,
    AutomaticWeapon,
}

#[derive(Component)]
struct GlyphTargetSlotButton {
    target_type: GlyphSocketTargetType,
    target_entity_slot_idx: usize, // For active skill index, or 0 if auto weapon
    glyph_slot_idx: usize,
}

// --- Event for Socketing ---
#[derive(Event)]
struct SocketGlyphRequestedEvent {
    glyph_to_socket: GlyphId,
    target_type: GlyphSocketTargetType,
    target_entity_slot_idx: usize,
    glyph_slot_idx: usize,
}

// --- Resource to track selected glyph ---
#[derive(Resource, Default)]
struct SelectedGlyphForSocketing(Option<GlyphId>);


fn reset_for_new_game_session(
    mut game_state: ResMut<GameState>,
    mut horror_spawn_timer: ResMut<HorrorSpawnTimer>,
    mut max_horrors: ResMut<MaxHorrors>,
) {
    game_state.score = 0;
    game_state.cycle_number = 1;
    game_state.horror_count = 0;
    game_state.game_timer = Timer::from_seconds(3600.0, TimerMode::Once);
    game_state.game_timer.reset();
    game_state.game_timer.unpause();
    game_state.difficulty_timer = Timer::from_seconds(DIFFICULTY_INCREASE_INTERVAL_SECONDS, TimerMode::Repeating);
    game_state.difficulty_timer.reset();
    horror_spawn_timer.timer.set_duration(std::time::Duration::from_secs_f32(INITIAL_SPAWN_INTERVAL_SECONDS));
    horror_spawn_timer.timer.reset();
    max_horrors.0 = INITIAL_MAX_HORRORS;
}

fn on_enter_ingame_state_actions(mut game_state: ResMut<GameState>) {
    if game_state.game_timer.paused() { game_state.game_timer.unpause(); }
    if game_state.difficulty_timer.paused() { game_state.difficulty_timer.unpause(); }
}

fn on_enter_pause_like_state_actions(mut game_state: ResMut<GameState>, _current_app_state: Res<State<AppState>>) {
    if !game_state.game_timer.paused() { game_state.game_timer.pause(); }
    if !game_state.difficulty_timer.paused() { game_state.difficulty_timer.pause(); }
}
fn log_entering_debug_menu_state() {}
fn log_exiting_debug_menu_state() {}


impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app .add_event::<UpgradeChosenEvent>() .add_event::<ItemCollectedEvent>()
            .add_event::<SocketGlyphRequestedEvent>() // Added event
            .add_plugins((UpgradePlugin, DebugMenuPlugin)) .init_state::<AppState>()
            .init_resource::<GameConfig>() .init_resource::<GameState>()
            .init_resource::<PreviousGameState>()
            .init_resource::<SelectedGlyphForSocketing>() // Added resource
            .insert_resource(HorrorSpawnTimer {timer: Timer::from_seconds(INITIAL_SPAWN_INTERVAL_SECONDS, TimerMode::Repeating)})
            .insert_resource(MaxHorrors(INITIAL_MAX_HORRORS)) .add_plugins(EchoingSoulPlugin)

            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu_ui)
            .add_systems(Update, main_menu_input_system.run_if(in_state(AppState::MainMenu)))
            .add_systems(OnExit(AppState::MainMenu), despawn_ui_by_marker::<MainMenuUI>)

            .add_systems(OnEnter(AppState::InGame), (on_enter_ingame_state_actions, setup_ingame_ui,))
            .add_systems(Update, (
                update_ingame_ui,
                update_game_timer,
                difficulty_scaling_system,
                global_key_listener,
                debug_weapon_switch_system,
            ).chain().run_if(in_state(AppState::InGame).or_else(in_state(AppState::DebugUpgradeMenu))))
            .add_systems(OnExit(AppState::InGame), (cleanup_session_entities, despawn_ui_by_marker::<InGameUI>))

            .add_systems(OnEnter(AppState::LevelUp), (setup_level_up_ui, on_enter_pause_like_state_actions))
            .add_systems(Update, handle_upgrade_choice_interaction.run_if(in_state(AppState::LevelUp)))
            .add_systems(Update, apply_chosen_upgrade.run_if(on_event::<UpgradeChosenEvent>()))
            .add_systems(OnExit(AppState::LevelUp), (despawn_ui_by_marker::<LevelUpUI>, on_enter_ingame_state_actions))

            .add_systems(OnEnter(AppState::DebugUpgradeMenu), (on_enter_pause_like_state_actions, log_entering_debug_menu_state))
            .add_systems(OnExit(AppState::DebugUpgradeMenu), (on_enter_ingame_state_actions, log_exiting_debug_menu_state))

            .add_systems(OnEnter(AppState::GlyphScreen), (setup_glyph_screen_ui, on_enter_pause_like_state_actions))
            .add_systems(Update, (
                glyph_screen_input_system,
                glyph_screen_button_interaction_system, // Added system
                handle_socket_glyph_request_system.run_if(on_event::<SocketGlyphRequestedEvent>()), // Added system
            ).chain().run_if(in_state(AppState::GlyphScreen)))
            .add_systems(OnExit(AppState::GlyphScreen), (
                despawn_ui_by_marker::<GlyphScreenUI>,
                on_enter_ingame_state_actions,
                |mut selected_glyph: ResMut<SelectedGlyphForSocketing>| selected_glyph.0 = None, // Clear selection on exit
            ))

            .add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
            .add_systems(Update, game_over_input_system.run_if(in_state(AppState::GameOver)))
            .add_systems(OnExit(AppState::GameOver), despawn_ui_by_marker::<GameOverUI>);
    }
}

fn global_key_listener(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut prev_game_state: ResMut<PreviousGameState>,
) {
    if keyboard_input.just_pressed(KeyCode::Backquote) {
        match current_app_state.get() {
            AppState::InGame => {
                prev_game_state.0 = Some(AppState::InGame);
                next_app_state.set(AppState::DebugUpgradeMenu);
            }
            AppState::DebugUpgradeMenu => {
                if let Some(prev) = prev_game_state.0.take() {
                    next_app_state.set(prev);
                } else {
                    next_app_state.set(AppState::InGame);
                }
            }
            _ => {}
        }
    }
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        match current_app_state.get() {
            AppState::InGame => {
                prev_game_state.0 = Some(AppState::InGame);
                next_app_state.set(AppState::GlyphScreen);
            }
            AppState::DebugUpgradeMenu => {
                prev_game_state.0 = Some(AppState::DebugUpgradeMenu);
                next_app_state.set(AppState::GlyphScreen);
            }
            AppState::GlyphScreen => {
                if let Some(prev) = prev_game_state.0.take() {
                    next_app_state.set(prev);
                } else {
                    next_app_state.set(AppState::InGame);
                }
            }
            _ => {}
        }
    }
}

fn glyph_screen_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut prev_game_state: ResMut<PreviousGameState>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) || keyboard_input.just_pressed(KeyCode::KeyG) {
        if let Some(prev) = prev_game_state.0.take() {
            next_app_state.set(prev);
        } else {
            next_app_state.set(AppState::InGame);
        }
    }
}


fn setup_glyph_screen_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<&Survivor>,
    glyph_library: Res<GlyphLibrary>,
    skill_library: Res<SkillLibrary>,
    weapon_library: Res<AutomaticWeaponLibrary>,
    selected_glyph: Res<SelectedGlyphForSocketing>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let title_style = TextStyle { font: font.clone(), font_size: 30.0, color: Color::WHITE };
    let header_style = TextStyle { font: font.clone(), font_size: 20.0, color: Color::CYAN };
    let item_name_style = TextStyle { font: font.clone(), font_size: 16.0, color: Color::WHITE };
    let item_desc_style = TextStyle { font: font.clone(), font_size: 14.0, color: Color::GRAY };
    let slot_style = TextStyle { font: font.clone(), font_size: 14.0, color: Color::YELLOW };
    let empty_slot_style = TextStyle { font: font.clone(), font_size: 14.0, color: Color::DARK_GRAY };
    let button_text_style = TextStyle { font: font.clone(), font_size: 14.0, color: Color::WHITE };
    let selected_button_color = Color::rgb(0.2, 0.5, 0.2);
    let default_button_color = Color::rgb(0.25, 0.25, 0.25);


    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::FlexStart, 
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(15.0),
                ..default()
            },
            background_color: Color::rgba(0.05, 0.05, 0.15, 0.95).into(),
            z_index: ZIndex::Global(20),
            ..default()
        },
        GlyphScreenUI,
        Name::new("GlyphScreenUIRoot"),
    ))
    .with_children(|parent| {
        parent.spawn(TextBundle::from_section("Glyph Socketing", title_style.clone()));
        parent.spawn(TextBundle::from_section("Press 'G' or 'Esc' to Close", item_desc_style.clone()));

        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(80.0), 
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceAround,
                ..default()
            },
            ..default()
        }).with_children(|main_layout| {
            main_layout.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(30.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(5.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(Color::GRAY),
                background_color: Color::rgba(0.1, 0.1, 0.2, 0.8).into(),
                ..default()
            }).with_children(|inventory_panel| {
                inventory_panel.spawn(TextBundle::from_section("Glyph Inventory", header_style.clone()));
                if let Ok(player) = player_query.get_single() {
                    if player.collected_glyphs.is_empty() {
                        inventory_panel.spawn(TextBundle::from_section("No glyphs collected.", item_desc_style.clone()));
                    } else {
                        for glyph_id in player.collected_glyphs.iter() {
                            if let Some(glyph_def) = glyph_library.get_glyph_definition(*glyph_id) {
                                let is_selected = selected_glyph.0 == Some(*glyph_id);
                                inventory_panel.spawn((
                                    ButtonBundle {
                                        style: Style {
                                            width: Val::Percent(100.0),
                                            padding: UiRect::all(Val::Px(8.0)),
                                            margin: UiRect::bottom(Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            flex_direction: FlexDirection::Column,
                                            ..default()
                                        },
                                        border_color: BorderColor(if is_selected {Color::GREEN} else {Color::DARK_GRAY}),
                                        background_color: (if is_selected {selected_button_color} else {default_button_color}).into(),
                                        ..default()
                                    },
                                    GlyphInventoryButton(*glyph_id),
                                    Name::new(format!("InvGlyph: {}", glyph_def.name))
                                )).with_children(|glyph_button|{
                                    glyph_button.spawn(TextBundle::from_section(glyph_def.name.clone(), item_name_style.clone()));
                                    glyph_button.spawn(TextBundle::from_section(glyph_def.description.clone(), item_desc_style.clone()));
                                });
                            }
                        }
                    }
                } else {
                    inventory_panel.spawn(TextBundle::from_section("Error: Player not found.", item_desc_style.clone()));
                }
            });

            main_layout.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(65.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(Color::GRAY),
                background_color: Color::rgba(0.1, 0.1, 0.2, 0.8).into(),
                ..default()
            }).with_children(|socketing_panel| {
                socketing_panel.spawn(TextBundle::from_section("Socket Glyphs Into:", header_style.clone()));

                if let Ok(player) = player_query.get_single() {
                    socketing_panel.spawn(TextBundle::from_section("Active Skills:", item_name_style.clone()).with_style(Style{margin: UiRect::bottom(Val::Px(5.0)), ..default()}));
                    for (skill_idx, active_skill) in player.equipped_skills.iter().enumerate() {
                        if let Some(skill_def) = skill_library.get_skill_definition(active_skill.definition_id) {
                            socketing_panel.spawn(NodeBundle{
                                style: Style { flex_direction: FlexDirection::Column, margin: UiRect::bottom(Val::Px(8.0)), ..default()},
                                ..default()
                            }).with_children(|skill_node| {
                                skill_node.spawn(TextBundle::from_section(format!("  {}: {}", skill_idx + 1, skill_def.name), item_name_style.clone()));
                                for (glyph_slot_idx, glyph_id_opt) in active_skill.equipped_glyphs.iter().enumerate() {
                                    if let Some(gid) = glyph_id_opt {
                                        let glyph_name = glyph_library.get_glyph_definition(*gid).map_or("Unknown Glyph".to_string(), |g_def| g_def.name.clone());
                                        skill_node.spawn(TextBundle::from_section(format!("    Slot {}: {}", glyph_slot_idx + 1, glyph_name), slot_style.clone()));
                                    } else {
                                        skill_node.spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    padding: UiRect::new(Val::Px(15.0), Val::Px(5.0), Val::Px(5.0), Val::Px(2.0)),
                                                    margin: UiRect::left(Val::Px(20.0)),
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                border_color: BorderColor(Color::DARK_GRAY),
                                                background_color: default_button_color.into(),
                                                ..default()
                                            },
                                            GlyphTargetSlotButton {
                                                target_type: GlyphSocketTargetType::ActiveSkill,
                                                target_entity_slot_idx: skill_idx,
                                                glyph_slot_idx,
                                            },
                                            Name::new(format!("SkillSocket:S{}:GS{}", skill_idx, glyph_slot_idx))
                                        )).with_children(|slot_button| {
                                            slot_button.spawn(TextBundle::from_section(format!("    Slot {}: EMPTY (Click to Socket)", glyph_slot_idx + 1), empty_slot_style.clone()));
                                        });
                                    }
                                }
                            });
                        }
                    }
                    socketing_panel.spawn(NodeBundle{style: Style{height: Val::Px(10.0), ..default()}, ..default()});

                    socketing_panel.spawn(TextBundle::from_section("Automatic Weapon:", item_name_style.clone()).with_style(Style{margin: UiRect::bottom(Val::Px(5.0)), ..default()}));
                    if let Some(weapon_id) = player.equipped_weapon_id {
                        if let Some(weapon_def) = weapon_library.get_weapon_definition(weapon_id) {
                             socketing_panel.spawn(NodeBundle{
                                style: Style { flex_direction: FlexDirection::Column, margin: UiRect::bottom(Val::Px(8.0)), ..default()},
                                ..default()
                            }).with_children(|weapon_node| {
                                weapon_node.spawn(TextBundle::from_section(format!("  Equipped: {}", weapon_def.name), item_name_style.clone()));
                                for (glyph_slot_idx, glyph_id_opt) in player.auto_weapon_equipped_glyphs.iter().enumerate() {
                                    if let Some(gid) = glyph_id_opt {
                                        let glyph_name = glyph_library.get_glyph_definition(*gid).map_or("Unknown Glyph".to_string(), |g_def| g_def.name.clone());
                                        weapon_node.spawn(TextBundle::from_section(format!("    Slot {}: {}", glyph_slot_idx + 1, glyph_name), slot_style.clone()));
                                    } else {
                                        weapon_node.spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    padding: UiRect::new(Val::Px(15.0), Val::Px(5.0), Val::Px(5.0), Val::Px(2.0)),
                                                    margin: UiRect::left(Val::Px(20.0)),
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                border_color: BorderColor(Color::DARK_GRAY),
                                                background_color: default_button_color.into(),
                                                ..default()
                                            },
                                            GlyphTargetSlotButton {
                                                target_type: GlyphSocketTargetType::AutomaticWeapon,
                                                target_entity_slot_idx: 0, // Only one auto weapon
                                                glyph_slot_idx,
                                            },
                                            Name::new(format!("AutoWpnSocket:GS{}", glyph_slot_idx))
                                        )).with_children(|slot_button| {
                                            slot_button.spawn(TextBundle::from_section(format!("    Slot {}: EMPTY (Click to Socket)", glyph_slot_idx + 1), empty_slot_style.clone()));
                                        });
                                    }
                                }
                            });
                        } else {
                            socketing_panel.spawn(TextBundle::from_section("  No weapon definition found.", item_desc_style.clone()));
                        }
                    } else {
                        socketing_panel.spawn(TextBundle::from_section("  No weapon equipped.", item_desc_style.clone()));
                    }

                } else {
                     socketing_panel.spawn(TextBundle::from_section("Error: Player not found.", item_desc_style.clone()));
                }
            });
        });
    });
}

fn glyph_screen_button_interaction_system(
    mut interaction_query: ParamSet<(
        Query<(&Interaction, &GlyphInventoryButton, &mut BackgroundColor, &mut BorderColor), (Changed<Interaction>, With<Button>)>,
        Query<(&Interaction, &GlyphTargetSlotButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
    )>,
    mut selected_glyph_for_socketing: ResMut<SelectedGlyphForSocketing>,
    mut socket_event_writer: EventWriter<SocketGlyphRequestedEvent>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
    mut query_glyph_buttons: Query<(&GlyphInventoryButton, &mut BackgroundColor, &mut BorderColor), Without<GlyphTargetSlotButton>>, // For resetting others
) {
    let selected_glyph_id_before_click = selected_glyph_for_socketing.0;

    for (interaction, inv_button, mut bg_color, mut border_color) in interaction_query.p0().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); // Or a more UI-specific sound
                if selected_glyph_for_socketing.0 == Some(inv_button.0) {
                    // Deselect if clicking the same selected glyph
                    selected_glyph_for_socketing.0 = None;
                    *bg_color = Color::rgb(0.25, 0.25, 0.25).into();
                    *border_color = Color::DARK_GRAY.into();
                } else {
                    // Deselect all other inventory buttons visually
                    for (other_inv_button, mut other_bg, mut other_border) in query_glyph_buttons.iter_mut() {
                        if other_inv_button.0 != inv_button.0 {
                             *other_bg = Color::rgb(0.25, 0.25, 0.25).into();
                             *other_border = Color::DARK_GRAY.into();
                        }
                    }
                    // Select this one
                    selected_glyph_for_socketing.0 = Some(inv_button.0);
                    *bg_color = Color::rgb(0.2, 0.5, 0.2).into();
                    *border_color = Color::GREEN.into();
                }
            }
            Interaction::Hovered => {
                if selected_glyph_for_socketing.0 != Some(inv_button.0) { // Don't change hover if selected
                    *bg_color = Color::rgb(0.35, 0.35, 0.35).into();
                }
            }
            Interaction::None => {
                 if selected_glyph_for_socketing.0 != Some(inv_button.0) { // Don't change if selected
                    *bg_color = Color::rgb(0.25, 0.25, 0.25).into();
                 }
            }
        }
    }

    for (interaction, slot_button, mut bg_color) in interaction_query.p1().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if let Some(glyph_to_socket) = selected_glyph_id_before_click {
                    sound_event_writer.send(PlaySoundEvent(SoundEffect::RitualCast)); // Socketing sound
                    socket_event_writer.send(SocketGlyphRequestedEvent {
                        glyph_to_socket,
                        target_type: slot_button.target_type,
                        target_entity_slot_idx: slot_button.target_entity_slot_idx,
                        glyph_slot_idx: slot_button.glyph_slot_idx,
                    });
                    selected_glyph_for_socketing.0 = None; // Clear selection after attempting to socket
                } else {
                    sound_event_writer.send(PlaySoundEvent(SoundEffect::SurvivorHit)); // Error/denial sound
                }
                *bg_color = Color::rgb(0.15, 0.15, 0.15).into();
            }
            Interaction::Hovered => {*bg_color = Color::rgb(0.35, 0.35, 0.35).into(); }
            Interaction::None => { *bg_color = Color::rgb(0.25, 0.25, 0.25).into(); }
        }
    }
}

fn handle_socket_glyph_request_system(
    mut events: EventReader<SocketGlyphRequestedEvent>,
    mut player_query: Query<&mut Survivor>,
    mut commands: Commands, // To despawn and respawn UI for refresh
    asset_server: Res<AssetServer>,
    glyph_library: Res<GlyphLibrary>,
    skill_library: Res<SkillLibrary>,
    weapon_library: Res<AutomaticWeaponLibrary>,
    selected_glyph: Res<SelectedGlyphForSocketing>, // To pass to setup function
    ui_root_query: Query<Entity, With<GlyphScreenUI>>, // To find the UI to despawn
) {
    if let Ok(mut player) = player_query.get_single_mut() {
        let mut refresh_ui = false;
        for event in events.read() {
            if let Some(collected_glyph_index) = player.collected_glyphs.iter().position(|&id| id == event.glyph_to_socket) {
                match event.target_type {
                    GlyphSocketTargetType::ActiveSkill => {
                        if let Some(skill_instance) = player.equipped_skills.get_mut(event.target_entity_slot_idx) {
                            if event.glyph_slot_idx < skill_instance.equipped_glyphs.len() && skill_instance.equipped_glyphs[event.glyph_slot_idx].is_none() {
                                skill_instance.equipped_glyphs[event.glyph_slot_idx] = Some(event.glyph_to_socket);
                                player.collected_glyphs.remove(collected_glyph_index);
                                refresh_ui = true;
                            }
                        }
                    }
                    GlyphSocketTargetType::AutomaticWeapon => {
                        if event.glyph_slot_idx < player.auto_weapon_equipped_glyphs.len() && player.auto_weapon_equipped_glyphs[event.glyph_slot_idx].is_none() {
                            player.auto_weapon_equipped_glyphs[event.glyph_slot_idx] = Some(event.glyph_to_socket);
                            player.collected_glyphs.remove(collected_glyph_index);
                            refresh_ui = true;
                        }
                    }
                }
            }
        }

        if refresh_ui {
            // Despawn existing UI
            for entity in ui_root_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
            // Respawn UI
            setup_glyph_screen_ui(commands, asset_server, Query::new(&player_query.world(), ()), glyph_library, skill_library, weapon_library, selected_glyph);
        }
    }
}


fn debug_weapon_switch_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut Survivor, &mut SanityStrain)>,
    weapon_library: Res<AutomaticWeaponLibrary>,
    current_app_state: Res<State<AppState>>,
) {
    if !matches!(*current_app_state.get(), AppState::InGame | AppState::DebugUpgradeMenu) {
        return;
    }

    if let Ok((mut survivor, mut sanity_strain)) = player_query.get_single_mut() {
        let num_defined_weapons = weapon_library.weapons.len() as u32;
        if num_defined_weapons == 0 { return; }

        let mut current_weapon_idx = survivor.equipped_weapon_id.map_or(0, |id| id.0);

        if keyboard_input.just_pressed(KeyCode::F5) {
            current_weapon_idx = (current_weapon_idx + 1) % num_defined_weapons;
        } else if keyboard_input.just_pressed(KeyCode::F6) {
            current_weapon_idx = if current_weapon_idx == 0 { num_defined_weapons - 1 } else { current_weapon_idx - 1};
        } else {
            return;
        }

        let new_weapon_id = AutomaticWeaponId(current_weapon_idx);
        if let Some(new_weapon_def) = weapon_library.get_weapon_definition(new_weapon_id) {
            survivor.equipped_weapon_id = Some(new_weapon_id);
            sanity_strain.base_fire_rate_secs = new_weapon_def.base_fire_rate_secs;
            survivor.auto_weapon_equipped_glyphs = vec![None; new_weapon_def.base_glyph_slots as usize];
        }
    }
}

fn despawn_ui_by_marker<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) { for entity in query.iter() { commands.entity(entity).despawn_recursive(); } }
fn setup_main_menu_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0), ..default() }, ..default() }, MainMenuUI, )).with_children(|parent| { parent.spawn( TextBundle::from_section( "Echoes of the Abyss", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 70.0, color: Color::WHITE, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( "Embrace the Madness (SPACE)", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 40.0, color: Color::rgba(0.8, 0.8, 0.8, 1.0), }, ).with_text_justify(JustifyText::Center) ); }); }
fn main_menu_input_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>, game_state: ResMut<GameState>, horror_spawn_timer: ResMut<HorrorSpawnTimer>, max_horrors: ResMut<MaxHorrors>, player_entity_query: Query<Entity, With<Survivor>>,) { if keyboard_input.just_pressed(KeyCode::Space) { for entity in player_entity_query.iter() { commands.entity(entity).despawn_recursive(); } reset_for_new_game_session(game_state, horror_spawn_timer, max_horrors); next_app_state.set(AppState::InGame); } }
fn setup_ingame_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(Val::Px(10.0)), position_type: PositionType::Absolute, ..default() }, z_index: ZIndex::Global(1), ..default() }, InGameUI, )).with_children(|parent| { parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceAround, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(5.0)), ..default() }, background_color: Color::rgba(0.0, 0.0, 0.0, 0.3).into(), ..default() }).with_children(|top_bar| { top_bar.spawn((TextBundle::from_section( "Endurance: 100", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::GREEN, }, ), EnduranceText)); top_bar.spawn((TextBundle::from_section( "Insight: 1", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::CYAN, }, ), InsightText)); top_bar.spawn((TextBundle::from_section( "Echoes: 0/100", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::YELLOW, }, ), EchoesText)); top_bar.spawn((TextBundle::from_section( "Cycle: 1", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::ORANGE_RED, }, ), CycleText)); }); parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::FlexEnd, padding: UiRect::all(Val::Px(5.0)), ..default() }, ..default() }).with_children(|bottom_bar| { bottom_bar.spawn((TextBundle::from_section( "Score: 0", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), ScoreText)); bottom_bar.spawn((TextBundle::from_section( "Time: 00:00", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), TimerText)); }); }); }
fn update_game_timer(mut game_state: ResMut<GameState>, time: Res<Time>) { if !game_state.game_timer.paused() { game_state.game_timer.tick(time.delta()); } }
fn difficulty_scaling_system(time: Res<Time>, mut game_state: ResMut<GameState>, mut horror_spawn_timer: ResMut<HorrorSpawnTimer>, mut max_horrors: ResMut<MaxHorrors>,) { if game_state.difficulty_timer.paused() { return; } game_state.difficulty_timer.tick(time.delta()); if game_state.difficulty_timer.just_finished() { game_state.cycle_number += 1; max_horrors.0 = (INITIAL_MAX_HORRORS + (game_state.cycle_number -1) * MAX_HORRORS_INCREMENT).min(200); let current_duration = horror_spawn_timer.timer.duration().as_secs_f32(); let new_duration = (current_duration * SPAWN_INTERVAL_DECREMENT_FACTOR).max(MIN_SPAWN_INTERVAL_SECONDS); horror_spawn_timer.timer.set_duration(std::time::Duration::from_secs_f32(new_duration)); } }
fn update_ingame_ui(player_query: Query<(&Survivor, &Health)>, game_state: Res<GameState>, mut ui_texts: ParamSet< ( Query<&mut Text, With<EnduranceText>>, Query<&mut Text, With<InsightText>>, Query<&mut Text, With<EchoesText>>, Query<&mut Text, With<ScoreText>>, Query<&mut Text, With<TimerText>>, Query<&mut Text, With<CycleText>>, )>,) { if let Ok((player_stats, player_health)) = player_query.get_single() { if let Ok(mut text) = ui_texts.p0().get_single_mut() { text.sections[0].value = format!("Endurance: {}/{}", player_health.0, player_stats.max_health); if player_health.0 < player_stats.max_health / 3 { text.sections[0].style.color = Color::RED; } else if player_health.0 < player_stats.max_health * 2 / 3 { text.sections[0].style.color = Color::YELLOW; } else { text.sections[0].style.color = Color::GREEN; } } if let Ok(mut text) = ui_texts.p1().get_single_mut() { text.sections[0].value = format!("Insight: {}", player_stats.level); } if let Ok(mut text) = ui_texts.p2().get_single_mut() { text.sections[0].value = format!("Echoes: {}/{}", player_stats.current_level_xp, player_stats.experience_to_next_level()); } } else { if let Ok(mut text) = ui_texts.p0().get_single_mut() { text.sections[0].value = "Endurance: --/--".to_string(); } if let Ok(mut text) = ui_texts.p1().get_single_mut() { text.sections[0].value = "Insight: --".to_string(); } if let Ok(mut text) = ui_texts.p2().get_single_mut() { text.sections[0].value = "Echoes: --/--".to_string(); } } if let Ok(mut text) = ui_texts.p3().get_single_mut() { text.sections[0].value = format!("Score: {}", game_state.score); } if let Ok(mut text) = ui_texts.p4().get_single_mut() { let elapsed_seconds = game_state.game_timer.elapsed().as_secs(); let minutes = elapsed_seconds / 60; let seconds = elapsed_seconds % 60; text.sections[0].value = format!("Time: {:02}:{:02}", minutes, seconds); } if let Ok(mut text) = ui_texts.p5().get_single_mut() { text.sections[0].value = format!("Cycle: {}", game_state.cycle_number); } }
fn setup_level_up_ui(mut commands: Commands, asset_server: Res<AssetServer>, player_query: Query<&Survivor>, upgrade_pool: Res<UpgradePool>,) { let player_level = if let Ok(player) = player_query.get_single() { player.level } else { 0 }; let current_offered_upgrades = OfferedUpgrades { choices: upgrade_pool.get_random_upgrades(3) }; commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(30.0), ..default() }, background_color: Color::rgba(0.1, 0.1, 0.2, 0.9).into(), z_index: ZIndex::Global(10), ..default() }, LevelUpUI, current_offered_upgrades.clone(), )).with_children(|parent| { parent.spawn( TextBundle::from_section( format!("Revelation! Insight: {}", player_level), TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 50.0, color: Color::GOLD, }, ).with_style(Style { margin: UiRect::bottom(Val::Px(20.0)), ..default()}) ); for (index, card) in current_offered_upgrades.choices.iter().enumerate() { parent.spawn(( ButtonBundle { style: Style { width: Val::Px(400.0), height: Val::Px(120.0), padding: UiRect::all(Val::Px(10.0)), justify_content: JustifyContent::Center, align_items: AlignItems::FlexStart, flex_direction: FlexDirection::Column, border: UiRect::all(Val::Px(2.0)), margin: UiRect::bottom(Val::Px(10.0)), ..default() }, border_color: BorderColor(Color::DARK_GRAY), background_color: Color::GRAY.into(), ..default() }, UpgradeButton(card.clone()), Name::new(format!("Upgrade Button {}", index + 1)), )).with_children(|button_parent| { button_parent.spawn(TextBundle::from_section( &card.name, TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 24.0, color: Color::WHITE, }, ).with_style(Style { margin: UiRect::bottom(Val::Px(5.0)), ..default() })); button_parent.spawn(TextBundle::from_section( &card.description, TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 18.0, color: Color::rgb(0.9, 0.9, 0.9), }, )); }); } }); }
fn handle_upgrade_choice_interaction(mut interaction_query: Query< (&Interaction, &UpgradeButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>), >, mut upgrade_chosen_event: EventWriter<UpgradeChosenEvent>, mut next_app_state: ResMut<NextState<AppState>>, keyboard_input: Res<ButtonInput<KeyCode>>, level_up_ui_query: Query<&OfferedUpgrades, With<LevelUpUI>>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (interaction, upgrade_button_data, mut bg_color) in interaction_query.iter_mut() { match *interaction { Interaction::Pressed => { sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); upgrade_chosen_event.send(UpgradeChosenEvent(upgrade_button_data.0.clone())); next_app_state.set(AppState::InGame); return; } Interaction::Hovered => { *bg_color = Color::DARK_GREEN.into(); } Interaction::None => { *bg_color = Color::GRAY.into(); } } } if let Ok(offered) = level_up_ui_query.get_single() { let choice_made = if keyboard_input.just_pressed(KeyCode::Digit1) && offered.choices.len() > 0 { Some(offered.choices[0].clone()) } else if keyboard_input.just_pressed(KeyCode::Digit2) && offered.choices.len() > 1 { Some(offered.choices[1].clone()) } else if keyboard_input.just_pressed(KeyCode::Digit3) && offered.choices.len() > 2 { Some(offered.choices[2].clone()) } else { None }; if let Some(chosen_card) = choice_made { sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); upgrade_chosen_event.send(UpgradeChosenEvent(chosen_card)); next_app_state.set(AppState::InGame); } } }

fn apply_chosen_upgrade(
    mut events: EventReader<UpgradeChosenEvent>,
    mut player_query: Query<(&mut Survivor, &mut SanityStrain, &mut Health, &mut CircleOfWarding, &mut SwarmOfNightmares)>,
    item_library: Res<ItemLibrary>,
    mut item_collected_writer: EventWriter<ItemCollectedEvent>,
    skill_library: Res<crate::skills::SkillLibrary>,
) {
    for event in events.read() {
        let Ok((mut player_stats, mut sanity_strain, mut health_stats, mut circle_aura, mut nightmare_swarm)) = player_query.get_single_mut() else { continue; };
        match &event.0.upgrade_type {
            UpgradeType::SurvivorSpeed(percentage) => { player_stats.speed *= 1.0 + (*percentage as f32 / 100.0); }
            UpgradeType::MaxEndurance(amount) => { player_stats.max_health += *amount; health_stats.0 += *amount; health_stats.0 = health_stats.0.min(player_stats.max_health); }

            UpgradeType::IncreaseAutoWeaponDamage(bonus_amount) => { player_stats.auto_weapon_damage_bonus += *bonus_amount; }
            UpgradeType::IncreaseAutoWeaponFireRate(percentage) => {
                let increase_factor = *percentage as f32 / 100.0;
                sanity_strain.base_fire_rate_secs /= 1.0 + increase_factor;
                sanity_strain.base_fire_rate_secs = sanity_strain.base_fire_rate_secs.max(0.05);
            }
            UpgradeType::IncreaseAutoWeaponProjectileSpeed(percentage_increase) => { player_stats.auto_weapon_projectile_speed_multiplier *= 1.0 + (*percentage_increase as f32 / 100.0); }
            UpgradeType::IncreaseAutoWeaponPiercing(amount) => { player_stats.auto_weapon_piercing_bonus += *amount; }
            UpgradeType::IncreaseAutoWeaponProjectiles(amount) => { player_stats.auto_weapon_additional_projectiles_bonus += *amount; }

            UpgradeType::EchoesGainMultiplier(percentage) => { player_stats.xp_gain_multiplier *= 1.0 + (*percentage as f32 / 100.0); }
            UpgradeType::SoulAttractionRadius(percentage) => { player_stats.pickup_radius_multiplier *= 1.0 + (*percentage as f32 / 100.0); }

            UpgradeType::InscribeCircleOfWarding => { if !circle_aura.is_active { circle_aura.is_active = true; } else { circle_aura.base_damage_per_tick += 1; circle_aura.current_radius *= 1.1; }}
            UpgradeType::IncreaseCircleRadius(percentage) => { if circle_aura.is_active { circle_aura.current_radius *= 1.0 + (*percentage as f32 / 100.0); }}
            UpgradeType::IncreaseCircleDamage(amount) => { if circle_aura.is_active { circle_aura.base_damage_per_tick += *amount; }}
            UpgradeType::DecreaseCircleTickRate(percentage) => { if circle_aura.is_active { let reduction_factor = *percentage as f32 / 100.0; let current_tick_duration = circle_aura.damage_tick_timer.duration().as_secs_f32(); let new_tick_duration = (current_tick_duration * (1.0 - reduction_factor)).max(0.1); circle_aura.damage_tick_timer.set_duration(std::time::Duration::from_secs_f32(new_tick_duration)); } }
            UpgradeType::EnduranceRegeneration(amount) => { player_stats.health_regen_rate += *amount; }
            UpgradeType::ManifestSwarmOfNightmares => { if !nightmare_swarm.is_active { nightmare_swarm.is_active = true; nightmare_swarm.num_larvae = nightmare_swarm.num_larvae.max(2); } else { nightmare_swarm.num_larvae += 1; nightmare_swarm.damage_per_hit += 1; }}
            UpgradeType::IncreaseNightmareCount(count) => { if nightmare_swarm.is_active { nightmare_swarm.num_larvae += *count; }}
            UpgradeType::IncreaseNightmareDamage(damage) => { if nightmare_swarm.is_active { nightmare_swarm.damage_per_hit += *damage; }}
            UpgradeType::IncreaseNightmareRadius(radius_increase) => { if nightmare_swarm.is_active { nightmare_swarm.orbit_radius += *radius_increase; }}
            UpgradeType::IncreaseNightmareRotationSpeed(speed_increase) => { if nightmare_swarm.is_active { nightmare_swarm.rotation_speed += *speed_increase; }}
            UpgradeType::IncreaseSkillDamage { slot_index, amount } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.flat_damage_bonus += *amount; skill_instance.current_level += 1; } }
            UpgradeType::GrantRandomRelic => { if !item_library.items.is_empty() { let mut rng = rand::thread_rng(); if let Some(random_item_def) = item_library.items.choose(&mut rng) { item_collected_writer.send(ItemCollectedEvent(random_item_def.id)); } } }
            UpgradeType::GrantSkill(skill_id_to_grant) => { let already_has_skill = player_stats.equipped_skills.iter().any(|s| s.definition_id == *skill_id_to_grant); if !already_has_skill { if player_stats.equipped_skills.len() < 5 { if let Some(skill_def) = skill_library.get_skill_definition(*skill_id_to_grant) { player_stats.equipped_skills.push(ActiveSkillInstance::new(*skill_id_to_grant, skill_def.base_glyph_slots)); } } } }
            UpgradeType::ReduceSkillCooldown { slot_index, percent_reduction } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.cooldown_multiplier *= 1.0 - percent_reduction; skill_instance.cooldown_multiplier = skill_instance.cooldown_multiplier.max(0.1); skill_instance.current_level +=1; } }
            UpgradeType::IncreaseSkillAoERadius { slot_index, percent_increase } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.aoe_radius_multiplier *= 1.0 + percent_increase; skill_instance.current_level +=1; } }
        }
    }
}
fn setup_game_over_ui(mut commands: Commands, game_state: Res<GameState>, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0), ..default() }, ..default() }, GameOverUI, )).with_children(|parent| { parent.spawn( TextBundle::from_section( "Consumed by Madness!", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 80.0, color: Color::RED, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( format!("Score: {}", game_state.score), TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 50.0, color: Color::WHITE, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( "Succumb Again? (R)", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 40.0, color: Color::rgba(0.8,0.8,0.8,1.0), }, ).with_text_justify(JustifyText::Center) ); }); }
fn game_over_input_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>, game_state: ResMut<GameState>, horror_spawn_timer: ResMut<HorrorSpawnTimer>, max_horrors: ResMut<MaxHorrors>, player_entity_query: Query<Entity, With<Survivor>>,) { if keyboard_input.just_pressed(KeyCode::KeyR) { for entity in player_entity_query.iter() { commands.entity(entity).despawn_recursive(); } reset_for_new_game_session(game_state, horror_spawn_timer, max_horrors); next_app_state.set(AppState::MainMenu); } }

fn cleanup_session_entities(
    mut commands: Commands,
    projectiles_query: Query<Entity, With<AutomaticProjectile>>,
    orbs_query: Query<Entity, With<EchoingSoul>>,
    skill_projectiles_query: Query<Entity, With<crate::skills::SkillProjectile>>,
    skill_aoe_query: Query<Entity, With<crate::skills::ActiveSkillAoEEffect>>,
) {
    for entity in projectiles_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in orbs_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_projectiles_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_aoe_query.iter() { commands.entity(entity).despawn_recursive(); }
}