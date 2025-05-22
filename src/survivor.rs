// src/survivor.rs
use bevy::{prelude::*, window::PrimaryWindow};
// use std::time::Duration; // Removed, Timer should handle its own needs or Bevy prelude covers it
// use rand::Rng; // Removed
use crate::{
    components::{Velocity, Health as ComponentHealth},
    game::{AppState},
    horror::Horror,
    audio::{PlaySoundEvent, SoundEffect},
};

pub const SURVIVOR_SIZE: Vec2 = Vec2::new(50.0, 50.0);
const XP_FOR_LEVEL: [u32; 10] = [100, 150, 250, 400, 600, 850, 1100, 1400, 1800, 2500];
pub const BASE_PICKUP_RADIUS: f32 = 100.0;
const PROJECTILE_SPREAD_ANGLE_DEGREES: f32 = 10.0;
pub const INITIAL_SURVIVOR_MAX_HEALTH: i32 = 100;
const BASE_SURVIVOR_SPEED: f32 = 250.0;
const ITEM_COLLECTION_RADIUS: f32 = SURVIVOR_SIZE.x / 2.0 + crate::items::ITEM_DROP_SIZE.x / 2.0;

#[derive(Component)] pub struct SanityStrain { pub base_fire_rate_secs: f32, pub fire_timer: Timer, }

pub struct SurvivorPlugin;
#[derive(Component)]
pub struct Survivor {
    pub speed: f32,
    // pub experience: u32, // Removed
    // pub current_level_xp: u32, // Removed
    // pub level: u32, // Removed
    pub aim_direction: Vec2,
    pub invincibility_timer: Timer,

    // pub auto_weapon_damage_bonus: i32, // Removed
    // pub auto_weapon_projectile_speed_multiplier: f32, // Removed
    // pub auto_weapon_piercing_bonus: u32, // Removed
    // pub auto_weapon_additional_projectiles_bonus: u32, // Removed

    // pub xp_gain_multiplier: f32, // Removed
    // pub pickup_radius_multiplier: f32, // Removed
    pub max_health: i32,
    // pub health_regen_rate: f32, // Removed
    // pub equipped_skills: Vec<ActiveSkillInstance>, // Removed
    // pub collected_item_ids: Vec<ItemId>, // Removed
    // pub collected_glyphs: Vec<GlyphId>, // Removed
    // pub equipped_weapon_id: Option<AutomaticWeaponId>, // Removed
    // pub auto_weapon_equipped_glyphs: Vec<Option<GlyphId>>, // Removed
}

impl Survivor {
    // All methods related to experience and leveling removed.
    // pub fn experience_to_next_level(&self) -> u32 { ... } // Removed
    // pub fn add_experience( &mut self, amount: u32, ...) { ... } // Removed
    // pub fn get_effective_pickup_radius(&self) -> f32 { BASE_PICKUP_RADIUS * self.pickup_radius_multiplier } // Removed

    pub fn new() -> Self {
        Self {
            speed: BASE_SURVIVOR_SPEED,
            aim_direction: Vec2::X,
            invincibility_timer: Timer::from_seconds(1.0, TimerMode::Once),
            max_health: INITIAL_SURVIVOR_MAX_HEALTH,
        }
    }
}

fn should_despawn_survivor(next_state: Res<NextState<AppState>>) -> bool { match next_state.0 { Some(AppState::GameOver) | Some(AppState::MainMenu) => true, _ => false, } }
fn no_survivor_exists(survivor_query: Query<(), With<Survivor>>) -> bool { survivor_query.is_empty() }

// --- Basic Player Projectile ---
#[derive(Component)]
pub struct BasicPlayerProjectile;

#[derive(Component)]
pub struct Damage(pub i32);

#[derive(Component)]
pub struct Lifetime(pub Timer);

