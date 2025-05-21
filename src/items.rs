use bevy::prelude::*;
use crate::{
    survivor::Survivor, 
    components::{Health as ComponentHealth, Health},
    game::{AppState, ItemCollectedEvent},
    horror::Horror, 
    visual_effects::spawn_damage_text,
    audio::{PlaySoundEvent, SoundEffect},
    skills::{SkillId, SkillLibrary, ActiveSkillInstance}, 
    weapons::{CircleOfWarding, SwarmOfNightmares, DoomPulseAura}, 
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct WeaponId(pub u32);

#[derive(Debug, Clone, Reflect)]
pub struct WeaponDefinition {
    pub id: WeaponId,
    pub name: String,
    pub projectile_sprite_path: String, 
    pub base_damage: i32,
    pub base_speed: f32,
    pub base_lifetime_secs: f32,
    pub base_piercing: u32,
    pub num_projectiles_at_once: u32, 
    pub projectile_spread_angle_degrees: f32, 
    pub projectile_size: Vec2,
    pub projectile_color: Color,
}

#[derive(Resource, Default, Reflect)] #[reflect(Resource)]
pub struct WeaponLibrary {
    pub weapons: Vec<WeaponDefinition>,
}
impl WeaponLibrary {
    pub fn get_weapon_definition(&self, id: WeaponId) -> Option<&WeaponDefinition> {
        self.weapons.iter().find(|def| def.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ItemCategory {
    Relic,
    Ring,
}


#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum SurvivorTemporaryBuff { HealthRegen { rate: f32, duration_secs: f32 }, }

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum ItemEffect {
    PassiveStatBoost { max_health_increase: Option<i32>, speed_multiplier: Option<f32>, damage_increase: Option<i32>, xp_gain_multiplier: Option<f32>, pickup_radius_increase: Option<f32>, },
    OnIchorBlastHitExplode { chance: f32, explosion_damage: i32, explosion_radius: f32, explosion_color: Color, },
    OnSurvivorHitRetaliate { chance: f32, retaliation_damage: i32, retaliation_radius: f32, retaliation_color: Color, },
    OnHorrorKillTrigger { chance: f32, effect: SurvivorTemporaryBuff, },
    GrantSpecificSkill { skill_id: SkillId, },
    ActivateCircleOfWarding, 
    ActivateSwarmOfNightmares, 
    ActivateDoomPulse,
}

#[derive(Debug, Clone, Reflect)]
pub struct ItemDefinition { 
    pub id: ItemId, 
    pub name: String, 
    pub description: String, 
    pub effects: Vec<ItemEffect>,
    pub category: Option<ItemCategory>,
}

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

pub struct ItemsPlugin;
impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app .register_type::<ItemId>() 
            .register_type::<WeaponId>() 
            .register_type::<WeaponDefinition>() 
            .register_type::<WeaponLibrary>() 
            .register_type::<ItemCategory>()
            .register_type::<SurvivorTemporaryBuff>() 
            .register_type::<ItemEffect>() 
            .register_type::<ItemLibrary>() 
            .register_type::<ExplosionEffect>() 
            .register_type::<RetaliationNovaEffect>() 
            .register_type::<TemporaryHealthRegenBuff>() 
            .init_resource::<ItemLibrary>() 
            .init_resource::<WeaponLibrary>() 
            .add_systems(Startup, (populate_item_library, populate_weapon_library)) 
            .add_systems(Update, ( 
                apply_collected_item_effects_system.run_if(on_event::<ItemCollectedEvent>()), 
                explosion_effect_system.run_if(in_state(AppState::InGame)), 
                retaliation_nova_effect_system.run_if(in_state(AppState::InGame)), 
                temporary_health_regen_buff_system.run_if(in_state(AppState::InGame)), 
            ));
    }
}

fn populate_weapon_library(mut library: ResMut<WeaponLibrary>) {
    library.weapons.push(WeaponDefinition {
        id: WeaponId(1), 
        name: "Ichor Dart Thrower".to_string(),
        projectile_sprite_path: "sprites/ichor_blast_placeholder.png".to_string(),
        base_damage: 10, 
        base_speed: 600.0, 
        base_lifetime_secs: 2.0, 
        base_piercing: 0,
        num_projectiles_at_once: 1, 
        projectile_spread_angle_degrees: 10.0, 
        projectile_size: crate::ichor_blast::ICHOR_BLAST_SIZE,
        projectile_color: Color::rgb(0.7, 0.5, 1.0),
    });
    library.weapons.push(WeaponDefinition { 
        id: WeaponId(2),
        name: "Void Ripper".to_string(),
        projectile_sprite_path: "sprites/void_ripper_projectile_placeholder.png".to_string(), 
        base_damage: 25,
        base_speed: 450.0, 
        base_lifetime_secs: 2.5,
        base_piercing: 1,
        num_projectiles_at_once: 1,
        projectile_spread_angle_degrees: 0.0, 
        projectile_size: Vec2::new(15.0, 30.0), 
        projectile_color: Color::rgb(0.3, 0.0, 0.5), 
    });
    library.weapons.push(WeaponDefinition {
        id: WeaponId(3),
        name: "Kinetic Pulse Driver".to_string(),
        projectile_sprite_path: "sprites/kinetic_pulse_placeholder.png".to_string(),
        base_damage: 35,
        base_speed: 700.0,
        base_lifetime_secs: 1.5,
        base_piercing: 0,
        num_projectiles_at_once: 1,
        projectile_spread_angle_degrees: 0.0,
        projectile_size: Vec2::new(20.0, 20.0),
        projectile_color: Color::rgb(0.2, 0.8, 1.0),
    });
    library.weapons.push(WeaponDefinition { 
        id: WeaponId(4),
        name: "Eldritch Scattergun".to_string(),
        projectile_sprite_path: "sprites/scatter_shard_placeholder.png".to_string(), 
        base_damage: 7,  
        base_speed: 500.0, 
        base_lifetime_secs: 0.8, 
        base_piercing: 0,
        num_projectiles_at_once: 5, 
        projectile_spread_angle_degrees: 30.0, 
        projectile_size: Vec2::new(8.0, 8.0), 
        projectile_color: Color::rgb(0.9, 0.4, 0.2), 
    });
    library.weapons.push(WeaponDefinition { 
        id: WeaponId(5),
        name: "Cryo Javelin".to_string(),
        projectile_sprite_path: "sprites/cryo_javelin_placeholder.png".to_string(), 
        base_damage: 20, 
        base_speed: 350.0, 
        base_lifetime_secs: 3.0, 
        base_piercing: 2,   
        num_projectiles_at_once: 1,
        projectile_spread_angle_degrees: 0.0,
        projectile_size: Vec2::new(12.0, 40.0), 
        projectile_color: Color::rgb(0.4, 0.7, 0.9), 
    });
}

fn populate_item_library(mut library: ResMut<ItemLibrary>) {
    library.items.push(ItemDefinition { id: ItemId(1), name: "Corrupted Heart".to_string(), description: "Increases Max Health by 25.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: Some(25), speed_multiplier: None, damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: None, }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(2), name: "Whispering Idol".to_string(), description: "Increases Movement Speed by 15%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: Some(1.15), damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: None, }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(3), name: "Shard of Agony".to_string(), description: "Increases automatic weapon damage by 5.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: Some(5), xp_gain_multiplier: None, pickup_radius_increase: None, }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(4), name: "Occult Tome Fragment".to_string(), description: "Increases XP gain by 20%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: None, xp_gain_multiplier: Some(1.20), pickup_radius_increase: None, }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(5), name: "Grasping Tentacle (Dried)".to_string(), description: "Increases pickup radius by 25%.".to_string(), effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: None, speed_multiplier: None, damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: Some(0.25), }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(6), name: "Fragmented Sanity".to_string(), description: "Your automatic attacks have a 15% chance to violently detonate on impact.".to_string(), effects: vec![ItemEffect::OnIchorBlastHitExplode { chance: 0.15, explosion_damage: 20, explosion_radius: 75.0, explosion_color: Color::rgba(1.0, 0.5, 0.2, 0.6), }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(7), name: "Cloak of VengefulSpirits".to_string(), description: "When struck, has a 25% chance to unleash a damaging psychic nova.".to_string(), effects: vec![ItemEffect::OnSurvivorHitRetaliate { chance: 0.25, retaliation_damage: 30, retaliation_radius: 120.0, retaliation_color: Color::rgba(0.9, 0.1, 0.1, 0.5), }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(8), name: "Soul Siphon Shard".to_string(), description: "Defeated foes have a 20% chance to grant brief, rapid health regeneration.".to_string(), effects: vec![ItemEffect::OnHorrorKillTrigger { chance: 0.20, effect: SurvivorTemporaryBuff::HealthRegen { rate: 5.0, duration_secs: 3.0 }, }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { id: ItemId(9), name: "Tome of Forbidden Rites".to_string(), description: "Grants knowledge of the 'Void Lance' skill.".to_string(), effects: vec![ItemEffect::GrantSpecificSkill { skill_id: SkillId(3) }], category: Some(ItemCategory::Relic) });
    library.items.push(ItemDefinition { 
        id: ItemId(10), 
        name: "Warding Totem".to_string(), 
        description: "A crumbling totem that pulses with protective energy, forming a Circle of Warding around you.".to_string(), 
        effects: vec![ItemEffect::ActivateCircleOfWarding], 
        category: Some(ItemCategory::Relic), 
    });
    library.items.push(ItemDefinition { 
        id: ItemId(11), 
        name: "Idol of the Swarm".to_string(), 
        description: "A pulsating idol that births Nightmare Larva to defend you.".to_string(), 
        effects: vec![ItemEffect::ActivateSwarmOfNightmares], 
        category: Some(ItemCategory::Relic), 
    });
    library.items.push(ItemDefinition {
        id: ItemId(12),
        name: "Ring of Minor Resilience".to_string(),
        description: "A simple iron band that slightly bolsters vitality. +10 Max Health.".to_string(),
        effects: vec![ItemEffect::PassiveStatBoost { max_health_increase: Some(10), speed_multiplier: None, damage_increase: None, xp_gain_multiplier: None, pickup_radius_increase: None, }],
        category: Some(ItemCategory::Ring),
    });
    library.items.push(ItemDefinition { 
        id: ItemId(13),
        name: "Whispering Band of Doom".to_string(),
        description: "This unsettling ring softly pulses with a faint, unnerving energy, harming nearby foes.".to_string(),
        effects: vec![ItemEffect::ActivateDoomPulse],
        category: Some(ItemCategory::Ring),
    });
}

fn apply_collected_item_effects_system( 
    mut commands: Commands, 
    mut events: EventReader<ItemCollectedEvent>, 
    mut player_query: Query<(Entity, &mut Survivor, Option<&mut ComponentHealth>, Option<&CircleOfWarding>, Option<&SwarmOfNightmares>, Option<&DoomPulseAura>)>, 
    item_library: Res<ItemLibrary>, 
    skill_library: Res<SkillLibrary>,
) { 
    if let Ok((player_entity, mut player, mut opt_health_component, opt_circle_aura_check, opt_swarm_check, opt_doom_pulse_check)) = player_query.get_single_mut() {
        for event in events.read() {
            let item_id = event.0; 
            
            let already_has_item_and_active_auras = if player.collected_item_ids.contains(&item_id) {
                let mut all_relevant_auras_active = true;
                if let Some(item_def_check) = item_library.get_item_definition(item_id) {
                    for effect_check in &item_def_check.effects {
                        match effect_check {
                            ItemEffect::ActivateCircleOfWarding => {
                                if opt_circle_aura_check.is_none() { 
                                    all_relevant_auras_active = false; break;
                                }
                            },
                            ItemEffect::ActivateSwarmOfNightmares => {
                                 if opt_swarm_check.is_none() { 
                                    all_relevant_auras_active = false; break;
                                }
                            },
                             ItemEffect::ActivateDoomPulse => {
                                if opt_doom_pulse_check.is_none() {
                                    all_relevant_auras_active = false; break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                all_relevant_auras_active 
            } else {
                false 
            };

            if already_has_item_and_active_auras { continue; }
            
            if let Some(item_def) = item_library.get_item_definition(item_id) {
                let is_newly_collected = !player.collected_item_ids.contains(&item_id);
                if is_newly_collected {
                    player.collected_item_ids.push(item_id);
                    if let Some(ItemCategory::Ring) = item_def.category {
                        let mut equipped = false;
                        for slot in player.equipped_rings.iter_mut() {
                            if slot.is_none() {
                                *slot = Some(item_id);
                                println!("Equipped Ring: {}", item_def.name);
                                equipped = true;
                                break;
                            }
                        }
                        if !equipped {
                            println!("No empty ring slots for {}.", item_def.name);
                        }
                    }
                }
                
                for effect in &item_def.effects {
                    match effect {
                        ItemEffect::PassiveStatBoost { max_health_increase, speed_multiplier, damage_increase, xp_gain_multiplier, pickup_radius_increase, } => {
                            if is_newly_collected { 
                                if let Some(hp_boost) = max_health_increase { player.max_health += *hp_boost; if let Some(ref mut health_comp) = opt_health_component { health_comp.0 += *hp_boost; health_comp.0 = health_comp.0.min(player.max_health); } }
                                if let Some(speed_mult) = speed_multiplier { player.speed *= *speed_mult; }
                                if let Some(dmg_inc) = damage_increase { player.automatic_weapon_damage_bonus += *dmg_inc; } 
                                if let Some(xp_mult) = xp_gain_multiplier { player.xp_gain_multiplier *= *xp_mult; }
                                if let Some(radius_inc_percent) = pickup_radius_increase { player.pickup_radius_multiplier *= 1.0 + radius_inc_percent; }
                            }
                        }
                        ItemEffect::GrantSpecificSkill { skill_id } => {
                            if is_newly_collected { 
                                if let Some(skill_to_grant_def) = skill_library.get_skill_definition(*skill_id) { 
                                    let already_has_skill = player.equipped_skills.iter().any(|s| s.definition_id == *skill_id);
                                    if !already_has_skill { if player.equipped_skills.len() < 5 { 
                                        player.equipped_skills.push(ActiveSkillInstance::new(*skill_id, skill_to_grant_def.base_glyph_slots)); 
                                    } }
                                }
                            }
                        }
                        ItemEffect::ActivateCircleOfWarding => {
                            if opt_circle_aura_check.is_none() {
                                commands.entity(player_entity).insert(CircleOfWarding::default());
                            }
                        }
                        ItemEffect::ActivateSwarmOfNightmares => {
                             if opt_swarm_check.is_none() {
                                commands.entity(player_entity).insert(SwarmOfNightmares {
                                    is_active: true, 
                                    num_larvae: 2, 
                                    ..default()
                                });
                            }
                        }
                        ItemEffect::ActivateDoomPulse => {
                            if opt_doom_pulse_check.is_none() {
                                commands.entity(player_entity).insert(DoomPulseAura::default());
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