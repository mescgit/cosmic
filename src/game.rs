use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::{
    horror::{HorrorSpawnTimer, MaxHorrors}, 
    echoing_soul::{EchoingSoul, EchoingSoulPlugin}, 
    survivor::Survivor, 
    components::Health,
    upgrades::{UpgradePlugin, UpgradePool, OfferedUpgrades, UpgradeCard, UpgradeType},
    weapons::{CircleOfWarding, SwarmOfNightmares}, 
    audio::{PlaySoundEvent, SoundEffect},
    debug_menu::DebugMenuPlugin,
    items::{ItemId, ItemLibrary},
    skills::{ActiveSkillInstance, SkillLibrary as GameSkillLibrary}, 
    ichor_blast::IchorBlast, 
    glyphs::{GlyphLibrary, GlyphId}, 
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
    #[default] 
    MainMenu, 
    InGame, 
    LevelUp, 
    GameOver, 
    DebugUpgradeMenu,
    GlyphSocketingMenu, 
}
#[derive(Resource)]
pub struct GameConfig { pub _width: f32, pub _height: f32, pub _spawn_area_padding: f32, }
impl Default for GameConfig { fn default() -> Self { Self { _width: SCREEN_WIDTH, _height: SCREEN_HEIGHT, _spawn_area_padding: 50.0 } } }
pub struct GamePlugin;
#[derive(Resource, Default)]
pub struct GameState { pub score: u32, pub cycle_number: u32, pub horror_count: u32, pub _game_over_timer: Timer, pub game_timer: Timer, pub difficulty_timer: Timer, } 
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
#[derive(Component)] struct GlyphSocketingMenuUI;
#[derive(Component)] struct GlyphSlotDisplayButton { pub skill_idx: usize, pub slot_idx: usize } 
#[derive(Component)] struct InventoryGlyphDisplayButton { pub glyph_id: GlyphId }
#[derive(Resource, Default)] struct SelectedInventoryGlyph { pub glyph_id: Option<GlyphId> }


const UI_TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const UI_PANEL_BG_COLOR: Color = Color::rgba(0.05, 0.05, 0.07, 0.95);
const UI_SECTION_BG_COLOR: Color = Color::rgba(0.1, 0.1, 0.12, 0.95);
const BUTTON_BG_COLOR: Color = Color::rgb(0.25, 0.25, 0.25);
const BUTTON_HOVER_BG_COLOR: Color = Color::rgb(0.35, 0.35, 0.35);
const BUTTON_PRESSED_BG_COLOR: Color = Color::rgb(0.15, 0.15, 0.15);
const BUTTON_SELECTED_BG_COLOR: Color = Color::rgb(0.45, 0.45, 0.25);


