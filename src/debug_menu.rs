// src/debug_menu.rs
use bevy::prelude::*;
use crate::{
    upgrades::{UpgradePool, UpgradeCard},
    game::{AppState, UpgradeChosenEvent, ItemCollectedEvent},
    audio::{PlaySoundEvent, SoundEffect},
    items::{ItemLibrary, ItemId, AutomaticWeaponLibrary}, // Keep AutomaticWeaponLibrary if debug_weapon_switch_system uses it
    skills::{SkillLibrary}, // Removed ActiveSkillInstance as it's not used
    survivor::Survivor,
};

pub struct DebugMenuPlugin;

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::DebugUpgradeMenu), setup_debug_menu_ui)
            .add_systems(Update,
                (
                    debug_menu_button_interaction_system,
                    debug_item_button_interaction_system,
                    debug_menu_keyboard_scroll_system,
                )
                .run_if(in_state(AppState::DebugUpgradeMenu))
            )
            .add_systems(OnExit(AppState::DebugUpgradeMenu), despawn_debug_menu_ui);
    }
}

#[derive(Component)] struct DebugMenuUIRoot;
#[derive(Component)] struct DebugUpgradeButton(UpgradeCard);
#[derive(Component)] struct DebugItemButton(ItemId);
#[derive(Component)] struct DebugMenuScrollView;
#[derive(Component)] struct DebugMenuScrollableContent;
#[derive(Component)] struct ScrollOffset(f32);

const DEBUG_BUTTON_HEIGHT: Val = Val::Px(20.0);
const DEBUG_BUTTON_MARGIN: Val = Val::Px(2.0);
const DEBUG_TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const DEBUG_BUTTON_BG_COLOR: Color = Color::rgb(0.25, 0.25, 0.25);
const DEBUG_BUTTON_HOVER_BG_COLOR: Color = Color::rgb(0.35, 0.35, 0.35);
const DEBUG_BUTTON_PRESSED_BG_COLOR: Color = Color::rgb(0.15, 0.15, 0.15);
const DEBUG_SCROLL_AREA_BG_COLOR: Color = Color::rgba(0.12, 0.12, 0.12, 1.0);
const KEYBOARD_SCROLL_SPEED: f32 = 30.0;

