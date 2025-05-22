// src/upgrades.rs
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::{
    skills::{SkillId, ActiveSkillInstance}, 
    survivor::MAX_ACTIVE_SKILLS, 
    items::AutomaticWeaponId,
};

#[derive(Debug, Clone, PartialEq)]
pub enum UpgradeType {
    // Survivor Stats
    SurvivorSpeed(u32), 
    MaxEndurance(i32), 
    EnduranceRegeneration(f32),
    EchoesGainMultiplier(u32), 
    SoulAttractionRadius(u32), 

    // Automatic Weapon Upgrades
    IncreaseAutoWeaponDamage(i32),          
    IncreaseAutoWeaponFireRate(u32),      
    IncreaseAutoWeaponProjectileSpeed(u32), 
    IncreaseAutoWeaponPiercing(u32),      
    IncreaseAutoWeaponProjectiles(u32),   
    IncreaseAutoWeaponChains(u32),
    IncreaseAutoWeaponChainRangePercent(u32),

    // Circle of Warding
    InscribeCircleOfWarding,
    IncreaseCircleRadius(u32), 
    IncreaseCircleDamage(i32), 
    DecreaseCircleTickRate(u32), 
    
    // Swarm of Nightmares
    ManifestSwarmOfNightmares, 
    IncreaseNightmareCount(u32), 
    IncreaseNightmareDamage(i32), 
    IncreaseNightmareRadius(f32), 
    IncreaseNightmareRotationSpeed(f32),
    
    // Active Skill Upgrades
    IncreaseSkillDamage { slot_index: usize, amount: i32 }, 
    ReduceSkillCooldown { slot_index: usize, percent_reduction: f32 }, 
    IncreaseSkillAoERadius { slot_index: usize, percent_increase: f32 }, // For skills with inherent AoE
    AddSkillImpactAoE { slot_index: usize, radius: f32, damage_fraction: f32 }, // New for projectile impact AoE
    IncreaseSkillPiercing { slot_index: usize, amount: u32 }, // New for projectile piercing
    
    // Utility/Granting
    GrantRandomRelic, 
    GrantSkill(SkillId),
}

#[derive(Debug, Clone)]
pub struct UpgradeCard { pub id: UpgradeId, pub name: String, pub description: String, pub upgrade_type: UpgradeType, }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpgradeId(pub u32);

#[derive(Resource, Default)]
pub struct UpgradePool { pub available_upgrades: Vec<UpgradeCard>, }

pub struct SurvivorUpgradeContext<'a> {
    pub is_circle_of_warding_active: bool,
    pub is_swarm_of_nightmares_active: bool,
    pub equipped_skills: &'a Vec<ActiveSkillInstance>,
    pub current_weapon_id: Option<AutomaticWeaponId>, 
}