fn reset_for_new_game_session(mut game_state: ResMut<GameState>, mut horror_spawn_timer: ResMut<HorrorSpawnTimer>, mut max_horrors: ResMut<MaxHorrors>,) { game_state.score = 0; game_state.cycle_number = 1; game_state.horror_count = 0; game_state.game_timer = Timer::from_seconds(3600.0, TimerMode::Once); game_state.game_timer.reset(); game_state.game_timer.unpause(); game_state.difficulty_timer = Timer::from_seconds(DIFFICULTY_INCREASE_INTERVAL_SECONDS, TimerMode::Repeating); game_state.difficulty_timer.reset(); horror_spawn_timer.timer.set_duration(std::time::Duration::from_secs_f32(INITIAL_SPAWN_INTERVAL_SECONDS)); horror_spawn_timer.timer.reset(); max_horrors.0 = INITIAL_MAX_HORRORS; } 
fn on_enter_ingame_state_actions(mut game_state: ResMut<GameState>) { if game_state.game_timer.paused() { game_state.game_timer.unpause(); } if game_state.difficulty_timer.paused() { game_state.difficulty_timer.unpause(); } }
fn on_enter_pause_like_state_actions(mut game_state: ResMut<GameState>, _current_app_state: Res<State<AppState>>) { if !game_state.game_timer.paused() { game_state.game_timer.pause(); } if !game_state.difficulty_timer.paused() { game_state.difficulty_timer.pause(); } }
fn log_entering_debug_menu_state() {}
fn log_exiting_debug_menu_state() {}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app .add_event::<UpgradeChosenEvent>() .add_event::<ItemCollectedEvent>()
            .add_plugins((UpgradePlugin, DebugMenuPlugin)) .init_state::<AppState>()
            .init_resource::<GameConfig>() .init_resource::<GameState>()
            .init_resource::<SelectedInventoryGlyph>() 
            .insert_resource(HorrorSpawnTimer {timer: Timer::from_seconds(INITIAL_SPAWN_INTERVAL_SECONDS, TimerMode::Repeating)}) 
            .insert_resource(MaxHorrors(INITIAL_MAX_HORRORS)) .add_plugins(EchoingSoulPlugin) 
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu_ui)
            .add_systems(Update, main_menu_input_system.run_if(in_state(AppState::MainMenu)))
            .add_systems(OnExit(AppState::MainMenu), despawn_ui_by_marker::<MainMenuUI>)
            .add_systems(OnEnter(AppState::InGame), (on_enter_ingame_state_actions, setup_ingame_ui,))
            .add_systems(Update, (update_ingame_ui, update_game_timer, difficulty_scaling_system, global_debug_key_listener,).chain().run_if(in_state(AppState::InGame).or_else(in_state(AppState::DebugUpgradeMenu)).or_else(in_state(AppState::GlyphSocketingMenu))))
            .add_systems(OnExit(AppState::InGame), (cleanup_session_entities, despawn_ui_by_marker::<InGameUI>))
            .add_systems(OnEnter(AppState::LevelUp), (setup_level_up_ui, on_enter_pause_like_state_actions))
            .add_systems(Update, handle_upgrade_choice_interaction.run_if(in_state(AppState::LevelUp)))
            .add_systems(Update, apply_chosen_upgrade.run_if(on_event::<UpgradeChosenEvent>()))
            .add_systems(OnExit(AppState::LevelUp), (despawn_ui_by_marker::<LevelUpUI>, on_enter_ingame_state_actions))
            .add_systems(OnEnter(AppState::DebugUpgradeMenu), (on_enter_pause_like_state_actions, log_entering_debug_menu_state))
            .add_systems(OnExit(AppState::DebugUpgradeMenu), (on_enter_ingame_state_actions, log_exiting_debug_menu_state))
            .add_systems(OnEnter(AppState::GlyphSocketingMenu), (setup_glyph_socketing_menu_ui, on_enter_pause_like_state_actions)) 
            .add_systems(Update, (
                glyph_socketing_menu_input_system,
                glyph_slot_button_interaction_system, 
                inventory_glyph_button_interaction_system, 
            ).chain().run_if(in_state(AppState::GlyphSocketingMenu))) 
            .add_systems(OnExit(AppState::GlyphSocketingMenu), (despawn_ui_by_marker::<GlyphSocketingMenuUI>, on_enter_ingame_state_actions, clear_selected_glyph_on_exit)) 
            .add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
            .add_systems(Update, game_over_input_system.run_if(in_state(AppState::GameOver)))
            .add_systems(OnExit(AppState::GameOver), despawn_ui_by_marker::<GameOverUI>);
    }
}

fn clear_selected_glyph_on_exit(mut selected_glyph: ResMut<SelectedInventoryGlyph>) {
    selected_glyph.glyph_id = None;
}

