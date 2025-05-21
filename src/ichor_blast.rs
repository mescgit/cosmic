use bevy::prelude::*;
use rand::Rng; 
use crate::{
    components::{Velocity, Damage, Lifetime, Health},
    visual_effects::spawn_damage_text,
    audio::{PlaySoundEvent, SoundEffect},
    skills::SkillProjectile, // May not be needed if this becomes purely for automatic attacks
    horror::HorrorProjectile, 
    survivor::Survivor, 
    items::{ItemLibrary, ItemEffect, ExplosionEffect}, 
};

pub const ICHOR_BLAST_SIZE: Vec2 = Vec2::new(10.0, 10.0); // Default size, can be overridden by weapon
// BASE_FRAGMENT_SPEED, BASE_FRAGMENT_DAMAGE, FRAGMENT_LIFETIME_SECONDS are now part of WeaponDefinition

pub struct IchorBlastPlugin;

impl Plugin for IchorBlastPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                // projectile_movement_system is now general and might be moved or duplicated if specific behavior is needed
                // ichor_blast_collision_system might become automatic_projectile_collision_system
                // ichor_blast_lifetime_system might become automatic_projectile_lifetime_system
                // For now, we keep them but they will operate on AutomaticProjectile marker
                projectile_movement_system, 
                automatic_projectile_collision_system,
                automatic_projectile_lifetime_system,
            ).chain());
    }
}

#[derive(Component)]
pub struct AutomaticProjectile { // Renamed from IchorBlast
    pub piercing_left: u32,
    // Add other properties if needed, e.g., source_weapon_id: WeaponId
}

// This function becomes more generic for spawning any automatic projectile defined by a weapon
pub fn spawn_ichor_blast( // Consider renaming to spawn_automatic_projectile
    commands: &mut Commands, 
    asset_server: &Res<AssetServer>, 
    position: Vec3, 
    direction: Vec2, 
    damage: i32, 
    speed: f32, 
    piercing: u32,
    lifetime_secs: f32, // New parameter
    sprite_path: String,  // New parameter
    size: Vec2,           // New parameter
    color: Color,         // New parameter
) {
    commands.spawn((
        SpriteBundle { 
            texture: asset_server.load(&sprite_path), // Use parameter
            sprite: Sprite { custom_size: Some(size), color, ..default() }, // Use parameters
            transform: Transform::from_translation(position)
                .with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))), 
            ..default() 
        }, 
        AutomaticProjectile { piercing_left: piercing }, 
        Velocity(direction * speed), 
        Damage(damage), 
        Lifetime { timer: Timer::from_seconds(lifetime_secs, TimerMode::Once) }, // Use parameter
        Name::new("AutomaticProjectile"), 
    ));
}

// This system should now operate on AutomaticProjectile
fn projectile_movement_system( 
    mut query: Query<(&mut Transform, &Velocity), Or<(With<AutomaticProjectile>, With<HorrorProjectile>, With<SkillProjectile>)>>, 
    time: Res<Time>,
) {
    for (mut transform, velocity) in query.iter_mut() { 
        transform.translation.x += velocity.0.x * time.delta_seconds(); 
        transform.translation.y += velocity.0.y * time.delta_seconds(); 
    }
}

// This system should now operate on AutomaticProjectile
fn automatic_projectile_lifetime_system( 
    mut commands: Commands, 
    time: Res<Time>, 
    mut query: Query<(Entity, &mut Lifetime), With<AutomaticProjectile>>, 
) {
    for (entity, mut lifetime) in query.iter_mut() { 
        lifetime.timer.tick(time.delta()); 
        if lifetime.timer.just_finished() { 
            commands.entity(entity).despawn_recursive(); 
        } 
    }
}

// This system should now operate on AutomaticProjectile
fn automatic_projectile_collision_system(
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &GlobalTransform, &Damage, &mut AutomaticProjectile, &Sprite)>, // Changed IchorBlast to AutomaticProjectile
    mut horror_query: Query<(Entity, &GlobalTransform, &mut Health, &crate::horror::Horror)>,
    player_query: Query<&Survivor>, 
    item_library: Res<ItemLibrary>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    let Ok(player) = player_query.get_single() else { return };

    for (projectile_entity, projectile_gtransform, projectile_damage, mut projectile_stats, projectile_sprite) in projectile_query.iter_mut() {
        for (horror_entity, horror_gtransform, mut horror_health, horror_data) in horror_query.iter_mut() {
            let distance = projectile_gtransform.translation().truncate().distance(horror_gtransform.translation().truncate());
            
            let projectile_radius = projectile_sprite.custom_size.map_or(ICHOR_BLAST_SIZE.x, |s| s.x) / 2.0; // Use actual sprite size or default
            let enemy_radius = horror_data.size.x / 2.0;

            if distance < projectile_radius + enemy_radius {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit));
                horror_health.0 -= projectile_damage.0;
                spawn_damage_text(&mut commands, &asset_server, horror_gtransform.translation(), projectile_damage.0, &time);

                let mut rng = rand::thread_rng();
                for item_id in player.collected_item_ids.iter() {
                    if let Some(item_def) = item_library.get_item_definition(*item_id) {
                        for effect in &item_def.effects {
                            // Assuming OnIchorBlastHitExplode should now be OnAutomaticProjectileHitExplode or similar general term
                            if let ItemEffect::OnIchorBlastHitExplode { chance, explosion_damage, explosion_radius, explosion_color } = effect {
                                if rng.gen_bool((*chance).into()) {
                                    commands.spawn((
                                        SpriteBundle {
                                            texture: asset_server.load("sprites/eldritch_nova_effect_placeholder.png"),
                                            sprite: Sprite {
                                                custom_size: Some(Vec2::splat(0.1)), 
                                                color: *explosion_color,
                                                ..default()
                                            },
                                            transform: Transform::from_translation(horror_gtransform.translation().truncate().extend(0.3)),
                                            ..default()
                                        },
                                        ExplosionEffect {
                                            damage: *explosion_damage,
                                            radius_sq: explosion_radius.powi(2),
                                            timer: Timer::from_seconds(0.3, TimerMode::Once), 
                                            already_hit_entities: vec![horror_entity], 
                                        },
                                        Name::new("ItemHitExplosion"),
                                    ));
                                }
                            }
                        }
                    }
                }

                if projectile_stats.piercing_left > 0 {
                    projectile_stats.piercing_left -= 1;
                } else {
                    commands.entity(projectile_entity).despawn_recursive();
                    break; 
                }
            }
        }
    }
}