impl UpgradePool {
    pub fn initialize(&mut self) {
        self.available_upgrades = vec![
            // ... (Keep existing stat, auto-weapon, aura, swarm upgrades as before) ...
            // Survivor Stats
            UpgradeCard {id: UpgradeId(0), name: "Borrowed Swiftness".to_string(), description: "Your limbs move with uncanny swiftness borrowed from beyond. +10% speed.".to_string(), upgrade_type: UpgradeType::SurvivorSpeed(10),},
            UpgradeCard {id: UpgradeId(1), name: "Flesh-Bound Pact".to_string(), description: "A pact seals your flesh against oblivion. +20 Max Endurance.".to_string(), upgrade_type: UpgradeType::MaxEndurance(20),},
            UpgradeCard {id: UpgradeId(5), name: "Otherworldly Agility".to_string(), description: "You glide like a creature not of this realm. +15% speed.".to_string(), upgrade_type: UpgradeType::SurvivorSpeed(15),},
            UpgradeCard {id: UpgradeId(6), name: "Resilient Corpus".to_string(), description: "Your form knits itself against harsher realities. +30 Max Endurance.".to_string(), upgrade_type: UpgradeType::MaxEndurance(30),},
            UpgradeCard {id: UpgradeId(300), name: "Unnatural Vigor".to_string(), description: "Reality warps to mend your wounds. Regenerate 0.5 Endurance/sec.".to_string(), upgrade_type: UpgradeType::EnduranceRegeneration(0.5),},
            UpgradeCard {id: UpgradeId(301), name: "Bound by Ichor".to_string(), description: "Strange energies sustain your form. Regenerate 1.0 Endurance/sec.".to_string(), upgrade_type: UpgradeType::EnduranceRegeneration(1.0),},

            // Automatic Weapon (Main Attack)
            UpgradeCard {id: UpgradeId(2), name: "Maddening Focus".to_string(), description: "Your automatic attacks strike with greater force. +5 Damage.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponDamage(5),},
            UpgradeCard {id: UpgradeId(3), name: "Rapid Sanity Strain".to_string(), description: "Your mind strains faster, casting automatic attacks more quickly. +15% fire rate.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponFireRate(15),},
            UpgradeCard {id: UpgradeId(4), name: "Swift Projectiles".to_string(), description: "Your automatic projectiles travel faster. +20% velocity.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponProjectileSpeed(20),},
            UpgradeCard {id: UpgradeId(7), name: "Piercing Thoughts".to_string(), description: "Your automatic attacks carry deeper malevolence. +8 Damage.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponDamage(8),},
            UpgradeCard {id: UpgradeId(8), name: "Hyper Reflex".to_string(), description: "Your mind strains with startling alacrity, casting automatic attacks faster. +20% fire rate.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponFireRate(20),},
            UpgradeCard {id: UpgradeId(9), name: "Unraveling Force".to_string(), description: "Your automatic projectiles tear through more horrors. Pierce +1 horror.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponPiercing(1),},
            UpgradeCard {id: UpgradeId(12), name: "Persistent Dread".to_string(), description: "Your automatic projectiles linger longer in reality. Pierce +2 horrors.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponPiercing(2),},
            UpgradeCard {id: UpgradeId(200), name: "Fractured Consciousness".to_string(), description: "Your mind splinters, projecting an additional automatic projectile. +1 Projectile.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponProjectiles(1),},
            UpgradeCard {id: UpgradeId(201), name: "Projectile Barrage".to_string(), description: "Your consciousness erupts, projecting two additional automatic projectiles. +2 Projectiles.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponProjectiles(2),},
            UpgradeCard {id: UpgradeId(202), name: "Forked Lightning".to_string(), description: "Your automatic attacks chain to an additional nearby enemy. +1 Chain.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponChains(1),}, 
            UpgradeCard {id: UpgradeId(203), name: "Storm Conduit".to_string(), description: "Your automatic attacks chain to 2 additional nearby enemies. +2 Chains.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponChains(2),}, 
            UpgradeCard {id: UpgradeId(204), name: "Reaching Tendrils".to_string(), description: "Your chain lightning arcs to more distant foes. +25% Chain Range.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponChainRangePercent(25),}, 
            UpgradeCard {id: UpgradeId(205), name: "Voltaic Grasp".to_string(), description: "Your chain lightning arcs significantly further. +50% Chain Range.".to_string(), upgrade_type: UpgradeType::IncreaseAutoWeaponChainRangePercent(50),}, 


            // Echoes (XP) & Pickups
            UpgradeCard {id: UpgradeId(10), name: "Glimpse Beyond The Veil".to_string(), description: "Glimpses of the abyss accelerate your horrific understanding. +20% Echoes gain.".to_string(), upgrade_type: UpgradeType::EchoesGainMultiplier(20),},
            UpgradeCard {id: UpgradeId(11), name: "Soul Grasp".to_string(), description: "The echoes of fallen horrors are drawn to you. +25% Echoing Soul attraction radius.".to_string(), upgrade_type: UpgradeType::SoulAttractionRadius(25),},
            UpgradeCard {id: UpgradeId(13), name: "Abyssal Understanding".to_string(), description: "You perceive deeper truths, hastening your evolution. +30% Echoes gain.".to_string(), upgrade_type: UpgradeType::EchoesGainMultiplier(30),},
            
            // Circle of Warding (Aura Weapon)
            UpgradeCard {id: UpgradeId(100), name: "Inscribe Circle of Warding".to_string(), description: "Manifest an aura of protective, damaging glyphs.".to_string(), upgrade_type: UpgradeType::InscribeCircleOfWarding,},
            UpgradeCard {id: UpgradeId(101), name: "Echoing Wards".to_string(), description: "Your protective circle extends further. +20% circle radius.".to_string(), upgrade_type: UpgradeType::IncreaseCircleRadius(20),},
            UpgradeCard {id: UpgradeId(102), name: "Maddening Wards".to_string(), description: "Your circle inflicts greater mental anguish. +2 circle damage.".to_string(), upgrade_type: UpgradeType::IncreaseCircleDamage(2),},
            UpgradeCard {id: UpgradeId(103), name: "Frenzied Wards".to_string(), description: "Your circle pulses with greater frequency. Circle damages 15% faster.".to_string(), upgrade_type: UpgradeType::DecreaseCircleTickRate(15),},

            // Swarm of Nightmares (Orbiter Weapon)
            UpgradeCard {id: UpgradeId(400), name: "Manifest Swarm of Nightmares".to_string(), description: "Conjure 2 nightmare larva that orbit and attack foes.".to_string(), upgrade_type: UpgradeType::ManifestSwarmOfNightmares,},
            UpgradeCard {id: UpgradeId(401), name: "Grow the Nightmare Swarm".to_string(), description: "Add another Nightmare Larva to your psychic defenses. +1 nightmare.".to_string(), upgrade_type: UpgradeType::IncreaseNightmareCount(1),},
            UpgradeCard {id: UpgradeId(402), name: "Venomous Nightmares".to_string(), description: "Your Nightmare Larva inflict deeper wounds. +3 nightmare damage.".to_string(), upgrade_type: UpgradeType::IncreaseNightmareDamage(3),},
            UpgradeCard {id: UpgradeId(403), name: "Extended Nightmare Patrol".to_string(), description: "Your Nightmare Larva patrol a wider area. +15 orbit radius.".to_string(), upgrade_type: UpgradeType::IncreaseNightmareRadius(15.0),},
            UpgradeCard {id: UpgradeId(404), name: "Swifter Nightmares".to_string(), description: "Your Nightmare Larva move with increased speed. +0.5 rad/s orbit speed.".to_string(), upgrade_type: UpgradeType::IncreaseNightmareRotationSpeed(0.5),},
            
            // Skill Specific Upgrades - Damage
            UpgradeCard {id: UpgradeId(500), name: "Empower Skill 1".to_string(), description: "Increase damage of Skill in Slot 1 by 10.".to_string(), upgrade_type: UpgradeType::IncreaseSkillDamage { slot_index: 0, amount: 10 },},
            // ... (other skill damage upgrades 501-504) ...

            // Skill Specific Upgrades - Piercing (NEW)
            UpgradeCard {id: UpgradeId(510), name: "Penetrating Bolt".to_string(), description: "Skill in Slot 1 (Eldritch Bolt) pierces +1 enemy.".to_string(), upgrade_type: UpgradeType::IncreaseSkillPiercing { slot_index: 0, amount: 1 },},
            UpgradeCard {id: UpgradeId(511), name: "Armor-Shattering Bolt".to_string(), description: "Skill in Slot 1 (Eldritch Bolt) pierces +2 enemies.".to_string(), upgrade_type: UpgradeType::IncreaseSkillPiercing { slot_index: 0, amount: 2 },},
            // (Could add for Void Lance - SkillId(3) - if it's in slot_index 2)
            UpgradeCard {id: UpgradeId(512), name: "Lance of Ruin".to_string(), description: "Skill in Slot 3 (Void Lance) pierces +1 additional enemy.".to_string(), upgrade_type: UpgradeType::IncreaseSkillPiercing{slot_index: 2, amount: 1},},


            // Skill Specific Upgrades - Impact AoE (NEW)
            UpgradeCard {id: UpgradeId(520), name: "Unstable Bolt".to_string(), description: "Skill in Slot 1 (Eldritch Bolt) explodes on impact for 30% damage in a small area (50 radius).".to_string(), upgrade_type: UpgradeType::AddSkillImpactAoE { slot_index: 0, radius: 50.0, damage_fraction: 0.30 },},
            UpgradeCard {id: UpgradeId(521), name: "Volatile Bolt".to_string(), description: "Skill in Slot 1 (Eldritch Bolt) explodes with greater force (75 radius, 50% damage).".to_string(), upgrade_type: UpgradeType::AddSkillImpactAoE { slot_index: 0, radius: 75.0, damage_fraction: 0.50 },},
            // (Could add for Void Lance - SkillId(3) - if it's in slot_index 2)
            UpgradeCard {id: UpgradeId(522), name: "Void Detonation".to_string(), description: "Skill in Slot 3 (Void Lance) erupts on final impact (60 radius, 40% damage).".to_string(), upgrade_type: UpgradeType::AddSkillImpactAoE{slot_index: 2, radius: 60.0, damage_fraction: 0.40},},
            
            // General/Utility
            UpgradeCard {id: UpgradeId(600), name: "Mysterious Relic".to_string(), description: "The abyss grants you a random relic.".to_string(), upgrade_type: UpgradeType::GrantRandomRelic,},

            // Grant Skills
            // ... (keep existing grant skill upgrades 700-705) ...
            UpgradeCard {id: UpgradeId(700), name: "Learn: Mind Shatter".to_string(), description: "Unlock the Mind Shatter psychic burst skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(2)),},
            UpgradeCard {id: UpgradeId(701), name: "Learn: Void Lance".to_string(), description: "Unlock the Void Lance piercing projectile skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(3)),},
            UpgradeCard {id: UpgradeId(702), name: "Learn: Fleeting Agility".to_string(), description: "Unlock the Fleeting Agility self-buff skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(4)),},
            UpgradeCard {id: UpgradeId(703), name: "Learn: Glacial Nova".to_string(), description: "Unlock the Glacial Nova chilling skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(5)),},
            UpgradeCard {id: UpgradeId(704), name: "Learn: Psychic Sentry".to_string(), description: "Unlock the Psychic Sentry summon skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(6)),},
            UpgradeCard {id: UpgradeId(705), name: "Learn: Ethereal Ward".to_string(), description: "Unlock the Ethereal Ward defensive skill. (Requires free skill slot)".to_string(), upgrade_type: UpgradeType::GrantSkill(SkillId(7)),},


            // Skill Meta Upgrades (Cooldown & AoE)
            // ... (keep existing cooldown and general AoE upgrades 800-809) ...
            UpgradeCard {id: UpgradeId(800), name: "Quicken Skill 1".to_string(), description: "Skill in Slot 1 recharges 15% faster.".to_string(), upgrade_type: UpgradeType::ReduceSkillCooldown { slot_index: 0, percent_reduction: 0.15 },},
            UpgradeCard {id: UpgradeId(801), name: "Quicken Skill 2".to_string(), description: "Skill in Slot 2 recharges 15% faster.".to_string(), upgrade_type: UpgradeType::ReduceSkillCooldown { slot_index: 1, percent_reduction: 0.15 },}, 
            UpgradeCard {id: UpgradeId(802), name: "Quicken Skill 3".to_string(), description: "Skill in Slot 3 recharges 10% faster.".to_string(), upgrade_type: UpgradeType::ReduceSkillCooldown { slot_index: 2, percent_reduction: 0.10 },},
            UpgradeCard {id: UpgradeId(803), name: "Quicken Skill 4".to_string(), description: "Skill in Slot 4 recharges 10% faster.".to_string(), upgrade_type: UpgradeType::ReduceSkillCooldown { slot_index: 3, percent_reduction: 0.10 },},
            UpgradeCard {id: UpgradeId(804), name: "Quicken Skill 5".to_string(), description: "Skill in Slot 5 recharges 10% faster.".to_string(), upgrade_type: UpgradeType::ReduceSkillCooldown { slot_index: 4, percent_reduction: 0.10 },}, 
            
            UpgradeCard {id: UpgradeId(805), name: "Expand Skill 1 AoE".to_string(), description: "Skill in Slot 1's area of effect expands by 15%.".to_string(), upgrade_type: UpgradeType::IncreaseSkillAoERadius { slot_index: 0, percent_increase: 0.15 },},
            UpgradeCard {id: UpgradeId(806), name: "Expand Skill 2 AoE".to_string(), description: "Skill in Slot 2's area of effect expands by 15%.".to_string(), upgrade_type: UpgradeType::IncreaseSkillAoERadius { slot_index: 1, percent_increase: 0.15 },},
            UpgradeCard {id: UpgradeId(807), name: "Expand Skill 3 AoE".to_string(), description: "Skill in Slot 3's area of effect expands by 20%.".to_string(), upgrade_type: UpgradeType::IncreaseSkillAoERadius { slot_index: 2, percent_increase: 0.20 },},
            UpgradeCard {id: UpgradeId(808), name: "Expand Skill 4 AoE".to_string(), description: "Skill in Slot 4's area of effect expands by 10%.".to_string(), upgrade_type: UpgradeType::IncreaseSkillAoERadius { slot_index: 3, percent_increase: 0.10 },},
            UpgradeCard {id: UpgradeId(809), name: "Expand Skill 5 AoE".to_string(), description: "Skill in Slot 5's area of effect expands by 15%.".to_string(), upgrade_type: UpgradeType::IncreaseSkillAoERadius { slot_index: 4, percent_increase: 0.15 },},
        ];
    }

    pub fn get_random_upgrades(&self, count: usize, context: &SurvivorUpgradeContext) -> Vec<UpgradeCard> {
        let is_chain_weapon_equipped = context.current_weapon_id == Some(crate::items::AutomaticWeaponId(3));

        let filtered_upgrades: Vec<UpgradeCard> = self.available_upgrades.iter().filter(|card| {
            match &card.upgrade_type {
                UpgradeType::InscribeCircleOfWarding => !context.is_circle_of_warding_active,
                UpgradeType::IncreaseCircleRadius(_) | UpgradeType::IncreaseCircleDamage(_) | UpgradeType::DecreaseCircleTickRate(_) => {
                    context.is_circle_of_warding_active
                }
                UpgradeType::ManifestSwarmOfNightmares => !context.is_swarm_of_nightmares_active,
                UpgradeType::IncreaseNightmareCount(_) | UpgradeType::IncreaseNightmareDamage(_) | UpgradeType::IncreaseNightmareRadius(_) | UpgradeType::IncreaseNightmareRotationSpeed(_) => {
                    context.is_swarm_of_nightmares_active
                }
                UpgradeType::IncreaseSkillDamage { slot_index, .. } | 
                UpgradeType::ReduceSkillCooldown { slot_index, .. } | 
                UpgradeType::IncreaseSkillAoERadius { slot_index, .. } |
                UpgradeType::AddSkillImpactAoE { slot_index, ..} | // Check if skill exists for these new types
                UpgradeType::IncreaseSkillPiercing { slot_index, ..} => { // Check if skill exists
                    context.equipped_skills.get(*slot_index).is_some()
                }
                UpgradeType::GrantSkill(skill_id_to_grant) => {
                    context.equipped_skills.len() < MAX_ACTIVE_SKILLS &&
                    !context.equipped_skills.iter().any(|s| s.definition_id == *skill_id_to_grant)
                }
                UpgradeType::IncreaseAutoWeaponChains(_) => is_chain_weapon_equipped,
                UpgradeType::IncreaseAutoWeaponChainRangePercent(_) => is_chain_weapon_equipped, 
                _ => true, 
            }
        }).cloned().collect();

        let mut rng = rand::thread_rng();
        filtered_upgrades.choose_multiple(&mut rng, count).cloned().collect()
    }
}

#[derive(Component, Debug, Clone)] pub struct OfferedUpgrades { pub choices: Vec<UpgradeCard>, }
pub struct UpgradePlugin;
impl Plugin for UpgradePlugin { fn build(&self, app: &mut App) { let mut upgrade_pool = UpgradePool::default(); upgrade_pool.initialize(); app.insert_resource(upgrade_pool); } }