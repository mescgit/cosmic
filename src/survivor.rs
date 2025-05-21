// src/survivor.rs
use bevy::{prelude::*, window::PrimaryWindow};
use std::time::Duration;
use rand::Rng;
use crate::{
    components::{Velocity, Health as ComponentHealth},
    game::{AppState, ItemCollectedEvent},
    automatic_projectiles::{spawn_automatic_projectile},
    horror::Horror,
    weapons::{CircleOfWarding, SwarmOfNightmares},
    audio::{PlaySoundEvent, SoundEffect},
    skills::{ActiveSkillInstance, SkillLibrary, SkillId, SurvivorBuffEffect, ActiveShield},
    items::{ItemId, ItemDrop, ItemLibrary, ItemEffect, RetaliationNovaEffect, AutomaticWeaponId, AutomaticWeaponLibrary},
    // glyphs::{GlyphId, GlyphLibrary, GlyphEffectType}, // Commented out
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
    pub speed: f32, pub experience: u32, pub current_level_xp: u32, pub level: u32,
    pub aim_direction: Vec2, pub invincibility_timer: Timer,

    pub auto_weapon_damage_bonus: i32,
    pub auto_weapon_projectile_speed_multiplier: f32,
    pub auto_weapon_piercing_bonus: u32,
    pub auto_weapon_additional_projectiles_bonus: u32,

    pub xp_gain_multiplier: f32, pub pickup_radius_multiplier: f32,
    pub max_health: i32, pub health_regen_rate: f32,
    pub equipped_skills: Vec<ActiveSkillInstance>,
    pub collected_item_ids: Vec<ItemId>,
    // pub collected_glyphs: Vec<GlyphId>, // Commented out
    pub equipped_weapon_id: Option<AutomaticWeaponId>,
    // pub auto_weapon_equipped_glyphs: Vec<Option<GlyphId>>, // Commented out
}

impl Survivor {
    pub fn experience_to_next_level(&self) -> u32 { if self.level == 0 { return 0; } if (self.level as usize -1) < XP_FOR_LEVEL.len() { XP_FOR_LEVEL[self.level as usize - 1] } else { XP_FOR_LEVEL.last().unwrap_or(&2500) + (self.level - XP_FOR_LEVEL.len() as u32) * 500 } }
    pub fn add_experience( &mut self, amount: u32, next_state_value: &mut NextState<AppState>, sound_event_writer: &mut EventWriter<PlaySoundEvent>,) { let actual_xp_gained = (amount as f32 * self.xp_gain_multiplier).round() as u32; self.current_level_xp += actual_xp_gained; self.experience += actual_xp_gained; while self.current_level_xp >= self.experience_to_next_level() && self.level > 0 { let needed = self.experience_to_next_level(); self.current_level_xp -= needed; self.level += 1; sound_event_writer.send(PlaySoundEvent(SoundEffect::Revelation)); next_state_value.set(AppState::LevelUp); if next_state_value.0 == Some(AppState::LevelUp) { break; } } }
    pub fn get_effective_pickup_radius(&self) -> f32 { BASE_PICKUP_RADIUS * self.pickup_radius_multiplier }

