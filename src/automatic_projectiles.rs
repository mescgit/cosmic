// src/automatic_projectiles.rs
use bevy::prelude::*;
use bevy_hanabi::prelude::*; // Hanabi prelude
use rand::Rng;
use crate::{
    components::{Velocity, Damage, Lifetime, Health},
    visual_effects::spawn_damage_text,
    audio::{PlaySoundEvent, SoundEffect},
    survivor::Survivor,
    items::{ItemLibrary, ItemEffect, ExplosionEffect, AutomaticWeaponId},
    horror::Horror,
};

pub const BASE_CHAIN_LIGHTNING_RANGE: f32 = 300.0;
pub const CHAIN_LIGHTNING_DAMAGE_MULTIPLIER: f32 = 0.75;

#[derive(Component)]
pub struct AutomaticProjectile {
    pub piercing_left: u32,
    pub chains_left: u32,
    pub weapon_id: AutomaticWeaponId,
    pub already_hit_entities: Vec<Entity>,
    pub damage_amount: i32,
}

#[derive(Component)]
pub struct ChainLightningVisual {
    pub timer: Timer,
}

#[derive(Component)]
pub struct ChainLightningStrikeEvent {
    pub source_position: Vec3,
    pub target_entity: Entity,
    pub damage: i32,
    pub remaining_chains: u32,
    pub already_hit_in_chain: Vec<Entity>,
    pub chain_range_sq: f32,
}

#[derive(Resource)]
pub struct LightningParticleEffects {
    pub bolt_effect: Handle<EffectAsset>,
}

fn setup_lightning_particle_effects(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: Res<AssetServer>,
) {
    let mut module = Module::default(); // Create module instance

    // Define gradients
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.8, 0.8, 1.0, 1.0));
    color_gradient.add_key(0.5, Vec4::new(0.5, 0.5, 1.0, 1.0));
    color_gradient.add_key(1.0, Vec4::new(0.3, 0.3, 1.0, 0.0));

    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec2::splat(6.0));
    size_gradient.add_key(0.3, Vec2::splat(8.0));
    size_gradient.add_key(1.0, Vec2::splat(0.0));

    // Define modifiers that use `module.lit()` BEFORE module is moved
    let pos_modifier = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(2.0_f32),
        dimension: ShapeDimension::Volume,
    };
    let vel_modifier = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(100.0_f32),
    };
    let lifetime_modifier = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(0.3_f32));
    let color_modifier = SetAttributeModifier::new(Attribute::COLOR, module.lit(Vec4::ONE));
    let size_modifier = SetAttributeModifier::new(Attribute::SIZE, module.lit(Vec2::splat(1.0)));
    let drag_modifier = LinearDragModifier { drag: module.lit(5.0_f32) };

    // Spawner::once expects a CpuValue<f32>, created via .into() from f32 literal
    let spawner = Spawner::once(10.0_f32.into(), true);

    let texture_handle: Handle<Image> = asset_server.load("sprites/scorch_mark.png");

    // Create EffectAsset, moving module here
    let bolt_effect_asset = EffectAsset::new(
        vec![32], // capacities
        spawner,   // spawner
        module     // module is moved here
    )
    .with_name("lightning_bolt")
    .init(pos_modifier) // Pass the pre-defined modifier instance
    .init(vel_modifier)
    .init(lifetime_modifier)
    .init(color_modifier)
    .init(size_modifier)
    .update(drag_modifier)
    // Render modifiers can be added after, as they don't use the `module` in the same way
    .add_render_modifier(Box::new(ColorOverLifetimeModifier { gradient: color_gradient }))
    .add_render_modifier(Box::new(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false }))
    .add_render_modifier(Box::new(ParticleTextureModifier { texture: texture_handle.into(), ..default()}));

    let bolt_effect_handle = effects.add(bolt_effect_asset);
    commands.insert_resource(LightningParticleEffects { bolt_effect: bolt_effect_handle });
}

