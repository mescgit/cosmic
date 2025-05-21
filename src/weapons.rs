use bevy::prelude::*;
use crate::{
    components::{Damage, Health, Lifetime}, 
    survivor::Survivor,
    horror::Horror,
    game::AppState,
    visual_effects::{spawn_damage_text, ImpactEffectRequest, spawn_impact_effect},
    audio::{PlaySoundEvent, SoundEffect},
};

const CIRCLE_OF_WARDING_BASE_RADIUS: f32 = 120.0;
const CIRCLE_OF_WARDING_BASE_DAMAGE: i32 = 5;
const CIRCLE_OF_WARDING_TICK_INTERVAL_SECONDS: f32 = 0.8;

const SWARM_OF_NIGHTMARES_BASE_RADIUS: f32 = 80.0;
const SWARM_OF_NIGHTMARES_BASE_DAMAGE: i32 = 3;
const SWARM_OF_NIGHTMARES_ROTATION_SPEED: f32 = 1.5; 
const SWARM_OF_NIGHTMARES_ORBIT_SPEED: f32 = 80.0; 
const SWARM_LARVA_SIZE: Vec2 = Vec2::new(12.0,12.0);

const DOOM_PULSE_AURA_BASE_RADIUS: f32 = 75.0;
const DOOM_PULSE_AURA_BASE_DAMAGE: i32 = 8;
const DOOM_PULSE_AURA_TICK_INTERVAL_SECONDS: f32 = 2.0; 
const DOOM_PULSE_AURA_VISUAL_DURATION_SECONDS: f32 = 0.3;


#[derive(Component, Debug, Reflect)] #[reflect(Component)] 
pub struct CircleOfWarding {
    pub is_active: bool,
    pub current_radius: f32,
    pub base_damage_per_tick: i32,
    pub damage_tick_timer: Timer,
}

#[derive(Component, Debug, Reflect)] #[reflect(Component)] 
pub struct SwarmOfNightmares {
    pub is_active: bool,
    pub num_larvae: u32,
    pub orbit_radius: f32,
    pub damage_per_hit: i32,
    pub rotation_speed: f32, 
    pub current_angle: f32,  
}

#[derive(Component)]
pub struct NightmareLarva {
    pub target_orbit_position: Vec2,
    pub parent_survivor: Entity, 
}

#[derive(Component, Debug, Reflect)] #[reflect(Component)] 
pub struct DoomPulseAura {
    pub damage: i32,
    pub radius_sq: f32,
    pub pulse_timer: Timer,
}

impl Default for CircleOfWarding {
    fn default() -> Self {
        Self {
            is_active: false, 
            current_radius: CIRCLE_OF_WARDING_BASE_RADIUS,
            base_damage_per_tick: CIRCLE_OF_WARDING_BASE_DAMAGE,
            damage_tick_timer: Timer::from_seconds(CIRCLE_OF_WARDING_TICK_INTERVAL_SECONDS, TimerMode::Repeating),
        }
    }
}

impl Default for SwarmOfNightmares {
    fn default() -> Self {
        Self {
            is_active: false, 
            num_larvae: 0, 
            orbit_radius: SWARM_OF_NIGHTMARES_BASE_RADIUS,
            damage_per_hit: SWARM_OF_NIGHTMARES_BASE_DAMAGE,
            rotation_speed: SWARM_OF_NIGHTMARES_ROTATION_SPEED,
            current_angle: 0.0,
        }
    }
}

impl Default for DoomPulseAura {
    fn default() -> Self {
        Self {
            damage: DOOM_PULSE_AURA_BASE_DAMAGE,
            radius_sq: DOOM_PULSE_AURA_BASE_RADIUS.powi(2),
            pulse_timer: Timer::from_seconds(DOOM_PULSE_AURA_TICK_INTERVAL_SECONDS, TimerMode::Repeating),
        }
    }
}


pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<CircleOfWarding>()
            .register_type::<SwarmOfNightmares>()
            .register_type::<DoomPulseAura>() 
            .add_systems(Update, 
                (
                    circle_of_warding_system,
                    manage_nightmare_larvae_system,
                    nightmare_larva_movement_system,
                    nightmare_larva_collision_system,
                    doom_pulse_aura_system, 
                ).chain().run_if(in_state(AppState::InGame))
            );
    }
}


fn circle_of_warding_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&GlobalTransform, &mut CircleOfWarding), With<Survivor>>,
    mut horror_query: Query<(&GlobalTransform, &mut Health), With<Horror>>,
    asset_server: Res<AssetServer>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
    mut impact_effect_requests: EventWriter<ImpactEffectRequest>,
) {
    for (survivor_g_transform, mut circle) in query.iter_mut() {
        if !circle.is_active { continue; }

        circle.damage_tick_timer.tick(time.delta());
        if circle.damage_tick_timer.just_finished() {
            let survivor_pos = survivor_g_transform.translation().truncate();
            let radius_sq = circle.current_radius.powi(2);

            sound_event_writer.send(PlaySoundEvent(SoundEffect::AuraPulse)); 

            for (horror_g_transform, mut horror_health) in horror_query.iter_mut() {
                let horror_pos = horror_g_transform.translation().truncate();
                if horror_pos.distance_squared(survivor_pos) < radius_sq {
                    horror_health.0 -= circle.base_damage_per_tick;
                    spawn_damage_text(&mut commands, &asset_server, horror_g_transform.translation(), circle.base_damage_per_tick, &time);
                    
                    impact_effect_requests.send(ImpactEffectRequest {
                        position: horror_g_transform.translation(),
                        base_color: Color::rgba(0.8, 0.7, 0.3, 0.7),
                        num_particles: 3,
                    });
                }
            }
        }
    }
}

fn manage_nightmare_larvae_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>, 
    mut survivor_query: Query<(Entity, &mut SwarmOfNightmares, &Transform), With<Survivor>>,
    larva_query: Query<Entity, With<NightmareLarva>>,
) {
    for (survivor_entity, mut swarm, _survivor_transform) in survivor_query.iter_mut() {
        if !swarm.is_active {
            for larva_entity in larva_query.iter() { 
                if let Some(ec) = commands.get_entity(larva_entity) { ec.despawn_recursive(); } 
            }
            continue;
        }

        let current_larva_count = larva_query.iter().count() as u32;
        if current_larva_count < swarm.num_larvae {
            for _ in 0..(swarm.num_larvae - current_larva_count) {
                commands.spawn((
                    SpriteBundle {
                        texture: asset_server.load("sprites/nightmare_larva_placeholder.png"),
                        sprite: Sprite { custom_size: Some(SWARM_LARVA_SIZE), ..default() },
                        transform: Transform::from_xyz(0.0, 0.0, 0.8), 
                        ..default()
                    },
                    NightmareLarva { target_orbit_position: Vec2::ZERO, parent_survivor: survivor_entity },
                    Damage(swarm.damage_per_hit), 
                    Name::new("NightmareLarva"),
                ));
            }
        } else if current_larva_count > swarm.num_larvae {
            for larva_entity in larva_query.iter().take((current_larva_count - swarm.num_larvae) as usize) {
                 if let Some(ec) = commands.get_entity(larva_entity) { ec.despawn_recursive(); } 
            }
        }
        swarm.current_angle += swarm.rotation_speed * time.delta_seconds();
        if swarm.current_angle > std::f32::consts::TAU {
            swarm.current_angle -= std::f32::consts::TAU;
        }
    }
}

