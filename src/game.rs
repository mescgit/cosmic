// src/game.rs
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::{
    horror::{HorrorSpawnTimer, MaxHorrors},
    // echoing_soul::{EchoingSoul, EchoingSoulPlugin}, // Removed
    survivor::{Survivor, SanityStrain},
    components::Health,
    // upgrades::{UpgradePlugin, UpgradePool, OfferedUpgrades, UpgradeCard, UpgradeType}, // Removed
    // weapons::{CircleOfWarding, SwarmOfNightmares}, // Removed
    audio::{PlaySoundEvent, SoundEffect},
    debug_menu::DebugMenuPlugin, // Will be removed later if problematic
    // items::{ItemId, ItemLibrary, AutomaticWeaponId, AutomaticWeaponLibrary}, // Removed
    // skills::{ActiveSkillInstance, SkillLibrary}, // Removed
    // automatic_projectiles::AutomaticProjectile, // Removed
    // glyphs::{GlyphLibrary, GlyphId}, // Removed
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
    // LevelUp, // Removed
    GameOver,
    // DebugUpgradeMenu, // Removed
    // GlyphScreen, // Removed
}

#[derive(Resource, Default)]
struct PreviousGameState(Option<AppState>);

#[derive(Resource)]
pub struct GameConfig { pub width: f32, pub height: f32, pub spawn_area_padding: f32, }
impl Default for GameConfig { fn default() -> Self { Self { width: SCREEN_WIDTH, height: SCREEN_HEIGHT, spawn_area_padding: 50.0 } } }
pub struct GamePlugin;
#[derive(Resource, Default)]
pub struct GameState { pub score: u32, pub cycle_number: u32, pub horror_count: u32, pub game_over_timer: Timer, pub game_timer: Timer, pub difficulty_timer: Timer, }
// #[derive(Event)] pub struct UpgradeChosenEvent(pub UpgradeCard); // Removed
// #[derive(Event)] pub struct ItemCollectedEvent(pub ItemId); // Removed

// --- Components for Glyph Screen UI (Commented out) ---
#[derive(Component)] struct MainMenuUI;
// #[derive(Component)] struct LevelUpUI; // Removed
// #[derive(Component)] struct UpgradeButton(UpgradeCard); // Removed
#[derive(Component)] struct GameOverUI;
#[derive(Component)] struct InGameUI;
// #[derive(Component)] struct GlyphScreenUI; // Removed
#[derive(Component)] struct EnduranceText;
// #[derive(Component)] struct InsightText; // Removed
// #[derive(Component)] struct EchoesText; // Removed
#[derive(Component)] struct ScoreText;
#[derive(Component)] struct TimerText;
#[derive(Component)] struct CycleText;

// #[derive(Component)] // Commented out
// struct GlyphInventoryButton(GlyphId);

// #[derive(Component, Debug, Clone, Copy, PartialEq, Eq)] // Commented out
// enum GlyphSocketTargetType {
//     ActiveSkill,
//     AutomaticWeapon,
// }

// #[derive(Component)] // Commented out
// #[derive(Component)] struct GlyphTargetSlotButton { // Removed
//     target_type: GlyphSocketTargetType, // Removed
//     target_entity_slot_idx: usize, // Removed
//     glyph_slot_idx: usize, // Removed
// } // Removed

// --- Event for Socketing (Commented out) --- // Removed
// #[derive(Event)] // Removed // Removed
// struct SocketGlyphRequestedEvent { // Removed
//     glyph_to_socket: GlyphId, // Removed
//     target_type: GlyphSocketTargetType, // Removed
//     target_entity_slot_idx: usize, // Removed
//     glyph_slot_idx: usize, // Removed
// } // Removed