fn global_debug_key_listener(keyboard_input: Res<ButtonInput<KeyCode>>, current_app_state: Res<State<AppState>>, mut next_app_state: ResMut<NextState<AppState>>,) { 
    if keyboard_input.just_pressed(KeyCode::Backquote) { 
        match current_app_state.get() { 
            AppState::InGame => { next_app_state.set(AppState::DebugUpgradeMenu); } 
            AppState::DebugUpgradeMenu => { next_app_state.set(AppState::InGame); } 
            _ => {} 
        } 
    }
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        match current_app_state.get() {
            AppState::InGame => { next_app_state.set(AppState::GlyphSocketingMenu); }
            AppState::GlyphSocketingMenu => { next_app_state.set(AppState::InGame); }
            _ => {}
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
    mut player_query: Query<(&mut Survivor, &mut crate::survivor::SanityStrain, &mut Health, &mut CircleOfWarding, &mut SwarmOfNightmares)>, 
    item_library: Res<ItemLibrary>, 
    glyph_library: Res<GlyphLibrary>, 
    mut item_collected_writer: EventWriter<ItemCollectedEvent>, 
    skill_library: Res<crate::skills::SkillLibrary>,
) { 
    for event in events.read() { 
        let Ok((mut player_stats, mut sanity_strain, mut health_stats, mut circle_aura, mut nightmare_swarm)) = player_query.get_single_mut() else { continue; }; 
        match &event.0.upgrade_type { 
            UpgradeType::SurvivorSpeed(percentage) => { player_stats.speed *= 1.0 + (*percentage as f32 / 100.0); } 
            UpgradeType::MaxEndurance(amount) => { player_stats.max_health += *amount; health_stats.0 += *amount; health_stats.0 = health_stats.0.min(player_stats.max_health); } 
            UpgradeType::IchorBlastIntensity(bonus_amount) => { player_stats.ichor_blast_damage_bonus += *bonus_amount; } 
            UpgradeType::IchorBlastSpeed(percentage) => { let reduction_factor = *percentage as f32 / 100.0; let new_base_fire_rate_secs = sanity_strain.base_fire_rate_secs * (1.0 - reduction_factor); sanity_strain.base_fire_rate_secs = new_base_fire_rate_secs.max(0.05); let timer_duration_val = sanity_strain.base_fire_rate_secs; sanity_strain.fire_timer.set_duration(std::time::Duration::from_secs_f32(timer_duration_val));} 
            UpgradeType::IchorBlastVelocity(percentage_increase) => { player_stats.ichor_blast_speed_multiplier *= 1.0 + (*percentage_increase as f32 / 100.0); } 
            UpgradeType::IchorBlastPiercing(amount) => { player_stats.ichor_blast_piercing += *amount; } 
            UpgradeType::EchoesGainMultiplier(percentage) => { player_stats.xp_gain_multiplier *= 1.0 + (*percentage as f32 / 100.0); } 
            UpgradeType::SoulAttractionRadius(percentage) => { player_stats.pickup_radius_multiplier *= 1.0 + (*percentage as f32 / 100.0); } 
            UpgradeType::AdditionalIchorBlasts(amount) => { player_stats.additional_ichor_blasts += *amount; } 
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
            UpgradeType::GrantRandomGlyph => { 
                if !glyph_library.glyphs.is_empty() {
                    let mut rng = rand::thread_rng();
                    if let Some(random_glyph_def) = glyph_library.glyphs.choose(&mut rng) {
                        if !player_stats.collected_glyphs.contains(&random_glyph_def.id) {
                             player_stats.collected_glyphs.push(random_glyph_def.id);
                        } 
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
    fragments_query: Query<Entity, With<IchorBlast>>, 
    orbs_query: Query<Entity, With<EchoingSoul>>, 
    skill_projectiles_query: Query<Entity, With<crate::skills::SkillProjectile>>, 
    skill_aoe_query: Query<Entity, With<crate::skills::ActiveSkillAoEEffect>>, 
) {
    for entity in fragments_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in orbs_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_projectiles_query.iter() { commands.entity(entity).despawn_recursive(); }
    for entity in skill_aoe_query.iter() { commands.entity(entity).despawn_recursive(); }
}

// --- Glyph Socketing Menu Systems ---
fn setup_glyph_socketing_menu_ui(
    mut commands: Commands, 
    asset_server: Res<AssetServer>,
    player_query: Query<&Survivor>,
    skill_library: Res<GameSkillLibrary>,
    glyph_library: Res<GlyphLibrary>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.85).into(),
            z_index: ZIndex::Global(20),
            ..default()
        },
        GlyphSocketingMenuUI,
        Name::new("GlyphSocketingMenu"),
    )).with_children(|menu_root| {
        menu_root.spawn(NodeBundle { 
            style: Style {
                width: Val::Percent(80.0),
                max_width: Val::Px(1000.0),
                height: Val::Percent(80.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: UI_PANEL_BG_COLOR.into(),
            border_color: Color::DARK_GRAY.into(),
            ..default()
        }).with_children(|main_panel| {
            main_panel.spawn(TextBundle::from_section(
                "Glyph Socketing",
                TextStyle { font: font.clone(), font_size: 32.0, color: Color::ORANGE_RED },
            ).with_style(Style { margin: UiRect::bottom(Val::Px(20.0)), align_self: AlignSelf::Center, ..default() }));

            main_panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    column_gap: Val::Px(20.0),
                    ..default()
                },
                ..default()
            }).with_children(|content_area| {
                content_area.spawn(NodeBundle {
                    style: Style {
                        flex_basis: Val::Percent(60.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    background_color: UI_SECTION_BG_COLOR.into(),
                    ..default()
                }).with_children(|skills_panel| {
                    skills_panel.spawn(TextBundle::from_section(
                        "Equipped Skills",
                        TextStyle { font: font.clone(), font_size: 20.0, color: UI_TEXT_COLOR },
                    ).with_style(Style { margin: UiRect::bottom(Val::Px(10.0)), ..default() }));

                    if let Ok(player) = player_query.get_single() {
                        for (skill_idx, active_skill) in player.equipped_skills.iter().enumerate() {
                            if let Some(skill_def) = skill_library.get_skill_definition(active_skill.definition_id) {
                                skills_panel.spawn(NodeBundle { 
                                    style: Style { flex_direction: FlexDirection::Column, margin: UiRect::bottom(Val::Px(15.0)), ..default() }, ..default()
                                }).with_children(|skill_entry| {
                                    skill_entry.spawn(TextBundle::from_section(
                                        format!("Skill {}: {} (Lvl {})", skill_idx + 1, skill_def.name, active_skill.current_level),
                                        TextStyle { font: font.clone(), font_size: 18.0, color: Color::CYAN },
                                    ));
                                    skill_entry.spawn(NodeBundle {
                                        style: Style { flex_direction: FlexDirection::Row, margin: UiRect::top(Val::Px(5.0)), column_gap: Val::Px(10.0), ..default()}, ..default()
                                    }).with_children(|slots_container| {
                                        for s_idx in 0..skill_def.base_glyph_slots { 
                                            if let Some(glyph_id_option) = active_skill.equipped_glyphs.get(s_idx as usize) {
                                                if let Some(glyph_id) = glyph_id_option { 
                                                    let glyph_name = glyph_library.get_glyph_definition(*glyph_id).map_or("Unknown Glyph".to_string(), |g| g.name.clone());
                                                    slots_container.spawn(NodeBundle {
                                                        style: Style { padding: UiRect::all(Val::Px(5.0)), ..default() },
                                                        background_color: Color::SEA_GREEN.into(), 
                                                        ..default()
                                                    }).with_children(|p| {
                                                        p.spawn(TextBundle::from_section(
                                                            format!("[Slot {}: {}]", s_idx + 1, glyph_name),
                                                            TextStyle { font: font.clone(), font_size: 14.0, color: UI_TEXT_COLOR },
                                                        ));
                                                    });
                                                } else { 
                                                    slots_container.spawn((
                                                        ButtonBundle {
                                                            style: Style { padding: UiRect::all(Val::Px(5.0)), ..default()},
                                                            background_color: BUTTON_BG_COLOR.into(),
                                                            ..default()
                                                        },
                                                        GlyphSlotDisplayButton { skill_idx, slot_idx: s_idx as usize },
                                                    )).with_children(|button_node| {
                                                        button_node.spawn(TextBundle::from_section(
                                                            format!("[Slot {}: Empty]", s_idx + 1),
                                                            TextStyle { font: font.clone(), font_size: 14.0, color: UI_TEXT_COLOR },
                                                        ));
                                                    });
                                                }
                                            } else { 
                                                 slots_container.spawn(TextBundle::from_section(
                                                    format!("[Slot {}: Error]", s_idx + 1),
                                                    TextStyle { font: font.clone(), font_size: 14.0, color: Color::RED },
                                                ));
                                            }
                                        }
                                        if skill_def.base_glyph_slots == 0 {
                                            slots_container.spawn(TextBundle::from_section(
                                                "(No Glyph Slots)", TextStyle { font: font.clone(), font_size: 14.0, color: Color::GRAY },
                                            ));
                                        }
                                    });
                                });
                            }
                        }
                    } else {
                        skills_panel.spawn(TextBundle::from_section("No player data.", TextStyle { font: font.clone(), font_size: 16.0, color: UI_TEXT_COLOR }));
                    }
                });

                content_area.spawn(NodeBundle {
                    style: Style {
                        flex_basis: Val::Percent(40.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        ..default()
                    },
                    background_color: UI_SECTION_BG_COLOR.into(),
                    ..default()
                }).with_children(|inventory_panel| {
                    inventory_panel.spawn(TextBundle::from_section(
                        "Collected Glyphs",
                        TextStyle { font: font.clone(), font_size: 20.0, color: UI_TEXT_COLOR },
                    ).with_style(Style { margin: UiRect::bottom(Val::Px(10.0)), ..default() }));

                    if let Ok(player) = player_query.get_single() {
                        if player.collected_glyphs.is_empty() {
                            inventory_panel.spawn(TextBundle::from_section("No glyphs collected.", TextStyle { font: font.clone(), font_size: 16.0, color: UI_TEXT_COLOR }));
                        } else {
                            for glyph_id_ref in player.collected_glyphs.iter() { 
                                if let Some(glyph_def) = glyph_library.get_glyph_definition(*glyph_id_ref) { 
                                    inventory_panel.spawn((
                                        ButtonBundle {
                                            style: Style {
                                                padding: UiRect::all(Val::Px(5.0)),
                                                margin: UiRect::bottom(Val::Px(3.0)),
                                                width: Val::Percent(100.0),
                                                ..default()
                                            },
                                            background_color: BUTTON_BG_COLOR.into(),
                                            ..default()
                                        },
                                        InventoryGlyphDisplayButton { glyph_id: *glyph_id_ref }, 
                                    )).with_children(|button_node| {
                                        button_node.spawn(TextBundle::from_section(
                                            format!("- {}", glyph_def.name), 
                                            TextStyle { font: font.clone(), font_size: 14.0, color: Color::YELLOW_GREEN },
                                        ));
                                    });
                                }
                            }
                        }
                    } else {
                         inventory_panel.spawn(TextBundle::from_section("No player data.", TextStyle { font: font.clone(), font_size: 16.0, color: UI_TEXT_COLOR }));
                    }
                });
            });
            main_panel.spawn(TextBundle::from_section(
                "Press 'G' or 'Esc' to Close",
                TextStyle { font: font.clone(), font_size: 18.0, color: UI_TEXT_COLOR },
            ).with_style(Style { margin: UiRect::top(Val::Px(20.0)), align_self: AlignSelf::Center, ..default() }));
        });
    });
}

fn glyph_socketing_menu_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyG) || keyboard_input.just_pressed(KeyCode::Escape) {
        next_app_state.set(AppState::InGame);
    }
}

fn glyph_slot_button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &GlyphSlotDisplayButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    selected_glyph: Res<SelectedInventoryGlyph>, 
    skill_library: Res<GameSkillLibrary>,
    glyph_library: Res<GlyphLibrary>,
    player_query: Query<&Survivor>,
) {
    for (interaction, button_data, mut bg_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BUTTON_PRESSED_BG_COLOR.into();
                if let Some(selected_glyph_id) = selected_glyph.glyph_id {
                    if let Ok(player) = player_query.get_single() {
                        if let Some(skill_instance) = player.equipped_skills.get(button_data.skill_idx) {
                            if let Some(skill_def) = skill_library.get_skill_definition(skill_instance.definition_id) {
                                if let Some(glyph_def) = glyph_library.get_glyph_definition(selected_glyph_id) {
                                    println!("Attempting to socket glyph '{}' into skill '{}' slot {}", glyph_def.name, skill_def.name, button_data.slot_idx + 1);
                                }
                            }
                        }
                    }
                } else {
                    println!("Clicked skill slot {} on skill {}, but no glyph is selected.", button_data.slot_idx + 1, button_data.skill_idx + 1);
                }
            }
            Interaction::Hovered => { *bg_color = BUTTON_HOVER_BG_COLOR.into(); }
            Interaction::None => { *bg_color = BUTTON_BG_COLOR.into(); }
        }
    }
}

