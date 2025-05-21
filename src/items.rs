// src/items.rs
use bevy::prelude::*;
use crate::{
    survivor::Survivor, 
    components::{Health as ComponentHealth, Health},
    game::{AppState, ItemCollectedEvent},
    horror::Horror, 
    visual_effects::spawn_damage_text,
    audio::{PlaySoundEvent, SoundEffect},
    skills::{SkillId, SkillLibrary, ActiveSkillInstance}, 
    weapons::{CircleOfWarding, SwarmOfNightmares}, 
    // glyphs::GlyphId, // Removed unused import
};

// --- Standard Items (Relics) ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum SurvivorTemporaryBuff { HealthRegen { rate: f32, duration_secs: f32 }, }

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum ItemEffect {
    PassiveStatBoost { 
        max_health_increase: Option<i32>, 
        speed_multiplier: Option<f32>, 
        damage_increase: Option<i32>, 
        xp_gain_multiplier: Option<f32>, 
        pickup_radius_increase: Option<f32>,
        auto_weapon_projectile_speed_multiplier_increase: Option<f32>, 
    },
    OnAutomaticProjectileHitExplode { chance: f32, explosion_damage: i32, explosion_radius: f32, explosion_color: Color, },
    OnSurvivorHitRetaliate { chance: f32, retaliation_damage: i32, retaliation_radius: f32, retaliation_color: Color, },
    OnHorrorKillTrigger { chance: f32, effect: SurvivorTemporaryBuff, },
    GrantSpecificSkill { skill_id: SkillId, },
    ActivateCircleOfWarding { base_damage: i32, base_radius: f32, base_tick_interval: f32 },
    ActivateSwarmOfNightmares { num_larvae: u32, base_damage: i32, base_orbit_radius: f32, base_rotation_speed: f32 },
}

#[derive(Debug, Clone, Reflect)]
pub struct ItemDefinition { pub id: ItemId, pub name: String, pub description: String, pub effects: Vec<ItemEffect>, }

#[derive(Resource, Default, Reflect)] #[reflect(Resource)]
pub struct ItemLibrary { pub items: Vec<ItemDefinition>, }
impl ItemLibrary { pub fn get_item_definition(&self, id: ItemId) -> Option<&ItemDefinition> { self.items.iter().find(|def| def.id == id) } }

#[derive(Component, Debug)] pub struct ItemDrop { pub item_id: ItemId, }
pub const ITEM_DROP_SIZE: Vec2 = Vec2::new(24.0, 24.0);

#[derive(Component, Reflect, Default, Debug)] #[reflect(Component)]
pub struct ExplosionEffect { pub damage: i32, pub radius_sq: f32, pub timer: Timer, pub already_hit_entities: Vec<Entity>, }
#[derive(Component, Reflect, Default, Debug)] #[reflect(Component)]
pub struct RetaliationNovaEffect { pub damage: i32, pub radius_sq: f32, pub timer: Timer, pub already_hit_entities: Vec<Entity>, }
#[derive(Component, Reflect, Default, Debug)] #[reflect(Component)]
pub struct TemporaryHealthRegenBuff { pub regen_per_second: f32, pub duration_timer: Timer, }

// --- Automatic Weapons ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct AutomaticWeaponId(pub u32);

#[derive(Debug, Clone, Reflect)]
pub struct AutomaticWeaponDefinition {
    pub id: AutomaticWeaponId,
    pub name: String,
    pub base_damage: i32,
    pub base_fire_rate_secs: f32, 
    pub base_projectile_speed: f32,
    pub base_piercing: u32,
    pub additional_projectiles: u32, 
    
    pub projectile_sprite_path: &'static str, 
    pub projectile_size: Vec2,
    pub projectile_color: Color,
    pub projectile_lifetime_secs: f32,
    pub base_glyph_slots: u8, 
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct AutomaticWeaponLibrary {
    pub weapons: Vec<AutomaticWeaponDefinition>,
}

impl AutomaticWeaponLibrary {
    pub fn get_weapon_definition(&self, id: AutomaticWeaponId) -> Option<&AutomaticWeaponDefinition> {
        self.weapons.iter().find(|def| def.id == id)
    }
}


pub struct ItemsPlugin;
impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app .register_type::<ItemId>() .register_type::<SurvivorTemporaryBuff>() .register_type::<ItemEffect>() .register_type::<ItemLibrary>() 
            .register_type::<ExplosionEffect>() .register_type::<RetaliationNovaEffect>() .register_type::<TemporaryHealthRegenBuff>() 
            .register_type::<AutomaticWeaponId>() .register_type::<AutomaticWeaponDefinition>() .register_type::<AutomaticWeaponLibrary>()
            .init_resource::<ItemLibrary>()
            .init_resource::<AutomaticWeaponLibrary>() 
            .add_systems(Startup, (populate_item_library, populate_automatic_weapon_library) ) 
            .add_systems(Update, ( apply_collected_item_effects_system.run_if(on_event::<ItemCollectedEvent>()), explosion_effect_system.run_if(in_state(AppState::InGame)), retaliation_nova_effect_system.run_if(in_state(AppState::InGame)), temporary_health_regen_buff_system.run_if(in_state(AppState::InGame)), ));
    }
}

