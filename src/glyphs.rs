// src/glyphs.rs
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct GlyphId(pub u32);

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum GlyphEffectType {
    ProjectileChain { bounces: u32, },
    IncreasedAoEDamage { percent_increase: f32, },
    AddedChaosDamageToProjectile { damage_amount: i32, },
    IncreaseBaseDamage { amount: i32 }, 
    IncreaseRate { percent_boost: f32 }, 
    IncreaseEffectScale { percent_boost: f32 }, // New effect for range/AoE size
}

#[derive(Debug, Clone, Reflect)]
pub struct GlyphDefinition {
    pub id: GlyphId,
    pub name: String,
    pub description: String,
    pub effect: GlyphEffectType,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct GlyphLibrary {
    pub glyphs: Vec<GlyphDefinition>,
}

impl GlyphLibrary {
    pub fn get_glyph_definition(&self, id: GlyphId) -> Option<&GlyphDefinition> {
        self.glyphs.iter().find(|def| def.id == id)
    }
}

pub struct GlyphsPlugin;

impl Plugin for GlyphsPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<GlyphId>()
            .register_type::<GlyphEffectType>()
            .register_type::<GlyphDefinition>()
            .register_type::<GlyphLibrary>()
            .init_resource::<GlyphLibrary>()
            .add_systems(Startup, populate_glyph_library);
    }
}

fn populate_glyph_library(mut library: ResMut<GlyphLibrary>) {
    library.glyphs.push(GlyphDefinition {
        id: GlyphId(1),
        name: "Glyph of Linked Nightmares".to_string(),
        description: "Your projectiles chain to 1 additional enemy.".to_string(),
        effect: GlyphEffectType::ProjectileChain { bounces: 1 },
    });
    library.glyphs.push(GlyphDefinition {
        id: GlyphId(2),
        name: "Glyph of Resonating Terror".to_string(),
        description: "Increases the damage of your area effects by 20%.".to_string(),
        effect: GlyphEffectType::IncreasedAoEDamage { percent_increase: 0.20 },
    });
    library.glyphs.push(GlyphDefinition {
        id: GlyphId(3),
        name: "Glyph of Abyssal Touch".to_string(),
        description: "Your projectiles deal an additional 10 chaos damage.".to_string(),
        effect: GlyphEffectType::AddedChaosDamageToProjectile { damage_amount: 10 },
    });
    library.glyphs.push(GlyphDefinition {
        id: GlyphId(4),
        name: "Glyph of Focused Intensity".to_string(),
        description: "Increases the direct damage of the modified ability or weapon by +5.".to_string(),
        effect: GlyphEffectType::IncreaseBaseDamage { amount: 5 },
    });
    library.glyphs.push(GlyphDefinition { 
        id: GlyphId(5),
        name: "Glyph of Swift Execution".to_string(),
        description: "Increases attack/casting speed by 10% (reduces cooldowns/fire intervals).".to_string(),
        effect: GlyphEffectType::IncreaseRate { percent_boost: 0.10 },
    });
    library.glyphs.push(GlyphDefinition { // New Glyph
        id: GlyphId(6),
        name: "Glyph of Eldritch Expansion".to_string(),
        description: "Increases the range (projectile lifetime) or area of effect by 15%.".to_string(),
        effect: GlyphEffectType::IncreaseEffectScale { percent_boost: 0.15 },
    });
}