const BASIC_PROJECTILE_SPEED: f32 = 600.0;
const BASIC_PROJECTILE_LIFETIME_SECS: f32 = 1.0;
const BASIC_PROJECTILE_DAMAGE: i32 = 10;

fn spawn_basic_player_projectile(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    start_transform: Transform,
    direction: Vec2,
) {
    let projectile_sprite = "sprites/eldritch_bolt_placeholder.png"; // Placeholder sprite

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load(projectile_sprite),
            sprite: Sprite { custom_size: Some(Vec2::new(15.0, 15.0)), ..default() }, // Placeholder size
            transform: start_transform,
            ..default()
        },
        BasicPlayerProjectile,
        Velocity(direction.normalize_or_zero() * BASIC_PROJECTILE_SPEED),
        Damage(BASIC_PROJECTILE_DAMAGE),
        Lifetime(Timer::from_seconds(BASIC_PROJECTILE_LIFETIME_SECS, TimerMode::Once)),
        Name::new("BasicPlayerProjectile"),
    ));
}

// --- Player Attack Timer ---
#[derive(Resource)]
pub struct PlayerAttackTimer(pub Timer);

// --- Systems ---
impl Plugin for SurvivorPlugin {
    fn build(&self, app: &mut App) {
        app .add_systems(OnEnter(AppState::InGame), spawn_survivor.run_if(no_survivor_exists))
            .add_systems(Update, (
                survivor_movement,
                survivor_aiming,
                survivor_basic_attack_system, // Replaced survivor_casting_system
                // survivor_health_regeneration_system, // To be removed
                survivor_horror_collision_system.before(check_survivor_death_system),
                survivor_invincibility_system,
                check_survivor_death_system,
                // survivor_item_drop_collection_system, // To be removed
            ).chain().run_if(in_state(AppState::InGame)))
            .add_systems(OnExit(AppState::InGame), despawn_survivor.run_if(should_despawn_survivor));
    }
}

fn spawn_survivor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("sprites/survivor_placeholder.png"),
            sprite: Sprite { custom_size: Some(SURVIVOR_SIZE), ..default() },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Survivor::new(),
        ComponentHealth(INITIAL_SURVIVOR_MAX_HEALTH),
        Velocity(Vec2::ZERO),
        Name::new("Survivor"),
    ));
    // Add PlayerAttackTimer as a resource, initialized and repeating.
    commands.insert_resource(PlayerAttackTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
}
fn despawn_survivor(mut commands: Commands, survivor_query: Query<Entity, With<Survivor>>) { if let Ok(survivor_entity) = survivor_query.get_single() { commands.entity(survivor_entity).despawn_recursive(); } }
fn survivor_health_regeneration_system(time: Res<Time>, mut query: Query<(&Survivor, &mut ComponentHealth)>,) { /* Removed */ }
fn survivor_movement( keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<(&Survivor, &mut Transform, &mut Velocity)>, time: Res<Time>,) { // Removed SurvivorBuffEffect
    for (survivor, mut transform, mut velocity) in query.iter_mut() {
        let mut direction = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keyboard_input.pressed(KeyCode::KeyD) { direction.x += 1.0; }
        if keyboard_input.pressed(KeyCode::KeyW) { direction.y += 1.0; }
        if keyboard_input.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
        let current_speed = survivor.speed; // Removed buff effect logic
        velocity.0 = if direction != Vec2::ZERO { direction.normalize() * current_speed } else { Vec2::ZERO };
        transform.translation.x += velocity.0.x * time.delta_seconds();
        transform.translation.y += velocity.0.y * time.delta_seconds();
    }
}
fn survivor_aiming(mut survivor_query: Query<(&mut Survivor, &Transform)>, window_query: Query<&Window, With<PrimaryWindow>>, camera_query: Query<(&Camera, &GlobalTransform)>,) { if let Ok((mut survivor, survivor_transform)) = survivor_query.get_single_mut() { if let Ok(primary_window) = window_query.get_single() { if let Ok((camera, camera_transform)) = camera_query.get_single() { if let Some(cursor_position) = primary_window.cursor_position() { if let Some(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) { let direction_to_mouse = (world_position - survivor_transform.translation.truncate()).normalize_or_zero(); if direction_to_mouse != Vec2::ZERO { survivor.aim_direction = direction_to_mouse; } } } } } } }