    pub fn new_with_skills_and_items(
        initial_skills: Vec<ActiveSkillInstance>,
        initial_items: Vec<ItemId>,
        initial_weapon_id: Option<AutomaticWeaponId>,
        weapon_library: &Res<AutomaticWeaponLibrary>,
    ) -> Self {
        // let mut initial_auto_weapon_glyphs = Vec::new(); // Commented out
        // if let Some(w_id) = initial_weapon_id { // Commented out
        //     if let Some(w_def) = weapon_library.get_weapon_definition(w_id) { // Commented out
        //         // initial_auto_weapon_glyphs = vec![None; w_def.base_glyph_slots as usize]; // Commented out
        //     }
        // }

        Self {
            speed: BASE_SURVIVOR_SPEED,
            experience: 0, current_level_xp: 0, level: 1,
            aim_direction: Vec2::X,
            invincibility_timer: Timer::from_seconds(1.0, TimerMode::Once),
            auto_weapon_damage_bonus: 0,
            auto_weapon_projectile_speed_multiplier: 1.0,
            auto_weapon_piercing_bonus: 0,
            auto_weapon_additional_projectiles_bonus: 0,
            xp_gain_multiplier: 1.0,
            pickup_radius_multiplier: 1.0,
            max_health: INITIAL_SURVIVOR_MAX_HEALTH,
            health_regen_rate: 0.0,
            equipped_skills: initial_skills,
            collected_item_ids: initial_items,
            // collected_glyphs: Vec::new(), // Commented out
            equipped_weapon_id: initial_weapon_id,
            // auto_weapon_equipped_glyphs: initial_auto_weapon_glyphs, // Commented out
        }
    }
}

fn should_despawn_survivor(next_state: Res<NextState<AppState>>) -> bool { match next_state.0 { Some(AppState::GameOver) | Some(AppState::MainMenu) => true, _ => false, } }
fn no_survivor_exists(survivor_query: Query<(), With<Survivor>>) -> bool { survivor_query.is_empty() }

impl Plugin for SurvivorPlugin {
    fn build(&self, app: &mut App) {
        app .add_systems(OnEnter(AppState::InGame), spawn_survivor.run_if(no_survivor_exists))
            .add_systems(Update, (
                survivor_movement,
                survivor_aiming,
                survivor_casting_system,
                survivor_health_regeneration_system,
                survivor_horror_collision_system.before(check_survivor_death_system),
                survivor_invincibility_system,
                check_survivor_death_system,
                survivor_item_drop_collection_system,
            ).chain().run_if(in_state(AppState::InGame)))
            .add_systems(OnExit(AppState::InGame), despawn_survivor.run_if(should_despawn_survivor));
    }
}