pub fn spawn_automatic_projectile(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    position: Vec3,
    direction: Vec2,
    damage: i32,
    speed: f32,
    piercing: u32,
    chains: u32,
    weapon_id: AutomaticWeaponId,
    sprite_path: &'static str,
    size: Vec2,
    color: Color,
    lifetime_secs: f32,
    particle_effects: Option<&Res<LightningParticleEffects>>,
) {
    let mut projectile_entity_commands = commands.spawn_empty();

    projectile_entity_commands.insert((
        AutomaticProjectile {
            piercing_left: piercing,
            chains_left: chains,
            weapon_id,
            already_hit_entities: Vec::new(),
            damage_amount: damage,
        },
        Velocity(direction * speed),
        Damage(damage),
        Lifetime { timer: Timer::from_seconds(lifetime_secs, TimerMode::Once) },
        Name::new(format!("AutoProj_{:?}", weapon_id)),
        VisibilityBundle::default(), // Added to fix B0004 warning
        GlobalTransform::default(),
        Transform::from_translation(position).with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))),
    ));

    if weapon_id == AutomaticWeaponId(3) {
        if let Some(effects_res) = particle_effects {
            projectile_entity_commands.with_children(|parent| {
                parent.spawn(ParticleEffectBundle {
                    effect: ParticleEffect::new(effects_res.bolt_effect.clone()),
                    transform: Transform::IDENTITY,
                    ..Default::default()
                });
            });
        } else {
            warn!("LightningParticleEffects resource not found for Chain Lightning, falling back to sprite.");
            projectile_entity_commands.with_children(|parent| {
                parent.spawn( SpriteBundle {
                    texture: asset_server.load(sprite_path),
                    sprite: Sprite { custom_size: Some(size), color, ..default() },
                    ..default()
                });
            });
        }
    } else {
        projectile_entity_commands.with_children(|parent| {
            parent.spawn( SpriteBundle {
                texture: asset_server.load(sprite_path),
                sprite: Sprite { custom_size: Some(size), color, ..default() },
                ..default()
            });
        });
    }
}