fn survivor_basic_attack_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    player_query: Query<(&Transform, &Survivor)>,
    mut player_attack_timer: ResMut<PlayerAttackTimer>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    if let Ok((survivor_transform, survivor_stats)) = player_query.get_single() {
        player_attack_timer.0.tick(time.delta());
        if mouse_button_input.pressed(MouseButton::Left) && player_attack_timer.0.just_finished() {
            if survivor_stats.aim_direction != Vec2::ZERO {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::RitualCast)); // Placeholder sound
                spawn_basic_player_projectile(
                    &mut commands,
                    &asset_server,
                    *survivor_transform,
                    survivor_stats.aim_direction,
                );
            }
        }
    }
}

fn survivor_horror_collision_system(
    // mut commands: Commands, // No longer needed as no components are added/removed here related to collision effects
    mut survivor_query: Query<(&Transform, &mut ComponentHealth, &mut Survivor)>,
    horror_query: Query<(&Transform, &Horror)>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    if let Ok((survivor_transform, mut survivor_health, mut survivor_component)) = survivor_query.get_single_mut() {
        if !survivor_component.invincibility_timer.finished() { return; }

        for (horror_transform, horror_stats) in horror_query.iter() {
            let distance = survivor_transform.translation.truncate().distance(horror_transform.translation.truncate());
            let survivor_radius = SURVIVOR_SIZE.x / 2.0; // Assuming SURVIVOR_SIZE is still defined
            let horror_radius = horror_stats.size.x / 2.0;

            if distance < survivor_radius + horror_radius {
                // No need to check invincibility_timer.finished() again, already checked for the survivor
                sound_event_writer.send(PlaySoundEvent(SoundEffect::SurvivorHit));
                let damage_to_take = horror_stats.damage_on_collision;

                if damage_to_take > 0 {
                    survivor_health.0 -= damage_to_take;
                }
                survivor_component.invincibility_timer.reset();
                // Break here if we only want one collision to register per frame for the survivor
                // For simplicity, let's assume multiple horrors can hit in the same frame if close enough
                // and invincibility starts after this frame's processing of collisions.
            }
        }
    }
}
fn survivor_invincibility_system(time: Res<Time>, mut query: Query<(&mut Survivor, &mut Sprite, &ComponentHealth)>,) { for (mut survivor, mut sprite, health) in query.iter_mut() { if health.0 <= 0 { if sprite.color.a() != 1.0 { sprite.color.set_a(1.0); } continue; } if !survivor.invincibility_timer.finished() { survivor.invincibility_timer.tick(time.delta()); let alpha = (time.elapsed_seconds() * 20.0).sin() / 2.0 + 0.7; sprite.color.set_a(alpha.clamp(0.3, 1.0) as f32); } else { if sprite.color.a() != 1.0 { sprite.color.set_a(1.0); } } } }
fn check_survivor_death_system(survivor_query: Query<&ComponentHealth, With<Survivor>>, mut app_state_next: ResMut<NextState<AppState>>, mut sound_event_writer: EventWriter<PlaySoundEvent>, current_app_state: Res<State<AppState>>,) { if let Ok(survivor_health) = survivor_query.get_single() { if survivor_health.0 <= 0 && *current_app_state.get() == AppState::InGame { sound_event_writer.send(PlaySoundEvent(SoundEffect::MadnessConsumes)); app_state_next.set(AppState::GameOver); } } }
// fn survivor_item_drop_collection_system removed
// fn survivor_health_regeneration_system was already removed (body commented out)