// src/game.rs
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::{
    horror::{HorrorSpawnTimer, MaxHorrors},
    echoing_soul::{EchoingSoul, EchoingSoulPlugin},
    survivor::{Survivor, SanityStrain, MAX_ACTIVE_SKILLS},
    components::Health,
    upgrades::{UpgradePlugin, UpgradePool, OfferedUpgrades, UpgradeCard, UpgradeType, SurvivorUpgradeContext},
    weapons::{CircleOfWarding, SwarmOfNightmares},
    audio::{PlaySoundEvent, SoundEffect},
    debug_menu::DebugMenuPlugin,
    items::{ItemId, ItemLibrary, AutomaticWeaponId, AutomaticWeaponLibrary},
    skills::{ActiveSkillInstance, SkillLibrary as SkillsSkillLibrary}, // Removed SkillDefinition
    automatic_projectiles::AutomaticProjectile, // This should be pub in automatic_projectiles.rs
};

pub const SCREEN_WIDTH: f32 = 1280.0;
pub const SCREEN_HEIGHT: f32 = 720.0;
const INITIAL_MAX_HORRORS: u32 = 20;
const INITIAL_SPAWN_INTERVAL_SECONDS: f32 = 2.0;
const DIFFICULTY_INCREASE_INTERVAL_SECONDS: f32 = 30.0;
const MAX_HORRORS_BASE_INCREMENT_PER_CYCLE: u32 = 10;
const MAX_HORRORS_INCREMENT_PER_LEVEL: u32 = 2;
const SPAWN_INTERVAL_DECREMENT_FACTOR_PER_CYCLE: f32 = 0.9;
const SPAWN_INTERVAL_DECREMENT_FACTOR_PER_LEVEL: f32 = 0.98;
const MIN_SPAWN_INTERVAL_SECONDS: f32 = 0.3;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default] MainMenu,
    InGame,
    LevelUp,
    GameOver,
    DebugUpgradeMenu,
}

#[derive(Resource, Default)]
struct PreviousGameState(Option<AppState>);

#[derive(Resource)]
pub struct GameConfig { pub width: f32, pub height: f32, pub spawn_area_padding: f32, }
impl Default for GameConfig { fn default() -> Self { Self { width: SCREEN_WIDTH, height: SCREEN_HEIGHT, spawn_area_padding: 50.0 } } }
pub struct GamePlugin;

#[derive(Resource, Default)]
pub struct GameState {
    pub score: u32,
    pub cycle_number: u32,
    pub horror_count: u32,
    pub game_over_timer: Timer,
    pub game_timer: Timer,
    pub difficulty_timer: Timer,
    pub current_difficulty_multiplier: f32,
}
#[derive(Event)] pub struct UpgradeChosenEvent(pub UpgradeCard);
#[derive(Event)] pub struct ItemCollectedEvent(pub ItemId);

#[derive(Component)] struct MainMenuUI;
#[derive(Component)] struct LevelUpUI;
#[derive(Component)] struct UpgradeButton(UpgradeCard);
#[derive(Component)] struct GameOverUI;
#[derive(Component)] struct InGameUI;
#[derive(Component)] struct EnduranceText;
#[derive(Component)] struct InsightText;
#[derive(Component)] struct EchoesText;
#[derive(Component)] struct ScoreText;
#[derive(Component)] struct TimerText;
#[derive(Component)] struct CycleText;


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
    game_state.current_difficulty_multiplier = 1.0;
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
            .add_plugins((UpgradePlugin, DebugMenuPlugin)) .init_state::<AppState>()
            .init_resource::<GameConfig>() .init_resource::<GameState>()
            .init_resource::<PreviousGameState>()
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
            .add_systems(OnExit(AppState::DebugUpgradeMenu), (on_enter_ingame_state_actions, log_exiting_debug_menu_state));

            app.add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
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
        }
    }
}