// --- Resource to track selected glyph (Commented out) --- // Removed
// #[derive(Resource, Default)] // Removed // Removed
// struct SelectedGlyphForSocketing(Option<GlyphId>); // Removed


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
        app // .add_event::<UpgradeChosenEvent>() // Removed
            // .add_event::<ItemCollectedEvent>() // Removed
            // .add_event::<SocketGlyphRequestedEvent>() // Removed
            // .add_plugins((UpgradePlugin, DebugMenuPlugin)) // Removed
            .init_state::<AppState>()
            .init_resource::<GameConfig>() .init_resource::<GameState>()
            .init_resource::<PreviousGameState>()
            // .init_resource::<SelectedGlyphForSocketing>() // Removed
            .insert_resource(HorrorSpawnTimer {timer: Timer::from_seconds(INITIAL_SPAWN_INTERVAL_SECONDS, TimerMode::Repeating)})
            .insert_resource(MaxHorrors(INITIAL_MAX_HORRORS)) // .add_plugins(EchoingSoulPlugin) // Removed

            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu_ui)
            .add_systems(Update, main_menu_input_system.run_if(in_state(AppState::MainMenu)))
            .add_systems(OnExit(AppState::MainMenu), despawn_ui_by_marker::<MainMenuUI>)

            .add_systems(OnEnter(AppState::InGame), (on_enter_ingame_state_actions, setup_ingame_ui,))
            .add_systems(Update, (
                update_ingame_ui,
                update_game_timer,
                difficulty_scaling_system,
                global_key_listener,
                // debug_weapon_switch_system, // Will be removed later
            ).chain().run_if(in_state(AppState::InGame))) // Removed .or_else(in_state(AppState::DebugUpgradeMenu))
            .add_systems(OnExit(AppState::InGame), (cleanup_session_entities, despawn_ui_by_marker::<InGameUI>))

            // Systems for AppState::LevelUp removed
            // .add_systems(OnEnter(AppState::LevelUp), (setup_level_up_ui, on_enter_pause_like_state_actions))
            // .add_systems(Update, handle_upgrade_choice_interaction.run_if(in_state(AppState::LevelUp)))
            // .add_systems(Update, apply_chosen_upgrade.run_if(on_event::<UpgradeChosenEvent>()))
            // .add_systems(OnExit(AppState::LevelUp), (despawn_ui_by_marker::<LevelUpUI>, on_enter_ingame_state_actions))

            // Systems for AppState::DebugUpgradeMenu removed
            // .add_systems(OnEnter(AppState::DebugUpgradeMenu), (on_enter_pause_like_state_actions, log_entering_debug_menu_state))
            // .add_systems(OnExit(AppState::DebugUpgradeMenu), (on_enter_ingame_state_actions, log_exiting_debug_menu_state));

            // --- GlyphScreen systems removed ---

            app.add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
            .add_systems(Update, game_over_input_system.run_if(in_state(AppState::GameOver)))
            .add_systems(OnExit(AppState::GameOver), despawn_ui_by_marker::<GameOverUI>);
    }
}

fn global_key_listener(
    _keyboard_input: Res<ButtonInput<KeyCode>>, // Now unused
    _current_app_state: Res<State<AppState>>, // Now unused
    _next_app_state: ResMut<NextState<AppState>>, // Now unused
    _prev_game_state: ResMut<PreviousGameState>, // Now unused
) {
    // Backquote logic removed as AppState::DebugUpgradeMenu is removed
    // 'G' key logic for GlyphScreen removed
}

// --- End Commented out GlyphScreen functions ---

// fn debug_weapon_switch_system removed