fn projectile_movement_system(
    mut query: Query<(&mut Transform, &Velocity), Or<(With<AutomaticProjectile>, With<crate::horror::HorrorProjectile>, With<crate::skills::SkillProjectile>)>>,
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
    mut projectile_query: Query<(Entity, &GlobalTransform, &Damage, &mut AutomaticProjectile, &Velocity)>,
    mut horror_query: Query<(Entity, &GlobalTransform, &mut Health, &Horror)>,
    player_query: Query<&Survivor>,
    item_library: Res<ItemLibrary>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    let Ok(player) = player_query.get_single() else { return; };

    for (projectile_entity, proj_gtransform, _proj_main_hit_damage_comp, mut proj_stats, _proj_velocity) in projectile_query.iter_mut() {
        let proj_pos = proj_gtransform.translation();
        let projectile_radius = 5.0;

        for (horror_entity, horror_gtransform, mut horror_health, horror_data) in horror_query.iter_mut() {
            if proj_stats.already_hit_entities.contains(&horror_entity) {
                continue;
            }
            let distance = proj_pos.truncate().distance(horror_gtransform.translation().truncate());
            let horror_radius = horror_data.size.x / 2.0;

            if distance < projectile_radius + horror_radius {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit));
                horror_health.0 -= proj_stats.damage_amount;
                spawn_damage_text(&mut commands, &asset_server, horror_gtransform.translation(), proj_stats.damage_amount, &time);
                proj_stats.already_hit_entities.push(horror_entity);

                for item_id in player.collected_item_ids.iter() {
                    if let Some(item_def) = item_library.get_item_definition(*item_id) {
                        for effect in &item_def.effects {
                            if let ItemEffect::OnAutomaticProjectileHitExplode { chance, explosion_damage, explosion_radius, explosion_color } = effect {
                                let mut rng = rand::thread_rng();
                                if rng.gen_bool((*chance).into()) {
                                    commands.spawn((
                                        SpriteBundle {
                                            texture: asset_server.load("sprites/eldritch_nova_effect_placeholder.png"),
                                            sprite: Sprite { custom_size: Some(Vec2::splat(0.1)), color: *explosion_color, ..default() },
                                            transform: Transform::from_translation(horror_gtransform.translation().truncate().extend(0.3)), ..default()
                                        },
                                        ExplosionEffect { damage: *explosion_damage, radius_sq: explosion_radius.powi(2), timer: Timer::from_seconds(0.3, TimerMode::Once), already_hit_entities: vec![horror_entity], },
                                        Name::new("ItemHitExplosion"),
                                    ));
                                }
                            }
                        }
                    }
                }

                if proj_stats.chains_left > 0 && proj_stats.weapon_id == AutomaticWeaponId(3) {
                    let effective_chain_range = BASE_CHAIN_LIGHTNING_RANGE * player.auto_weapon_chain_range_multiplier;
                    commands.spawn(ChainLightningStrikeEvent {
                        source_position: horror_gtransform.translation(),
                        target_entity: horror_entity,
                        damage: (proj_stats.damage_amount as f32 * CHAIN_LIGHTNING_DAMAGE_MULTIPLIER).round() as i32,
                        remaining_chains: proj_stats.chains_left,
                        already_hit_in_chain: vec![horror_entity],
                        chain_range_sq: effective_chain_range.powi(2),
                    });
                    proj_stats.chains_left = 0;
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

fn chain_lightning_strike_processor_system(
    mut commands: Commands,
    strike_query: Query<(Entity, &ChainLightningStrikeEvent)>,
    mut horror_query: Query<(Entity, &GlobalTransform, &mut Health, &Horror)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    for (event_entity, strike_data) in strike_query.iter() {
        let mut potential_targets: Vec<(Entity, f32)> = Vec::new();
        for (horror_entity, horror_gtransform, _horror_health, _horror_data) in horror_query.iter() {
            if strike_data.already_hit_in_chain.contains(&horror_entity) {
                continue;
            }
            let distance_sq = strike_data.source_position.truncate().distance_squared(horror_gtransform.translation().truncate());
            if distance_sq < strike_data.chain_range_sq {
                potential_targets.push((horror_entity, distance_sq));
            }
        }
        potential_targets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((target_horror_entity, _)) = potential_targets.first() {
            if let Ok((_h_ent, target_gtransform, mut target_health, _h_data)) = horror_query.get_mut(*target_horror_entity) {
                let target_pos = target_gtransform.translation();
                let midpoint = (strike_data.source_position + target_pos) / 2.0;
                let distance = strike_data.source_position.distance(target_pos);
                let angle = (target_pos.y - strike_data.source_position.y).atan2(target_pos.x - strike_data.source_position.x);

                commands.spawn((
                    SpriteBundle {
                        texture: asset_server.load("sprites/chain_lightning_segment_placeholder.png"),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(distance, 5.0)),
                            color: Color::rgba(0.8, 0.8, 1.0, 0.7),
                            ..default()
                        },
                        transform: Transform::from_translation(midpoint.truncate().extend(0.8))
                            .with_rotation(Quat::from_rotation_z(angle)),
                        ..default()
                    },
                    ChainLightningVisual { timer: Timer::from_seconds(0.15, TimerMode::Once) },
                    Name::new("ChainLightningSegment"),
                ));

                sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit));
                target_health.0 -= strike_data.damage;
                spawn_damage_text(&mut commands, &asset_server, target_gtransform.translation(), strike_data.damage, &time);

                let mut next_hit_list = strike_data.already_hit_in_chain.clone();
                next_hit_list.push(*target_horror_entity);

                if strike_data.remaining_chains > 1 {
                    commands.spawn(ChainLightningStrikeEvent {
                        source_position: target_gtransform.translation(),
                        target_entity: *target_horror_entity,
                        damage: (strike_data.damage as f32 * CHAIN_LIGHTNING_DAMAGE_MULTIPLIER).round() as i32,
                        remaining_chains: strike_data.remaining_chains - 1,
                        already_hit_in_chain: next_hit_list,
                        chain_range_sq: strike_data.chain_range_sq,
                    });
                }
            }
        }
        commands.entity(event_entity).despawn();
    }
}

fn chain_lightning_visual_despawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut ChainLightningVisual)>,
) {
    for (entity, mut visual) in query.iter_mut() {
        visual.timer.tick(time.delta());
        if visual.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub struct AutomaticProjectilesPlugin;

impl Plugin for AutomaticProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_lightning_particle_effects)
            .add_systems(Update, (
                projectile_movement_system,
                automatic_projectile_collision_system.after(projectile_movement_system),
                automatic_projectile_lifetime_system,
                chain_lightning_strike_processor_system,
                chain_lightning_visual_despawn_system,
            ).chain());
    }
}