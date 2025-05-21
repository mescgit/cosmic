use bevy::prelude::*;

mod survivor; 
mod components;
mod horror; 
mod ichor_blast; 
mod game;
mod echoing_soul; 
mod upgrades;
mod level_event_effects;
mod weapons;
mod visual_effects;
mod audio;
mod camera_systems;
mod background;
mod debug_menu;
mod skills;
mod items;
mod glyphs;

use survivor::SurvivorPlugin; 
use horror::HorrorPlugin; 
use ichor_blast::IchorBlastPlugin; 
use game::{GamePlugin, SCREEN_WIDTH, SCREEN_HEIGHT};
use level_event_effects::LevelEventEffectsPlugin;
use weapons::WeaponsPlugin; 
use visual_effects::VisualEffectsPlugin;
use audio::GameAudioPlugin;
use camera_systems::CameraSystemsPlugin; 
use background::BackgroundPlugin;
use skills::SkillsPlugin;
use items::ItemsPlugin; 
use glyphs::GlyphsPlugin;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Echoes of the Abyss".into(),
                resolution: (SCREEN_WIDTH, SCREEN_HEIGHT).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            GamePlugin, 
            SurvivorPlugin, 
            HorrorPlugin, 
            IchorBlastPlugin,
            LevelEventEffectsPlugin, 
            WeaponsPlugin, 
            VisualEffectsPlugin,
            GameAudioPlugin, 
            CameraSystemsPlugin, 
            BackgroundPlugin,
            SkillsPlugin, 
            ItemsPlugin, 
            GlyphsPlugin,
        ))
        .register_type::<weapons::DoomPulseAura>() 
        .add_systems(Startup, setup_global_camera)
        .run();
}

fn setup_global_camera(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.transform.translation.z = 999.0; 
    commands.spawn((camera_bundle, crate::camera_systems::MainCamera)); 
}