fn despawn_ui_by_marker<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) { for entity in query.iter() { commands.entity(entity).despawn_recursive(); } }
fn setup_main_menu_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0), ..default() }, ..default() }, MainMenuUI, )).with_children(|parent| { parent.spawn( TextBundle::from_section( "Echoes of the Abyss", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 70.0, color: Color::WHITE, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( "Embrace the Madness (SPACE)", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 40.0, color: Color::rgba(0.8, 0.8, 0.8, 1.0), }, ).with_text_justify(JustifyText::Center) ); }); }
fn main_menu_input_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>, game_state: ResMut<GameState>, horror_spawn_timer: ResMut<HorrorSpawnTimer>, max_horrors: ResMut<MaxHorrors>, player_entity_query: Query<Entity, With<Survivor>>,) { if keyboard_input.just_pressed(KeyCode::Space) { for entity in player_entity_query.iter() { commands.entity(entity).despawn_recursive(); } reset_for_new_game_session(game_state, horror_spawn_timer, max_horrors); next_app_state.set(AppState::InGame); } }
fn setup_ingame_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(Val::Px(10.0)), position_type: PositionType::Absolute, ..default() }, z_index: ZIndex::Global(1), ..default() }, InGameUI, )).with_children(|parent| { parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceAround, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(5.0)), ..default() }, background_color: Color::rgba(0.0, 0.0, 0.0, 0.3).into(), ..default() }).with_children(|top_bar| { top_bar.spawn((TextBundle::from_section( "Endurance: 100", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::GREEN, }, ), EnduranceText)); top_bar.spawn((TextBundle::from_section( "Cycle: 1", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::ORANGE_RED, }, ), CycleText)); }); parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::FlexEnd, padding: UiRect::all(Val::Px(5.0)), ..default() }, ..default() }).with_children(|bottom_bar| { bottom_bar.spawn((TextBundle::from_section( "Score: 0", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), ScoreText)); bottom_bar.spawn((TextBundle::from_section( "Time: 00:00", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), TimerText)); }); }); }
fn update_game_timer(mut game_state: ResMut<GameState>, time: Res<Time>) { if !game_state.game_timer.paused() { game_state.game_timer.tick(time.delta()); } }
fn difficulty_scaling_system(time: Res<Time>, mut game_state: ResMut<GameState>, mut horror_spawn_timer: ResMut<HorrorSpawnTimer>, mut max_horrors: ResMut<MaxHorrors>,) { if game_state.difficulty_timer.paused() { return; } game_state.difficulty_timer.tick(time.delta()); if game_state.difficulty_timer.just_finished() { game_state.cycle_number += 1; max_horrors.0 = (INITIAL_MAX_HORRORS + (game_state.cycle_number -1) * MAX_HORRORS_INCREMENT).min(200); let current_duration = horror_spawn_timer.timer.duration().as_secs_f32(); let new_duration = (current_duration * SPAWN_INTERVAL_DECREMENT_FACTOR).max(MIN_SPAWN_INTERVAL_SECONDS); horror_spawn_timer.timer.set_duration(std::time::Duration::from_secs_f32(new_duration)); } }
fn update_ingame_ui(player_query: Query<(&Survivor, &Health)>, game_state: Res<GameState>, mut ui_texts: ParamSet< ( Query<&mut Text, With<EnduranceText>>, Query<&mut Text, With<ScoreText>>, Query<&mut Text, With<TimerText>>, Query<&mut Text, With<CycleText>>, )>,) {
    if let Ok((player_stats, player_health)) = player_query.get_single() {
        if let Ok(mut text) = ui_texts.p0().get_single_mut() {
            text.sections[0].value = format!("Endurance: {}/{}", player_health.0, player_stats.max_health);
            if player_health.0 < player_stats.max_health / 3 { text.sections[0].style.color = Color::RED; }
            else if player_health.0 < player_stats.max_health * 2 / 3 { text.sections[0].style.color = Color::YELLOW; }
            else { text.sections[0].style.color = Color::GREEN; }
        }
        // InsightText and EchoesText update logic removed
    } else {
        if let Ok(mut text) = ui_texts.p0().get_single_mut() { text.sections[0].value = "Endurance: --/--".to_string(); }
        // InsightText and EchoesText fallback update logic removed
    }

    if let Ok(mut text) = ui_texts.p1().get_single_mut() { text.sections[0].value = format!("Score: {}", game_state.score); }
    if let Ok(mut text) = ui_texts.p2().get_single_mut() {
        let elapsed_seconds = game_state.game_timer.elapsed().as_secs();
        let minutes = elapsed_seconds / 60;
        let seconds = elapsed_seconds % 60;
        text.sections[0].value = format!("Time: {:02}:{:02}", minutes, seconds);
    }
    if let Ok(mut text) = ui_texts.p3().get_single_mut() { text.sections[0].value = format!("Cycle: {}", game_state.cycle_number); }
}
// fn setup_level_up_ui removed
// fn handle_upgrade_choice_interaction removed
// fn apply_chosen_upgrade removed

fn setup_game_over_ui(mut commands: Commands, game_state: Res<GameState>, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0), ..default() }, ..default() }, GameOverUI, )).with_children(|parent| { parent.spawn( TextBundle::from_section( "Consumed by Madness!", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 80.0, color: Color::RED, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( format!("Score: {}", game_state.score), TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 50.0, color: Color::WHITE, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( "Succumb Again? (R)", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 40.0, color: Color::rgba(0.8,0.8,0.8,1.0), }, ).with_text_justify(JustifyText::Center) ); }); }
fn game_over_input_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>, game_state: ResMut<GameState>, horror_spawn_timer: ResMut<HorrorSpawnTimer>, max_horrors: ResMut<MaxHorrors>, player_entity_query: Query<Entity, With<Survivor>>,) { if keyboard_input.just_pressed(KeyCode::KeyR) { for entity in player_entity_query.iter() { commands.entity(entity).despawn_recursive(); } reset_for_new_game_session(game_state, horror_spawn_timer, max_horrors); next_app_state.set(AppState::MainMenu); } }

fn cleanup_session_entities(
    mut commands: Commands,
    // projectiles_query: Query<Entity, With<AutomaticProjectile>>, // Removed
    // orbs_query: Query<Entity, With<EchoingSoul>>, // Removed
    // skill_projectiles_query: Query<Entity, With<crate::skills::SkillProjectile>>, // Removed
    // skill_aoe_query: Query<Entity, With<crate::skills::ActiveSkillAoEEffect>>, // Removed
) {
    // for entity in projectiles_query.iter() { commands.entity(entity).despawn_recursive(); } // Removed
    // for entity in orbs_query.iter() { commands.entity(entity).despawn_recursive(); } // Removed
    // for entity in skill_projectiles_query.iter() { commands.entity(entity).despawn_recursive(); } // Removed
    // for entity in skill_aoe_query.iter() { commands.entity(entity).despawn_recursive(); } // Removed
}