fn spawn_survivor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    skill_library: Res<SkillLibrary>,
    weapon_library: Res<AutomaticWeaponLibrary>,
) {
    let mut initial_skills = Vec::new();
    if let Some(skill_def_bolt) = skill_library.get_skill_definition(SkillId(1)) {
        // let bolt_instance = ActiveSkillInstance::new(SkillId(1), skill_def_bolt.base_glyph_slots); // Original
        let bolt_instance = ActiveSkillInstance::new(SkillId(1) /*, skill_def_bolt.base_glyph_slots // Commented out glyph slots */);
        initial_skills.push(bolt_instance);
    }

    let default_weapon_id = AutomaticWeaponId(0);
    let mut initial_fire_rate = 0.5;

    if let Some(weapon_def) = weapon_library.get_weapon_definition(default_weapon_id) {
        initial_fire_rate = weapon_def.base_fire_rate_secs;
    }

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("sprites/survivor_placeholder.png"),
            sprite: Sprite { custom_size: Some(SURVIVOR_SIZE), ..default() },
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        },
        Survivor::new_with_skills_and_items(initial_skills, Vec::new(), Some(default_weapon_id), &weapon_library),
        ComponentHealth(INITIAL_SURVIVOR_MAX_HEALTH),
        Velocity(Vec2::ZERO),
        SanityStrain {
            base_fire_rate_secs: initial_fire_rate,
            fire_timer: Timer::from_seconds(initial_fire_rate, TimerMode::Repeating),
        },
        CircleOfWarding::default(),
        SwarmOfNightmares::default(),
        Name::new("Survivor"),
    ));
}
fn despawn_survivor(mut commands: Commands, survivor_query: Query<Entity, With<Survivor>>) { if let Ok(survivor_entity) = survivor_query.get_single() { commands.entity(survivor_entity).despawn_recursive(); } }
fn survivor_health_regeneration_system(time: Res<Time>, mut query: Query<(&Survivor, &mut ComponentHealth)>,) { for (survivor_stats, mut current_health) in query.iter_mut() { if survivor_stats.health_regen_rate > 0.0 && current_health.0 > 0 && current_health.0 < survivor_stats.max_health { let regen_amount = survivor_stats.health_regen_rate * time.delta_seconds(); current_health.0 = (current_health.0 as f32 + regen_amount).round() as i32; current_health.0 = current_health.0.min(survivor_stats.max_health); } } }
fn survivor_movement( keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<(&Survivor, &mut Transform, &mut Velocity, Option<&SurvivorBuffEffect>)>, time: Res<Time>,) { for (survivor, mut transform, mut velocity, buff_effect_opt) in query.iter_mut() { let mut direction = Vec2::ZERO; if keyboard_input.pressed(KeyCode::KeyA) { direction.x -= 1.0; } if keyboard_input.pressed(KeyCode::KeyD) { direction.x += 1.0; } if keyboard_input.pressed(KeyCode::KeyW) { direction.y += 1.0; } if keyboard_input.pressed(KeyCode::KeyS) { direction.y -= 1.0; } let mut current_speed = survivor.speed; if let Some(buff) = buff_effect_opt { current_speed *= 1.0 + buff.speed_multiplier_bonus; } velocity.0 = if direction != Vec2::ZERO { direction.normalize() * current_speed } else { Vec2::ZERO }; transform.translation.x += velocity.0.x * time.delta_seconds(); transform.translation.y += velocity.0.y * time.delta_seconds(); } }
fn survivor_aiming(mut survivor_query: Query<(&mut Survivor, &Transform)>, window_query: Query<&Window, With<PrimaryWindow>>, camera_query: Query<(&Camera, &GlobalTransform)>,) { if let Ok((mut survivor, survivor_transform)) = survivor_query.get_single_mut() { if let Ok(primary_window) = window_query.get_single() { if let Ok((camera, camera_transform)) = camera_query.get_single() { if let Some(cursor_position) = primary_window.cursor_position() { if let Some(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) { let direction_to_mouse = (world_position - survivor_transform.translation.truncate()).normalize_or_zero(); if direction_to_mouse != Vec2::ZERO { survivor.aim_direction = direction_to_mouse; } } } } } } }