fn populate_automatic_weapon_library(mut library: ResMut<AutomaticWeaponLibrary>) {
    library.weapons.push(AutomaticWeaponDefinition {
        id: AutomaticWeaponId(0), 
        name: "Primordial Ichor Blast".to_string(),
        base_damage: 10, 
        base_fire_rate_secs: 0.5, 
        base_projectile_speed: 600.0, 
        base_piercing: 0,
        additional_projectiles: 0,
        projectile_sprite_path: "sprites/ichor_blast_placeholder.png", 
        projectile_size: Vec2::new(10.0, 10.0), 
        projectile_color: Color::rgb(0.7, 0.5, 1.0),
        projectile_lifetime_secs: 2.0, 
        base_glyph_slots: 2, 
    });

    library.weapons.push(AutomaticWeaponDefinition {
        id: AutomaticWeaponId(1), 
        name: "Eldritch Gatling".to_string(),
        base_damage: 5, 
        base_fire_rate_secs: 0.15, 
        base_projectile_speed: 550.0, 
        base_piercing: 0,
        additional_projectiles: 0,
        projectile_sprite_path: "sprites/eldritch_gatling_projectile_placeholder.png", 
        projectile_size: Vec2::new(8.0, 16.0),
        projectile_color: Color::rgb(0.3, 0.9, 0.4),
        projectile_lifetime_secs: 1.5, 
        base_glyph_slots: 3, 
    });

    library.weapons.push(AutomaticWeaponDefinition {
        id: AutomaticWeaponId(2), 
        name: "Void Cannon".to_string(),
        base_damage: 30, 
        base_fire_rate_secs: 1.25, 
        base_projectile_speed: 450.0, 
        base_piercing: 1, 
        additional_projectiles: 0,
        projectile_sprite_path: "sprites/void_cannon_projectile_placeholder.png", 
        projectile_size: Vec2::new(18.0, 18.0),
        projectile_color: Color::rgb(0.4, 0.1, 0.7),
        projectile_lifetime_secs: 2.5, 
        base_glyph_slots: 1, 
    });
}

