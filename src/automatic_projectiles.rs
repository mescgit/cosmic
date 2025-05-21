// src/automatic_projectiles.rs
use bevy::prelude::*;
use rand::Rng; 
use crate::{
    components::{Velocity, Damage, Lifetime, Health},
    visual_effects::spawn_damage_text,
    audio::{PlaySoundEvent, SoundEffect},
    skills::SkillProjectile, 
    horror::HorrorProjectile, 
    survivor::Survivor, 
    items::{ItemLibrary, ItemEffect, ExplosionEffect, AutomaticWeaponId}, 
};

pub struct AutomaticProjectilesPlugin;

impl Plugin for AutomaticProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                projectile_movement_system, 
                automatic_projectile_collision_system, 
                automatic_projectile_lifetime_system, 
            ).chain());
    }
}

#[derive(Component)]
pub struct AutomaticProjectile { 
    pub piercing_left: u32,
    pub weapon_id: AutomaticWeaponId, 
}

pub fn spawn_automatic_projectile( 
    commands: &mut Commands, 
    asset_server: &Res<AssetServer>, 
    position: Vec3, 
    direction: Vec2, 
    damage: i32, 
    speed: f32, 
    piercing: u32,
    weapon_id: AutomaticWeaponId, 
    sprite_path: &'static str, // Changed to &'static str
    size: Vec2,
    color: Color,
    lifetime_secs: f32,
) {
    commands.spawn(( 
        SpriteBundle { 
            texture: asset_server.load(sprite_path), 
            sprite: Sprite { 
                custom_size: Some(size), 
                color, 
                ..default() 
            }, 
            transform: Transform::from_translation(position).with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))), 
            ..default() 
        }, 
        AutomaticProjectile { piercing_left: piercing, weapon_id }, 
        Velocity(direction * speed), 
        Damage(damage), 
        Lifetime { timer: Timer::from_seconds(lifetime_secs, TimerMode::Once) }, 
        Name::new("AutomaticProjectile"), 
    ));
}

fn projectile_movement_system( 
    mut query: Query<(&mut Transform, &Velocity), Or<(With<AutomaticProjectile>, With<HorrorProjectile>, With<SkillProjectile>)>>, 
    time: Res<Time>,
) {
    for (mut transform, velocity) in query.iter_mut() { 
        transform.translation.x += velocity.0.x * time.delta_seconds(); 
        transform.translation.y += velocity.0.y * time.delta_seconds(); 
    }
}

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

fn automatic_projectile_collision_system( 
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &GlobalTransform, &Damage, &mut AutomaticProjectile, &Sprite)>, 
    mut horror_query: Query<(Entity, &GlobalTransform, &mut Health, &crate::horror::Horror)>,
    player_query: Query<&Survivor>, 
    item_library: Res<ItemLibrary>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    let Ok(player) = player_query.get_single() else { return };

    for (projectile_entity, proj_gtransform, proj_damage, mut proj_stats, proj_sprite) in projectile_query.iter_mut() {
        for (horror_entity, horror_gtransform, mut horror_health, horror_data) in horror_query.iter_mut() {
            let distance = proj_gtransform.translation().truncate().distance(horror_gtransform.translation().truncate());
            
            let projectile_radius = proj_sprite.custom_size.map_or(5.0, |s| s.x.max(s.y) / 2.0);
            let horror_radius = horror_data.size.x / 2.0;

            if distance < projectile_radius + horror_radius {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit));
                horror_health.0 -= proj_damage.0;
                spawn_damage_text(&mut commands, &asset_server, horror_gtransform.translation(), proj_damage.0, &time);

                for item_id in player.collected_item_ids.iter() {
                    if let Some(item_def) = item_library.get_item_definition(*item_id) {
                        for effect in &item_def.effects {
                            if let ItemEffect::OnAutomaticProjectileHitExplode { chance, explosion_damage, explosion_radius, explosion_color } = effect {
                                let mut rng = rand::thread_rng();
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

                if proj_stats.piercing_left > 0 {
                    proj_stats.piercing_left -= 1;
                } else {
                    commands.entity(projectile_entity).despawn_recursive();
                    break; 
                }
            }
        }
    }
}