fn despawn_ui_by_marker<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) { for entity in query.iter() { commands.entity(entity).despawn_recursive(); } }
fn setup_main_menu_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(20.0), ..default() }, ..default() }, MainMenuUI, )).with_children(|parent| { parent.spawn( TextBundle::from_section( "Eldritch Hero", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 70.0, color: Color::WHITE, }, ).with_text_justify(JustifyText::Center) ); parent.spawn( TextBundle::from_section( "Embrace the Madness (SPACE)", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 40.0, color: Color::rgba(0.8, 0.8, 0.8, 1.0), }, ).with_text_justify(JustifyText::Center) ); }); }
fn main_menu_input_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut next_app_state: ResMut<NextState<AppState>>, game_state: ResMut<GameState>, horror_spawn_timer: ResMut<HorrorSpawnTimer>, max_horrors: ResMut<MaxHorrors>, player_entity_query: Query<Entity, With<Survivor>>,) { if keyboard_input.just_pressed(KeyCode::Space) { for entity in player_entity_query.iter() { commands.entity(entity).despawn_recursive(); } reset_for_new_game_session(game_state, horror_spawn_timer, max_horrors); next_app_state.set(AppState::InGame); } }
fn setup_ingame_ui(mut commands: Commands, asset_server: Res<AssetServer>) { commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, justify_content: JustifyContent::SpaceBetween, padding: UiRect::all(Val::Px(10.0)), position_type: PositionType::Absolute, ..default() }, z_index: ZIndex::Global(1), ..default() }, InGameUI, )).with_children(|parent| { parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceAround, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(5.0)), ..default() }, background_color: Color::rgba(0.0, 0.0, 0.0, 0.3).into(), ..default() }).with_children(|top_bar| { top_bar.spawn((TextBundle::from_section( "Endurance: 100", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::GREEN, }, ), EnduranceText)); top_bar.spawn((TextBundle::from_section( "Insight: 1", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::CYAN, }, ), InsightText)); top_bar.spawn((TextBundle::from_section( "Echoes: 0/100", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::YELLOW, }, ), EchoesText)); top_bar.spawn((TextBundle::from_section( "Cycle: 1", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::ORANGE_RED, }, ), CycleText)); }); parent.spawn(NodeBundle { style: Style { width: Val::Percent(100.0), justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::FlexEnd, padding: UiRect::all(Val::Px(5.0)), ..default() }, ..default() }).with_children(|bottom_bar| { bottom_bar.spawn((TextBundle::from_section( "Score: 0", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), ScoreText)); bottom_bar.spawn((TextBundle::from_section( "Time: 00:00", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 20.0, color: Color::WHITE, }, ), TimerText)); }); }); }
fn update_game_timer(mut game_state: ResMut<GameState>, time: Res<Time>) { if !game_state.game_timer.paused() { game_state.game_timer.tick(time.delta()); } }

fn difficulty_scaling_system(
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    mut horror_spawn_timer: ResMut<HorrorSpawnTimer>,
    mut max_horrors: ResMut<MaxHorrors>,
    player_query: Query<&Survivor>,
) {
    if game_state.difficulty_timer.paused() { return; }
    game_state.difficulty_timer.tick(time.delta());

    let player_level = if let Ok(player) = player_query.get_single() {
        player.level
    } else {
        1
    };

    if game_state.difficulty_timer.just_finished() {
        game_state.cycle_number += 1;
    }

    let time_based_factor = 1.0 + (game_state.cycle_number as f32 - 1.0) * 0.1;
    let level_based_factor = 1.0 + (player_level as f32 - 1.0) * 0.05;
    game_state.current_difficulty_multiplier = (time_based_factor + level_based_factor - 1.0).max(1.0);

    let base_max_horrors_from_cycle = INITIAL_MAX_HORRORS + (game_state.cycle_number -1) * MAX_HORRORS_BASE_INCREMENT_PER_CYCLE;
    let max_horrors_from_level = (player_level -1) * MAX_HORRORS_INCREMENT_PER_LEVEL;
    max_horrors.0 = (base_max_horrors_from_cycle + max_horrors_from_level).min(300);

    let mut new_spawn_interval = INITIAL_SPAWN_INTERVAL_SECONDS;
    for _ in 1..game_state.cycle_number {
        new_spawn_interval *= SPAWN_INTERVAL_DECREMENT_FACTOR_PER_CYCLE;
    }
    for _ in 1..player_level {
        new_spawn_interval *= SPAWN_INTERVAL_DECREMENT_FACTOR_PER_LEVEL;
    }
    new_spawn_interval = new_spawn_interval.max(MIN_SPAWN_INTERVAL_SECONDS);

    if horror_spawn_timer.timer.duration().as_secs_f32() != new_spawn_interval {
        horror_spawn_timer.timer.set_duration(std::time::Duration::from_secs_f32(new_spawn_interval));
    }
}