fn nightmare_larva_movement_system(
    time: Res<Time>,
    mut larva_query: Query<(&mut Transform, &mut NightmareLarva), With<NightmareLarva>>,
    survivor_query: Query<&Transform, (With<Survivor>, Without<NightmareLarva>)>,
    swarm_query: Query<&SwarmOfNightmares, With<Survivor>>, 
) {
    if let Ok(survivor_transform) = survivor_query.get_single() {
        if let Ok(swarm) = swarm_query.get_single() {
            if !swarm.is_active { return; }

            let survivor_pos = survivor_transform.translation.truncate();
            let mut angle_step = 0.0;
            if swarm.num_larvae > 0 { 
                angle_step = 2.0 * std::f32::consts::PI / swarm.num_larvae as f32;
            }
            
            let base_angle = swarm.current_angle; 

            for (i, (mut larva_transform, mut larva_comp)) in larva_query.iter_mut().enumerate() {
                let current_larva_angle = base_angle + (i as f32 * angle_step);
                larva_comp.target_orbit_position = survivor_pos + Vec2::new(current_larva_angle.cos(), current_larva_angle.sin()) * swarm.orbit_radius;
                
                let direction_to_target = (larva_comp.target_orbit_position - larva_transform.translation.truncate()).normalize_or_zero();
                larva_transform.translation += (direction_to_target * SWARM_OF_NIGHTMARES_ORBIT_SPEED * time.delta_seconds()).extend(0.0);
            }
        }
    }
}

fn nightmare_larva_collision_system(
    mut commands: Commands,
    larva_query: Query<(Entity, &GlobalTransform, &Damage), With<NightmareLarva>>,
    mut horror_query: Query<(&GlobalTransform, &mut Health, &Horror)>, 
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    for (larva_entity, larva_g_transform, larva_damage) in larva_query.iter() {
        for (horror_g_transform, mut horror_health, horror_data) in horror_query.iter_mut() { 
            let distance = larva_g_transform.translation().truncate().distance(horror_g_transform.translation().truncate());
            let horror_radius = horror_data.size.x / 2.0; 
            if distance < SWARM_LARVA_SIZE.x / 2.0 + horror_radius { 
                sound_event_writer.send(PlaySoundEvent(SoundEffect::OrganicHit)); 
                horror_health.0 -= larva_damage.0;
                spawn_damage_text(&mut commands, &asset_server, horror_g_transform.translation(), larva_damage.0, &time);
                commands.entity(larva_entity).despawn_recursive();
                break; 
            }
        }
    }
}

fn doom_pulse_aura_system(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<(Entity, &GlobalTransform, &mut DoomPulseAura), With<Survivor>>,
    mut horror_query: Query<(&GlobalTransform, &mut Health), With<Horror>>,
    asset_server: Res<AssetServer>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
    mut impact_effect_requests: EventWriter<ImpactEffectRequest>,
) {
    for (player_entity, player_g_transform, mut aura) in player_query.iter_mut() {
        aura.pulse_timer.tick(time.delta());
        if aura.pulse_timer.just_finished() {
            let player_pos = player_g_transform.translation().truncate();
            sound_event_writer.send(PlaySoundEvent(SoundEffect::AuraPulse)); 

            commands.entity(player_entity).with_children(|parent| {
                parent.spawn((
                    SpriteBundle {
                        texture: asset_server.load("sprites/doom_pulse_effect_placeholder.png"), 
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(DOOM_PULSE_AURA_BASE_RADIUS * 2.0)),
                            color: Color::rgba(0.6, 0.1, 0.7, 0.5), 
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 0.1), 
                        ..default()
                    },
                    Lifetime { timer: Timer::from_seconds(DOOM_PULSE_AURA_VISUAL_DURATION_SECONDS, TimerMode::Once)},
                    Name::new("DoomPulseVisual"),
                ));
            });

            for (horror_g_transform, mut horror_health) in horror_query.iter_mut() { 
                let horror_pos = horror_g_transform.translation().truncate();
                if horror_pos.distance_squared(player_pos) < aura.radius_sq {
                    horror_health.0 -= aura.damage;
                    spawn_damage_text(&mut commands, &asset_server, horror_g_transform.translation(), aura.damage, &time);
                    impact_effect_requests.send(ImpactEffectRequest {
                        position: horror_g_transform.translation(),
                        base_color: Color::rgba(0.6, 0.1, 0.7, 0.7),
                        num_particles: 2,
                    });
                }
            }
        }
    }
}