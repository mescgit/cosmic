use bevy::prelude::*;

#[derive(Event)]
pub struct PlaySoundEvent(pub SoundEffect);

#[derive(Debug, Clone, Copy)]
pub enum SoundEffect {
    SurvivorHit,
    HorrorDeath,
    HorrorProjectile,
    SoulCollect,
    Revelation,     
    OmenAccepted,   
    RitualCast,     
    AuraPulse,      // Added
    OrganicHit,     // Added
    MadnessConsumes,// Added
    // Add more specific sound effects as needed
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlaySoundEvent>()
            .add_systems(Update, play_sound_system);
    }
}

fn play_sound_system(
    mut R_sound_event: EventReader<PlaySoundEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for event in R_sound_event.read() {
        let sound_effect = match event.0 {
            SoundEffect::SurvivorHit => "audio/survivor_hit_placeholder.ogg",
            SoundEffect::HorrorDeath => "audio/horror_death_placeholder.ogg",
            SoundEffect::HorrorProjectile => "audio/horror_projectile_placeholder.ogg",
            SoundEffect::SoulCollect => "audio/soul_collect_placeholder.ogg",
            SoundEffect::Revelation => "audio/revelation_placeholder.ogg",
            SoundEffect::OmenAccepted => "audio/omen_accepted_placeholder.ogg",
            SoundEffect::RitualCast => "audio/ritual_cast_placeholder.ogg",
            SoundEffect::AuraPulse => "audio/aura_pulse_placeholder.ogg", 
            SoundEffect::OrganicHit => "audio/organic_hit_placeholder.ogg", 
            SoundEffect::MadnessConsumes => "audio/madness_consumes_placeholder.ogg",
        };
        commands.spawn(AudioBundle {
            source: asset_server.load(sound_effect),
            settings: PlaybackSettings::DESPAWN, 
        });
    }
}