fn update_ingame_ui(player_query: Query<(&Survivor, &Health)>, game_state: Res<GameState>, mut ui_texts: ParamSet< ( Query<&mut Text, With<EnduranceText>>, Query<&mut Text, With<InsightText>>, Query<&mut Text, With<EchoesText>>, Query<&mut Text, With<ScoreText>>, Query<&mut Text, With<TimerText>>, Query<&mut Text, With<CycleText>>, )>,) { if let Ok((player_stats, player_health)) = player_query.get_single() { if let Ok(mut text) = ui_texts.p0().get_single_mut() { text.sections[0].value = format!("Endurance: {}/{}", player_health.0, player_stats.max_health); if player_health.0 < player_stats.max_health / 3 { text.sections[0].style.color = Color::RED; } else if player_health.0 < player_stats.max_health * 2 / 3 { text.sections[0].style.color = Color::YELLOW; } else { text.sections[0].style.color = Color::GREEN; } } if let Ok(mut text) = ui_texts.p1().get_single_mut() { text.sections[0].value = format!("Insight: {}", player_stats.level); } if let Ok(mut text) = ui_texts.p2().get_single_mut() { text.sections[0].value = format!("Echoes: {}/{}", player_stats.current_level_xp, player_stats.experience_to_next_level()); } } else { if let Ok(mut text) = ui_texts.p0().get_single_mut() { text.sections[0].value = "Endurance: --/--".to_string(); } if let Ok(mut text) = ui_texts.p1().get_single_mut() { text.sections[0].value = "Insight: --".to_string(); } if let Ok(mut text) = ui_texts.p2().get_single_mut() { text.sections[0].value = "Echoes: --/--".to_string(); } } if let Ok(mut text) = ui_texts.p3().get_single_mut() { text.sections[0].value = format!("Score: {}", game_state.score); } if let Ok(mut text) = ui_texts.p4().get_single_mut() { let elapsed_seconds = game_state.game_timer.elapsed().as_secs(); let minutes = elapsed_seconds / 60; let seconds = elapsed_seconds % 60; text.sections[0].value = format!("Time: {:02}:{:02}", minutes, seconds); } if let Ok(mut text) = ui_texts.p5().get_single_mut() { text.sections[0].value = format!("Cycle: {}", game_state.cycle_number); } }

fn setup_level_up_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<(&Survivor, Option<&CircleOfWarding>, Option<&SwarmOfNightmares>)>,
    upgrade_pool: Res<UpgradePool>,
) {
    let (player_stats, opt_circle_aura, opt_nightmare_swarm) = if let Ok(query_result) = player_query.get_single() {
        query_result
    } else {
        return;
    };

    let player_level = player_stats.level;

    let context = SurvivorUpgradeContext {
        is_circle_of_warding_active: opt_circle_aura.map_or(false, |aura| aura.is_active),
        is_swarm_of_nightmares_active: opt_nightmare_swarm.map_or(false, |swarm| swarm.is_active),
        equipped_skills: &player_stats.equipped_skills,
        current_weapon_id: player_stats.equipped_weapon_id,
    };

    let current_offered_upgrades = OfferedUpgrades { choices: upgrade_pool.get_random_upgrades(3, &context) };

    commands.spawn(( NodeBundle { style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: Val::Px(30.0), ..default() }, background_color: Color::rgba(0.1, 0.1, 0.2, 0.9).into(), z_index: ZIndex::Global(10), ..default() }, LevelUpUI, current_offered_upgrades.clone(), )).with_children(|parent| {
        parent.spawn( TextBundle::from_section( format!("Revelation! Insight: {}", player_level), TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 50.0, color: Color::GOLD, }, ).with_style(Style { margin: UiRect::bottom(Val::Px(20.0)), ..default()}) );
        for (index, card) in current_offered_upgrades.choices.iter().enumerate() {
            parent.spawn(( ButtonBundle { style: Style { width: Val::Px(450.0), min_height: Val::Px(100.0), padding: UiRect::all(Val::Px(10.0)), justify_content: JustifyContent::Center, align_items: AlignItems::FlexStart, flex_direction: FlexDirection::Column, border: UiRect::all(Val::Px(2.0)), margin: UiRect::bottom(Val::Px(10.0)), ..default() }, border_color: BorderColor(Color::DARK_GRAY), background_color: Color::GRAY.into(), ..default() }, UpgradeButton(card.clone()), Name::new(format!("Upgrade Button {}", index + 1)), )).with_children(|button_parent| {
                button_parent.spawn(TextBundle::from_section( &card.name, TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 22.0, color: Color::WHITE, }, ).with_style(Style { margin: UiRect::bottom(Val::Px(5.0)), ..default() }));
                button_parent.spawn(TextBundle::from_section( &card.description, TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 16.0, color: Color::rgb(0.9, 0.9, 0.9), }, ).with_style(Style { max_width: Val::Px(420.0), ..default()}));
            });
        }
        if current_offered_upgrades.choices.is_empty() {
             parent.spawn(TextBundle::from_section( "No eligible upgrades available at this time.", TextStyle { font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 22.0, color: Color::YELLOW, }, ));
        }
    });
}