fn survivor_casting_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut query: Query<(&Transform, &Survivor, &mut SanityStrain, Option<&SurvivorBuffEffect>)>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
    weapon_library: Res<AutomaticWeaponLibrary>,
    // glyph_library: Res<GlyphLibrary>, // Commented out
) {
    for (survivor_transform, survivor_stats, mut sanity_strain, buff_effect_opt) in query.iter_mut() {
        let weapon_def = match survivor_stats.equipped_weapon_id {
            Some(id) => weapon_library.get_weapon_definition(id),
            None => return,
        }.unwrap_or_else(|| weapon_library.get_weapon_definition(AutomaticWeaponId(0)).expect("Default weapon ID 0 not found"));

        let mut effective_fire_rate_secs = sanity_strain.base_fire_rate_secs;

        // --- Commented out Glyph Logic ---
        // for glyph_opt in survivor_stats.auto_weapon_equipped_glyphs.iter() {
        //     if let Some(glyph_id) = glyph_opt {
        //         if let Some(glyph_def) = glyph_library.get_glyph_definition(*glyph_id) {
        //             match &glyph_def.effect {
        //                 GlyphEffectType::IncreaseRate { percent_boost } => {
        //                     effective_fire_rate_secs /= 1.0 + percent_boost;
        //                 }
        //                 _ => {}
        //             }
        //         }
        //     }
        // }
        // --- End Commented out Glyph Logic ---

        if let Some(buff) = buff_effect_opt {
            effective_fire_rate_secs /= 1.0 + buff.fire_rate_multiplier_bonus;
        }

        let new_duration = Duration::from_secs_f32(effective_fire_rate_secs.max(0.05));
        if sanity_strain.fire_timer.duration() != new_duration {
            sanity_strain.fire_timer.set_duration(new_duration);
        }
        sanity_strain.fire_timer.tick(time.delta());

        if sanity_strain.fire_timer.just_finished() {
            if survivor_stats.aim_direction != Vec2::ZERO {
                sound_event_writer.send(PlaySoundEvent(SoundEffect::RitualCast));

                let mut current_damage = weapon_def.base_damage + survivor_stats.auto_weapon_damage_bonus;
                let mut effective_projectile_lifetime_secs = weapon_def.projectile_lifetime_secs;

                // --- Commented out Glyph Logic ---
                // for glyph_opt in survivor_stats.auto_weapon_equipped_glyphs.iter() {
                //     if let Some(glyph_id) = glyph_opt {
                //         if let Some(glyph_def) = glyph_library.get_glyph_definition(*glyph_id) {
                //             match &glyph_def.effect {
                //                 GlyphEffectType::IncreaseBaseDamage { amount } => {
                //                     current_damage += *amount;
                //                 }
                //                 GlyphEffectType::IncreaseEffectScale { percent_boost } => {
                //                     effective_projectile_lifetime_secs *= 1.0 + percent_boost;
                //                 }
                //                 _ => {}
                //             }
                //         }
                //     }
                // }
                // --- End Commented out Glyph Logic ---

                let current_speed = weapon_def.base_projectile_speed * survivor_stats.auto_weapon_projectile_speed_multiplier;
                let current_piercing = weapon_def.base_piercing + survivor_stats.auto_weapon_piercing_bonus;
                let total_fragments = 1 + weapon_def.additional_projectiles + survivor_stats.auto_weapon_additional_projectiles_bonus;

                let base_angle = survivor_stats.aim_direction.to_angle();
                for i in 0..total_fragments {
                    let angle_offset_rad = if total_fragments > 1 {
                        let total_spread_angle_rad = (total_fragments as f32 - 1.0) * PROJECTILE_SPREAD_ANGLE_DEGREES.to_radians();
                        let start_angle_rad = base_angle - total_spread_angle_rad / 2.0;
                        start_angle_rad + (i as f32 * PROJECTILE_SPREAD_ANGLE_DEGREES.to_radians())
                    } else { base_angle };
                    let fragment_direction = Vec2::from_angle(angle_offset_rad);

                    spawn_automatic_projectile(
                        &mut commands,
                        &asset_server,
                        survivor_transform.translation,
                        fragment_direction,
                        current_damage,
                        current_speed,
                        current_piercing,
                        weapon_def.id,
                        weapon_def.projectile_sprite_path,
                        weapon_def.projectile_size,
                        weapon_def.projectile_color,
                        effective_projectile_lifetime_secs,
                    );
                }
            }
        }
    }
}
fn survivor_horror_collision_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut survivor_query: Query<(Entity, &Transform, &mut ComponentHealth, &mut Survivor, Option<&mut ActiveShield>)>,
    horror_query: Query<(&Transform, &Horror)>,
    item_library: Res<ItemLibrary>,
    mut sound_event_writer: EventWriter<PlaySoundEvent>,
) {
    if let Ok((survivor_entity, survivor_transform, mut survivor_health, mut survivor_component, mut opt_active_shield)) = survivor_query.get_single_mut() {
        if !survivor_component.invincibility_timer.finished() { return; }

        for (horror_transform, horror_stats) in horror_query.iter() {
            let distance = survivor_transform.translation.truncate().distance(horror_transform.translation.truncate());
            let survivor_radius = SURVIVOR_SIZE.x / 2.0;
            let horror_radius = horror_stats.size.x / 2.0;

            if distance < survivor_radius + horror_radius {
                if survivor_component.invincibility_timer.finished() {
                    sound_event_writer.send(PlaySoundEvent(SoundEffect::SurvivorHit));
                    let mut damage_to_take = horror_stats.damage_on_collision;

                    if let Some(ref mut shield) = opt_active_shield {
                        if shield.amount > 0 {
                            let damage_absorbed = damage_to_take.min(shield.amount);
                            shield.amount -= damage_absorbed;
                            damage_to_take -= damage_absorbed;

                            if shield.amount <= 0 {
                                commands.entity(survivor_entity).remove::<ActiveShield>();
                            }
                        }
                    }

                    if damage_to_take > 0 {
                        survivor_health.0 -= damage_to_take;
                    }

                    survivor_component.invincibility_timer.reset();

                    let mut rng = rand::thread_rng();
                    for item_id in survivor_component.collected_item_ids.iter() {
                        if let Some(item_def) = item_library.get_item_definition(*item_id) {
                            for effect in &item_def.effects {
                                if let ItemEffect::OnSurvivorHitRetaliate { chance, retaliation_damage, retaliation_radius, retaliation_color } = effect {
                                    if rng.gen_bool((*chance).into()) {
                                        commands.entity(survivor_entity).with_children(|parent| {
                                            parent.spawn((
                                                SpriteBundle { texture: asset_server.load("sprites/eldritch_nova_effect_placeholder.png"), sprite: Sprite { custom_size: Some(Vec2::splat(0.1)), color: *retaliation_color, ..default() }, transform: Transform::from_xyz(0.0, 0.0, 0.3), ..default() },
                                                RetaliationNovaEffect { damage: *retaliation_damage, radius_sq: retaliation_radius.powi(2), timer: Timer::from_seconds(0.4, TimerMode::Once), already_hit_entities: Vec::new(), },
                                                Name::new("RetaliationNova"),
                                            ));
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
fn survivor_invincibility_system(time: Res<Time>, mut query: Query<(&mut Survivor, &mut Sprite, &ComponentHealth)>,) { for (mut survivor, mut sprite, health) in query.iter_mut() { if health.0 <= 0 { if sprite.color.a() != 1.0 { sprite.color.set_a(1.0); } continue; } if !survivor.invincibility_timer.finished() { survivor.invincibility_timer.tick(time.delta()); let alpha = (time.elapsed_seconds() * 20.0).sin() / 2.0 + 0.7; sprite.color.set_a(alpha.clamp(0.3, 1.0) as f32); } else { if sprite.color.a() != 1.0 { sprite.color.set_a(1.0); } } } }
fn check_survivor_death_system(survivor_query: Query<&ComponentHealth, With<Survivor>>, mut app_state_next: ResMut<NextState<AppState>>, mut sound_event_writer: EventWriter<PlaySoundEvent>, current_app_state: Res<State<AppState>>,) { if let Ok(survivor_health) = survivor_query.get_single() { if survivor_health.0 <= 0 && *current_app_state.get() == AppState::InGame { sound_event_writer.send(PlaySoundEvent(SoundEffect::MadnessConsumes)); app_state_next.set(AppState::GameOver); } } }
fn survivor_item_drop_collection_system(mut commands: Commands, survivor_query: Query<&Transform, With<Survivor>>, item_drop_query: Query<(Entity, &Transform, &ItemDrop)>, mut item_collected_event_writer: EventWriter<ItemCollectedEvent>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { if let Ok(survivor_transform) = survivor_query.get_single() { let survivor_pos = survivor_transform.translation.truncate(); for (item_drop_entity, item_drop_transform, item_drop_data) in item_drop_query.iter() { let item_drop_pos = item_drop_transform.translation.truncate(); if survivor_pos.distance(item_drop_pos) < ITEM_COLLECTION_RADIUS { item_collected_event_writer.send(ItemCollectedEvent(item_drop_data.item_id)); sound_event_writer.send(PlaySoundEvent(SoundEffect::SoulCollect)); commands.entity(item_drop_entity).despawn_recursive(); } } } }