fn setup_debug_menu_ui(
    mut commands: Commands, asset_server: Res<AssetServer>,
    upgrade_pool: Res<UpgradePool>, item_library: Res<ItemLibrary>,
    _skill_library: Res<SkillLibrary>, // Prefixed as unused for now
    _weapon_library: Res<AutomaticWeaponLibrary>, // Prefixed as unused for now
    player_query: Query<&Survivor>,
) {
    let Ok(_player) = player_query.get_single() else { return; }; 
    
    commands.spawn(( NodeBundle { style: Style { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, background_color: Color::rgba(0.0, 0.0, 0.0, 0.90).into(), z_index: ZIndex::Global(50), ..default() }, DebugMenuUIRoot, Name::new("DebugMenuUIRoot"), )).with_children(|parent| {
        parent.spawn(NodeBundle { style: Style { width: Val::Percent(90.0), min_width: Val::Px(900.0), max_width: Val::Px(1400.0), height: Val::Percent(90.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceAround, border: UiRect::all(Val::Px(2.0)), padding: UiRect::all(Val::Px(10.0)), ..default() }, border_color: BorderColor(Color::DARK_GRAY).into(), background_color: Color::rgb(0.05, 0.05, 0.07).into(), ..default()
        }).with_children(|sections_container| {
            // Upgrades Panel
            sections_container.spawn(NodeBundle { style: Style { flex_direction: FlexDirection::Column, flex_basis: Val::Percent(33.0),  margin: UiRect::horizontal(Val::Px(5.0)), ..default() }, ..default() }).with_children(|panel| { panel.spawn(TextBundle::from_section( "UPGRADES", TextStyle {font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 16.0, color: Color::ORANGE_RED,}, ).with_style(Style {margin: UiRect::bottom(Val::Px(8.0)), align_self: AlignSelf::Center, ..default()})); panel.spawn(( NodeBundle { style: Style { overflow: Overflow { y: OverflowAxis::Clip, ..default() }, flex_grow: 1.0, ..default()}, background_color: DEBUG_SCROLL_AREA_BG_COLOR.into(), ..default() }, DebugMenuScrollView, ScrollOffset(0.0), Name::new("UpgradeScroll"), )).with_children(|scroll| { scroll.spawn(( NodeBundle {style: Style {position_type: PositionType::Absolute, width: Val::Percent(100.0), top: Val::Px(0.0), left: Val::Px(0.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch, ..default()}, ..default()}, DebugMenuScrollableContent, Name::new("UpgradeList"), )).with_children(|list| { for card in upgrade_pool.available_upgrades.iter() { list.spawn(( ButtonBundle { style: Style {height: DEBUG_BUTTON_HEIGHT, margin: UiRect::bottom(DEBUG_BUTTON_MARGIN), padding: UiRect::horizontal(Val::Px(5.0)), justify_content: JustifyContent::FlexStart, align_items: AlignItems::Center, ..default()}, background_color: DEBUG_BUTTON_BG_COLOR.into(), ..default()}, DebugUpgradeButton(card.clone()), Name::new(format!("DbgUp:{}", card.name)), )).with_children(|btn| { btn.spawn(TextBundle::from_section(format!("[{}] {}", card.id.0, card.name), TextStyle {font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 11.0, color: DEBUG_TEXT_COLOR,}));}); } }); }); });
            // Items Panel
            sections_container.spawn(NodeBundle { style: Style { flex_direction: FlexDirection::Column, flex_basis: Val::Percent(33.0),  margin: UiRect::horizontal(Val::Px(5.0)), ..default() }, ..default() }).with_children(|panel| { panel.spawn(TextBundle::from_section( "ITEMS (Grant)", TextStyle {font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 16.0, color: Color::CYAN,}, ).with_style(Style {margin: UiRect::bottom(Val::Px(8.0)), align_self: AlignSelf::Center, ..default()})); panel.spawn(( NodeBundle { style: Style { overflow: Overflow { y: OverflowAxis::Clip, ..default() }, flex_grow: 1.0, ..default()}, background_color: DEBUG_SCROLL_AREA_BG_COLOR.into(), ..default() }, DebugMenuScrollView, ScrollOffset(0.0), Name::new("ItemScroll"), )).with_children(|scroll| { scroll.spawn(( NodeBundle {style: Style {position_type: PositionType::Absolute, width: Val::Percent(100.0), top: Val::Px(0.0), left: Val::Px(0.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch, ..default()}, ..default()}, DebugMenuScrollableContent, Name::new("ItemList"), )).with_children(|list| { for item_def in item_library.items.iter() { list.spawn(( ButtonBundle { style: Style {height: DEBUG_BUTTON_HEIGHT, margin: UiRect::bottom(DEBUG_BUTTON_MARGIN), padding: UiRect::horizontal(Val::Px(5.0)), justify_content: JustifyContent::FlexStart, align_items: AlignItems::Center, ..default()}, background_color: DEBUG_BUTTON_BG_COLOR.into(), ..default()}, DebugItemButton(item_def.id), Name::new(format!("DbgItem:{}", item_def.name)), )).with_children(|btn| { btn.spawn(TextBundle::from_section(format!("[{}] {}", item_def.id.0, item_def.name), TextStyle {font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 11.0, color: DEBUG_TEXT_COLOR,}));}); } }); }); });
            
             sections_container.spawn(NodeBundle { style: Style { flex_direction: FlexDirection::Column, flex_basis: Val::Percent(33.0), margin: UiRect::horizontal(Val::Px(5.0)), ..default() }, ..default() }).with_children(|panel| {
                panel.spawn(TextBundle::from_section( "OTHER (Placeholder)", TextStyle {font: asset_server.load("fonts/FiraSans-Bold.ttf"), font_size: 16.0, color: Color::GRAY,}, ).with_style(Style {margin: UiRect::bottom(Val::Px(8.0)), align_self: AlignSelf::Center, ..default()}));
             });
        });
    });
}

fn debug_menu_keyboard_scroll_system( keyboard_input: Res<ButtonInput<KeyCode>>, mut scroll_view_query: Query<(&mut ScrollOffset, &Node, &Children, &GlobalTransform), With<DebugMenuScrollView>>, mut content_query: Query<(&Node, &mut Style), With<DebugMenuScrollableContent>>, window_query: Query<&Window, With<bevy::window::PrimaryWindow>>, ) { let Ok(_primary_window) = window_query.get_single() else { return }; let _cursor_pos_option = _primary_window.cursor_position(); for (mut scroll_offset, scroll_view_node, scroll_view_children, _scroll_view_gtransform) in scroll_view_query.iter_mut() { let mut content_entity = None; for &child in scroll_view_children.iter() { if content_query.get(child).is_ok() { content_entity = Some(child); break; } } if let Some(content_e) = content_entity { if let Ok((content_node, mut content_style)) = content_query.get_mut(content_e) { let scroll_view_height = scroll_view_node.size().y; let content_height = content_node.size().y; let mut new_offset = scroll_offset.0; let mut scrolled = false; if keyboard_input.pressed(KeyCode::ArrowUp) { new_offset -= KEYBOARD_SCROLL_SPEED; scrolled = true; } if keyboard_input.pressed(KeyCode::ArrowDown) { new_offset += KEYBOARD_SCROLL_SPEED; scrolled = true; } if scrolled { let max_scroll = (content_height - scroll_view_height).max(0.0); new_offset = new_offset.clamp(0.0, max_scroll); if (scroll_offset.0 - new_offset).abs() > f32::EPSILON { scroll_offset.0 = new_offset; content_style.top = Val::Px(-new_offset); } } } } } }
fn debug_menu_button_interaction_system( mut interaction_query: Query<(&Interaction, &DebugUpgradeButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>, mut upgrade_chosen_event: EventWriter<UpgradeChosenEvent>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (interaction, debug_button_data, mut bg_color) in interaction_query.iter_mut() { match *interaction { Interaction::Pressed => { *bg_color = DEBUG_BUTTON_PRESSED_BG_COLOR.into(); sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); upgrade_chosen_event.send(UpgradeChosenEvent(debug_button_data.0.clone())); } Interaction::Hovered => { *bg_color = DEBUG_BUTTON_HOVER_BG_COLOR.into(); } Interaction::None => { *bg_color = DEBUG_BUTTON_BG_COLOR.into(); } } } }
fn debug_item_button_interaction_system( mut interaction_query: Query<(&Interaction, &DebugItemButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>, mut item_collected_event: EventWriter<ItemCollectedEvent>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (interaction, debug_item_button, mut bg_color) in interaction_query.iter_mut() { match *interaction { Interaction::Pressed => { *bg_color = DEBUG_BUTTON_PRESSED_BG_COLOR.into(); sound_event_writer.send(PlaySoundEvent(SoundEffect::OmenAccepted)); item_collected_event.send(ItemCollectedEvent(debug_item_button.0)); } Interaction::Hovered => { *bg_color = DEBUG_BUTTON_HOVER_BG_COLOR.into(); } Interaction::None => { *bg_color = DEBUG_BUTTON_BG_COLOR.into(); } } } }
fn despawn_debug_menu_ui(mut commands: Commands, query: Query<Entity, With<DebugMenuUIRoot>>) { for entity in query.iter() { commands.entity(entity).despawn_recursive(); } }