fn populate_item_library(mut library: ResMut<ItemLibrary>) {
    library.items.push(ItemDefinition { id: ItemId(1), name: "Corrupted Heart".to_string(), description: "Increases Max Health by 25.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: Some(25), speed_multiplier: None, damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: None, auto_weapon_projectile_speed_multiplier_increase: None }], });
    library.items.push(ItemDefinition { id: ItemId(2), name: "Whispering Idol".to_string(), description: "Increases Movement Speed by 15%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: Some(1.15), damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: None, auto_weapon_projectile_speed_multiplier_increase: None }], });
    library.items.push(ItemDefinition { id: ItemId(3), name: "Shard of Agony".to_string(), description: "Increases automatic weapon damage by 5.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: Some(5), xp_gain_multiplier: None, pickup_radius_increase: None, auto_weapon_projectile_speed_multiplier_increase: None }], });
    library.items.push(ItemDefinition { id: ItemId(4), name: "Occult Tome Fragment".to_string(), description: "Increases XP gain by 20%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: None, xp_gain_multiplier: Some(1.20), pickup_radius_increase: None, auto_weapon_projectile_speed_multiplier_increase: None }], });
    library.items.push(ItemDefinition { id: ItemId(5), name: "Grasping Tentacle (Dried)".to_string(), description: "Increases pickup radius by 25%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: Some(0.25), auto_weapon_projectile_speed_multiplier_increase: None }], });
    library.items.push(ItemDefinition { id: ItemId(6), name: "Fragmented Sanity".to_string(), description: "Your automatic projectiles have a chance to violently detonate on impact.".to_string(), effects: vec![ItemEffect::OnAutomaticProjectileHitExplode { chance: 0.15, explosion_damage: 20, explosion_radius: 75.0, explosion_color: Color::rgba(1.0, 0.5, 0.2, 0.6), }], });
    library.items.push(ItemDefinition { id: ItemId(7), name: "Cloak of VengefulSpirits".to_string(), description: "When struck, has a chance to unleash a damaging psychic nova.".to_string(), effects: vec![ItemEffect::OnSurvivorHitRetaliate { chance: 0.25, retaliation_damage: 30, retaliation_radius: 120.0, retaliation_color: Color::rgba(0.9, 0.1, 0.1, 0.5), }], });
    library.items.push(ItemDefinition { id: ItemId(8), name: "Soul Siphon Shard".to_string(), description: "Defeated foes have a 20% chance to grant brief, rapid health regeneration.".to_string(), effects: vec![ItemEffect::OnHorrorKillTrigger { chance: 0.20, effect: SurvivorTemporaryBuff::HealthRegen { rate: 5.0, duration_secs: 3.0 }, }], });
    library.items.push(ItemDefinition { id: ItemId(9), name: "Tome of Forbidden Rites".to_string(), description: "Grants knowledge of the 'Void Lance' skill.".to_string(), effects: vec![ItemEffect::GrantSpecificSkill { skill_id: SkillId(3) }], });
    library.items.push(ItemDefinition { id: ItemId(10), name: "Glyph-Etched Wardstone".to_string(), description: "Activates a Circle of Warding, damaging nearby foes.".to_string(), effects: vec![ItemEffect::ActivateCircleOfWarding { base_damage: 3, base_radius: 75.0, base_tick_interval: 0.5, }], });
    library.items.push(ItemDefinition { id: ItemId(11), name: "Broodmother's Oculus".to_string(), description: "Summons a Swarm of Nightmares to orbit and attack enemies.".to_string(), effects: vec![ItemEffect::ActivateSwarmOfNightmares { num_larvae: 2, base_damage: 5, base_orbit_radius: 80.0, base_rotation_speed: std::f32::consts::PI / 2.0, }], });
    library.items.push(ItemDefinition { id: ItemId(12), name: "Crystalline Conduit".to_string(), description: "Increases automatic weapon damage by +3 and projectile speed by +10%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: Some(3), xp_gain_multiplier: None, pickup_radius_increase: None, auto_weapon_projectile_speed_multiplier_increase: Some(0.10) }], });
}

fn apply_collected_item_effects_system( 
    mut events: EventReader<ItemCollectedEvent>, 
    mut player_query: Query<(&mut Survivor, Option<&mut ComponentHealth>, Option<&mut CircleOfWarding>, Option<&mut SwarmOfNightmares>)>,
    item_library: Res<ItemLibrary>, 
    skill_library: Res<SkillLibrary>,
) { 
    if let Ok((mut player, mut opt_health_component, mut opt_circle_aura, mut opt_nightmare_swarm)) = player_query.get_single_mut() {
        for event in events.read() {
            let item_id = event.0; 
            let is_new_item = !player.collected_item_ids.contains(&item_id);
            if is_new_item {
                player.collected_item_ids.push(item_id);
            }

            if let Some(item_def) = item_library.get_item_definition(item_id) {
                for effect in &item_def.effects {
                    match effect {
                        ItemEffect::PassiveStatBoost { 
                            max_health_increase, 
                            speed_multiplier, 
                            damage_increase, 
                            xp_gain_multiplier, 
                            pickup_radius_increase, 
                            auto_weapon_projectile_speed_multiplier_increase 
                        } => {
                            if is_new_item { 
                                if let Some(hp_boost) = max_health_increase { player.max_health += *hp_boost; if let Some(ref mut health_comp) = opt_health_component { health_comp.0 += *hp_boost; health_comp.0 = health_comp.0.min(player.max_health); } }
                                if let Some(speed_mult) = speed_multiplier { player.speed *= *speed_mult; }
                                if let Some(dmg_inc) = damage_increase { player.auto_weapon_damage_bonus += *dmg_inc; } 
                                if let Some(xp_mult) = xp_gain_multiplier { player.xp_gain_multiplier *= *xp_mult; }
                                if let Some(radius_inc_percent) = pickup_radius_increase { player.pickup_radius_multiplier *= 1.0 + radius_inc_percent; }
                                if let Some(projectile_speed_inc) = auto_weapon_projectile_speed_multiplier_increase { player.auto_weapon_projectile_speed_multiplier *= 1.0 + projectile_speed_inc; }
                            }
                        }
                        ItemEffect::GrantSpecificSkill { skill_id } => {
                            if is_new_item { 
                                if let Some(skill_to_grant_def) = skill_library.get_skill_definition(*skill_id) { 
                                    let already_has_skill = player.equipped_skills.iter().any(|s| s.definition_id == *skill_id);
                                    if !already_has_skill { if player.equipped_skills.len() < 4 { 
                                        player.equipped_skills.push(ActiveSkillInstance::new(*skill_id, skill_to_grant_def.base_glyph_slots)); 
                                    } }
                                }
                            }
                        }
                        ItemEffect::ActivateCircleOfWarding { base_damage, base_radius, base_tick_interval } => {
                            if let Some(ref mut circle_aura) = opt_circle_aura {
                                if !circle_aura.is_active { 
                                    circle_aura.is_active = true;
                                    circle_aura.base_damage_per_tick = *base_damage;
                                    circle_aura.current_radius = *base_radius;
                                    circle_aura.damage_tick_timer = Timer::from_seconds(*base_tick_interval, TimerMode::Repeating);
                                } else {
                                    circle_aura.base_damage_per_tick += 1; 
                                }
                            }
                        }
                        ItemEffect::ActivateSwarmOfNightmares { num_larvae, base_damage, base_orbit_radius, base_rotation_speed } => {
                            if let Some(ref mut nightmare_swarm) = opt_nightmare_swarm {
                                if !nightmare_swarm.is_active { 
                                    nightmare_swarm.is_active = true;
                                    nightmare_swarm.num_larvae = *num_larvae;
                                    nightmare_swarm.damage_per_hit = *base_damage;
                                    nightmare_swarm.orbit_radius = *base_orbit_radius;
                                    nightmare_swarm.rotation_speed = *base_rotation_speed;
                                } else {
                                    nightmare_swarm.num_larvae = (nightmare_swarm.num_larvae + 1).min(5); 
                                    nightmare_swarm.damage_per_hit += 1;
                                }
                            }
                        }
                        _ => {} 
                    }
                }
            }
        }
    }
}

fn explosion_effect_system( mut commands: Commands, time: Res<Time>, mut explosion_query: Query<(Entity, &mut ExplosionEffect, &GlobalTransform, &mut Sprite, &mut Transform)>, mut horror_query: Query<(Entity, &GlobalTransform, &mut Health), With<Horror>>, asset_server: Res<AssetServer>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (explosion_entity, mut explosion, explosion_g_transform, mut sprite, mut vis_transform) in explosion_query.iter_mut() { explosion.timer.tick(time.delta()); let progress = explosion.timer.fraction(); let current_radius = explosion.radius_sq.sqrt(); vis_transform.scale = Vec3::splat(current_radius * 2.0 * progress); sprite.color.set_a(1.0 - progress); if explosion.timer.fraction() < 0.5 { let explosion_pos = explosion_g_transform.translation().truncate(); for (horror_entity, horror_gtransform, mut horror_health) in horror_query.iter_mut() { if explosion.already_hit_entities.contains(&horror_entity) { continue; } let horror_pos = horror_gtransform.translation().truncate(); if horror_pos.distance_squared(explosion_pos) < explosion.radius_sq { horror_health.0 -= explosion.damage; spawn_damage_text(&mut commands, &asset_server, horror_gtransform.translation(), explosion.damage, &time); sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit)); explosion.already_hit_entities.push(horror_entity); } } } if explosion.timer.finished() { commands.entity(explosion_entity).despawn_recursive(); } } }
fn retaliation_nova_effect_system( mut commands: Commands, time: Res<Time>, mut nova_query: Query<(Entity, &mut RetaliationNovaEffect, &GlobalTransform, &mut Sprite, &mut Transform)>, mut horror_query: Query<(Entity, &GlobalTransform, &mut Health), With<Horror>>, asset_server: Res<AssetServer>, mut sound_event_writer: EventWriter<PlaySoundEvent>,) { for (nova_entity, mut nova, nova_g_transform, mut sprite, mut vis_transform) in nova_query.iter_mut() { nova.timer.tick(time.delta()); let progress = nova.timer.fraction(); let current_radius = nova.radius_sq.sqrt(); vis_transform.scale = Vec3::splat(current_radius * 2.0 * progress); sprite.color.set_a(1.0 - progress * progress); if nova.timer.fraction() < 0.3 { let nova_pos = nova_g_transform.translation().truncate(); for (horror_entity, horror_gtransform, mut horror_health) in horror_query.iter_mut() { if nova.already_hit_entities.contains(&horror_entity) { continue; } let horror_pos = horror_gtransform.translation().truncate(); if horror_pos.distance_squared(nova_pos) < nova.radius_sq { horror_health.0 -= nova.damage; spawn_damage_text(&mut commands, &asset_server, horror_gtransform.translation(), nova.damage, &time); sound_event_writer.send(PlaySoundEvent(SoundEffect::HorrorHit)); nova.already_hit_entities.push(horror_entity); } } } if nova.timer.finished() { commands.entity(nova_entity).despawn_recursive(); } } }
fn temporary_health_regen_buff_system( mut commands: Commands, time: Res<Time>, mut buff_query: Query<(Entity, &mut TemporaryHealthRegenBuff, &Survivor, &mut ComponentHealth)>,) { for (entity, mut buff, survivor_stats, mut health_component) in buff_query.iter_mut() { buff.duration_timer.tick(time.delta()); if buff.duration_timer.finished() { commands.entity(entity).remove::<TemporaryHealthRegenBuff>(); } else { let regen_amount = buff.regen_per_second * time.delta().as_secs_f32(); health_component.0 = (health_component.0 as f32 + regen_amount).round() as i32; health_component.0 = health_component.0.min(survivor_stats.max_health); } } }