fn handle_upgrade_choice_interaction(mut interaction_query: Query< (&Interaction, &UpgradeButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>), >, mut upgrade_chosen_event: EventWriter<UpgradeChosenEvent>, mut next_app_state: ResMut<NextState<AppState>>, keyboard_input: Res<ButtonInput<KeyCode>>, level_up_ui_query: Query<&OfferedUpgrades, With<LevelUpUI>>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (interaction, upgrade_button_data, mut bg_color) in interaction_query.iter_mut() { match *interaction { Interaction::Pressed => { sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); upgrade_chosen_event.send(UpgradeChosenEvent(upgrade_button_data.0.clone())); next_app_state.set(AppState::InGame); return; } Interaction::Hovered => { *bg_color = Color::DARK_GREEN.into(); } Interaction::None => { *bg_color = Color::GRAY.into(); } } } if let Ok(offered) = level_up_ui_query.get_single() { let choice_made = if keyboard_input.just_pressed(KeyCode::Digit1) && offered.choices.len() > 0 { Some(offered.choices[0].clone()) } else if keyboard_input.just_pressed(KeyCode::Digit2) && offered.choices.len() > 1 { Some(offered.choices[1].clone()) } else if keyboard_input.just_pressed(KeyCode::Digit3) && offered.choices.len() > 2 { Some(offered.choices[2].clone()) } else { None }; if let Some(chosen_card) = choice_made { sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); upgrade_chosen_event.send(UpgradeChosenEvent(chosen_card)); next_app_state.set(AppState::InGame); } } }