fn inventory_glyph_button_interaction_system(
    // Query for buttons whose interaction state *just changed*
    mut changed_interaction_query: Query<
        (Entity, &Interaction, &InventoryGlyphDisplayButton),
        (Changed<Interaction>, With<Button>),
    >,
    // Query to access all inventory buttons to update their colors
    mut all_buttons_query: Query<(Entity, &mut BackgroundColor, &InventoryGlyphDisplayButton), With<Button>>,
    mut selected_glyph: ResMut<SelectedInventoryGlyph>,
) {
    let mut pressed_button_info: Option<(Entity, GlyphId)> = None;

    // First, check for any presses among the buttons whose interaction changed
    for (entity, interaction, button_data) in changed_interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            pressed_button_info = Some((entity, button_data.glyph_id));
            break; // Process one press per frame for selection logic
        }
    }

    let mut selection_updated_this_frame = false;

    if let Some((pressed_entity, pressed_glyph_id)) = pressed_button_info {
        selection_updated_this_frame = true;
        if selected_glyph.glyph_id == Some(pressed_glyph_id) {
            // Clicked the currently selected glyph: deselect it
            selected_glyph.glyph_id = None;
            println!("Deselected inventory glyph: {:?}", pressed_glyph_id);
        } else {
            // Selected a new glyph
            selected_glyph.glyph_id = Some(pressed_glyph_id);
            println!("Selected inventory glyph: {:?}", pressed_glyph_id);
        }
    }

    // Update all button colors based on the final selection state and current hover states
    for (entity, mut bg_color, button_data) in all_buttons_query.iter_mut() {
        if selected_glyph.glyph_id == Some(button_data.glyph_id) {
            *bg_color = BUTTON_SELECTED_BG_COLOR.into();
        } else {
            // If no press occurred this frame OR if this button wasn't the one pressed but its interaction changed.
            // We need its current interaction state for hover.
            // The `changed_interaction_query` is only for *changed* interactions.
            // For buttons that didn't change interaction but need color reset (e.g., after another was selected),
            // we fall back to default. For hover on non-selected, we need their current Interaction.
            // This is tricky with two queries. A simpler way: if changed_interaction_query had the BgColor, use it.
            // Otherwise, iterate all_buttons and check interaction from a non-mut query if possible, or just handle selected/default.

            // Let's try to get the current interaction for *this* specific entity if it's in the changed_interaction_query.
            // This logic can be complex to get right without conflicts.
            // For now, if a press happened, all buttons are reset/set.
            // If no press, we only update based on hover for those in `changed_interaction_query` that are not selected.
            
            let mut current_interaction = Interaction::None; // Default if not in changed_interaction_query
            if !selection_updated_this_frame { // Only consider hover if no press changed the selection
                for (changed_entity, interaction, _) in changed_interaction_query.iter() {
                    if changed_entity == entity {
                        current_interaction = *interaction;
                        break;
                    }
                }
            }


            if selected_glyph.glyph_id == Some(button_data.glyph_id) { // Should be caught by the outer if, but defensive
                 *bg_color = BUTTON_SELECTED_BG_COLOR.into();
            } else {
                match current_interaction {
                    Interaction::Hovered => *bg_color = BUTTON_HOVER_BG_COLOR.into(),
                    Interaction::None => *bg_color = BUTTON_BG_COLOR.into(),
                    Interaction::Pressed => { // If pressed but not selected (e.g. another button was pressed)
                        *bg_color = BUTTON_BG_COLOR.into(); 
                    }
                }
            }
        }
    }
}