fn apply_chosen_upgrade(
    mut events: EventReader<UpgradeChosenEvent>,
    mut player_query: Query<(&mut Survivor, &mut SanityStrain, &mut Health, &mut CircleOfWarding, &mut SwarmOfNightmares)>,
    item_library: Res<ItemLibrary>,
    mut item_collected_writer: EventWriter<ItemCollectedEvent>,
    skill_library: Res<SkillsSkillLibrary>,
) {
    for event in events.read() {
        let Ok((mut player_stats, mut sanity_strain, mut health_stats, mut circle_aura, mut nightmare_swarm)) = player_query.get_single_mut() else { continue; };
        match &event.0.upgrade_type {
            UpgradeType::SurvivorSpeed(percentage) => { player_stats.speed *= 1.0 + (*percentage as f32 / 100.0); }
            UpgradeType::MaxEndurance(amount) => { player_stats.max_health += *amount; health_stats.0 += *amount; health_stats.0 = health_stats.0.min(player_stats.max_health); }
            UpgradeType::EnduranceRegeneration(amount) => { player_stats.health_regen_rate += *amount; }
            UpgradeType::EchoesGainMultiplier(percentage) => { player_stats.xp_gain_multiplier *= 1.0 + (*percentage as f32 / 100.0); }
            UpgradeType::SoulAttractionRadius(percentage) => { player_stats.pickup_radius_multiplier *= 1.0 + (*percentage as f32 / 100.0); }

            UpgradeType::IncreaseAutoWeaponDamage(bonus_amount) => { player_stats.auto_weapon_damage_bonus += *bonus_amount; }
            UpgradeType::IncreaseAutoWeaponFireRate(percentage) => {
                let increase_factor = *percentage as f32 / 100.0;
                sanity_strain.base_fire_rate_secs /= 1.0 + increase_factor;
                sanity_strain.base_fire_rate_secs = sanity_strain.base_fire_rate_secs.max(0.05);
            }
            UpgradeType::IncreaseAutoWeaponProjectileSpeed(percentage_increase) => {
                player_stats.auto_weapon_projectile_speed_multiplier *= 1.0 + (*percentage_increase as f32 / 100.0);
            }
            UpgradeType::IncreaseAutoWeaponPiercing(amount) => { player_stats.auto_weapon_piercing_bonus += *amount; }
            UpgradeType::IncreaseAutoWeaponProjectiles(amount) => { player_stats.auto_weapon_additional_projectiles_bonus += *amount; }
            UpgradeType::IncreaseAutoWeaponChains(amount) => { player_stats.auto_weapon_chain_bonus += *amount; }
            UpgradeType::IncreaseAutoWeaponChainRangePercent(percentage) => {
                player_stats.auto_weapon_chain_range_multiplier *= 1.0 + (*percentage as f32 / 100.0);
            }

            UpgradeType::InscribeCircleOfWarding => { if !circle_aura.is_active { circle_aura.is_active = true; } else { circle_aura.base_damage_per_tick += 1; circle_aura.current_radius *= 1.1; }}
            UpgradeType::IncreaseCircleRadius(percentage) => { if circle_aura.is_active { circle_aura.current_radius *= 1.0 + (*percentage as f32 / 100.0); }}
            UpgradeType::IncreaseCircleDamage(amount) => { if circle_aura.is_active { circle_aura.base_damage_per_tick += *amount; }}
            UpgradeType::DecreaseCircleTickRate(percentage) => { if circle_aura.is_active { let reduction_factor = *percentage as f32 / 100.0; let current_tick_duration = circle_aura.damage_tick_timer.duration().as_secs_f32(); let new_tick_duration = (current_tick_duration * (1.0 - reduction_factor)).max(0.1); circle_aura.damage_tick_timer.set_duration(std::time::Duration::from_secs_f32(new_tick_duration)); } }

            UpgradeType::ManifestSwarmOfNightmares => { if !nightmare_swarm.is_active { nightmare_swarm.is_active = true; nightmare_swarm.num_larvae = nightmare_swarm.num_larvae.max(2); } else { nightmare_swarm.num_larvae += 1; nightmare_swarm.damage_per_hit += 1; }}
            UpgradeType::IncreaseNightmareCount(count) => { if nightmare_swarm.is_active { nightmare_swarm.num_larvae += *count; }}
            UpgradeType::IncreaseNightmareDamage(damage) => { if nightmare_swarm.is_active { nightmare_swarm.damage_per_hit += *damage; }}
            UpgradeType::IncreaseNightmareRadius(radius_increase) => { if nightmare_swarm.is_active { nightmare_swarm.orbit_radius += *radius_increase; }}
            UpgradeType::IncreaseNightmareRotationSpeed(speed_increase) => { if nightmare_swarm.is_active { nightmare_swarm.rotation_speed += *speed_increase; }}

            UpgradeType::IncreaseSkillDamage { slot_index, amount } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.flat_damage_bonus += *amount; skill_instance.current_level += 1; } }
            UpgradeType::ReduceSkillCooldown { slot_index, percent_reduction } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.cooldown_multiplier *= 1.0 - percent_reduction; skill_instance.cooldown_multiplier = skill_instance.cooldown_multiplier.max(0.1); skill_instance.current_level +=1; } }
            UpgradeType::IncreaseSkillAoERadius { slot_index, percent_increase } => { if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) { skill_instance.aoe_radius_multiplier *= 1.0 + percent_increase; skill_instance.current_level +=1; } }
            UpgradeType::AddSkillImpactAoE { slot_index, radius, damage_fraction } => {
                if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) {
                    skill_instance.impact_aoe_radius = *radius;
                    skill_instance.impact_aoe_damage_fraction = *damage_fraction;
                    skill_instance.current_level += 1;
                }
            }
            UpgradeType::IncreaseSkillPiercing { slot_index, amount: _amount } => { // _amount is prefixed
                 if let Some(skill_instance) = player_stats.equipped_skills.get_mut(*slot_index) {
                    warn!("IncreaseSkillPiercing chosen for slot {}, amount {}. (Effect to be fully implemented for SkillInstance's piercing bonus)", slot_index, _amount);
                    // To make this functional, ActiveSkillInstance would need a `bonus_piercing: u32` field,
                    // and survivor_skill_input_system in skills.rs would need to add this bonus
                    // to the SkillProjectile's piercing_left when spawned.
                    skill_instance.current_level += 1;
                 }
            }

            UpgradeType::GrantRandomRelic => { if !item_library.items.is_empty() { let mut rng = rand::thread_rng(); if let Some(random_item_def) = item_library.items.choose(&mut rng) { item_collected_writer.send(ItemCollectedEvent(random_item_def.id)); } } }
            UpgradeType::GrantSkill(skill_id_to_grant) => {
                let already_has_skill = player_stats.equipped_skills.iter().any(|s| s.definition_id == *skill_id_to_grant);
                if !already_has_skill && player_stats.equipped_skills.len() < MAX_ACTIVE_SKILLS {
                    if let Some(_skill_def) = skill_library.get_skill_definition(*skill_id_to_grant) {
                        player_stats.equipped_skills.push(ActiveSkillInstance::new(*skill_id_to_grant ));
                    }
                }
            }
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
    chain_visual_query: Query<Entity, With<crate::automatic_projectiles::ChainLightningVisual>>,
) {
    for entity in projectiles_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in orbs_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_projectiles_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_aoe_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in chain_visual_query.iter() { commands.entity(entity).despawn_recursive(); }
}