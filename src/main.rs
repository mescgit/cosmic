// src/main.rs
use bevy::prelude::*;
use bevy_hanabi::HanabiPlugin; // Import HanabiPlugin

mod survivor;
mod components;
mod horror;
mod automatic_projectiles; 
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
// mod glyphs; // Commented out

use survivor::SurvivorPlugin;
use horror::HorrorPlugin;
use automatic_projectiles::AutomaticProjectilesPlugin; 
use game::{GamePlugin, SCREEN_WIDTH, SCREEN_HEIGHT};
use level_event_effects::LevelEventEffectsPlugin;
use weapons::WeaponsPlugin;
use visual_effects::VisualEffectsPlugin;
use audio::GameAudioPlugin;
use camera_systems::{CameraSystemsPlugin, MainCamera};
use background::BackgroundPlugin;
use skills::SkillsPlugin;
use items::{ItemsPlugin, AutomaticWeaponLibrary, AutomaticWeaponDefinition, AutomaticWeaponId};
// use glyphs::GlyphsPlugin; // Commented out


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Eldritch Hero".into(), // Updated game title
                resolution: (SCREEN_WIDTH, SCREEN_HEIGHT).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .register_type::<AutomaticWeaponId>()
        .register_type::<AutomaticWeaponDefinition>()
        .register_type::<AutomaticWeaponLibrary>()
        .add_plugins((
            GamePlugin,
            SurvivorPlugin,
            HorrorPlugin,
            AutomaticProjectilesPlugin, 
            LevelEventEffectsPlugin,
            WeaponsPlugin,
            VisualEffectsPlugin,
            GameAudioPlugin,
            CameraSystemsPlugin,
            BackgroundPlugin,
            SkillsPlugin,
            ItemsPlugin,
            // GlyphsPlugin, // Commented out
            HanabiPlugin, // Add the HanabiPlugin here
        ))
        .add_systems(Startup, setup_global_camera)
        .run();
}

fn setup_global_camera(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    // Ensure our 2D camera is suitable for 2D particle effects if bevy_hanabi requires specific camera setup.
    // For now, the default 2D camera should be okay.
    // Hanabi examples often show use with both 2D and 3D cameras.
    camera_bundle.transform.translation.z = 999.0; // Keep UI and other elements visible
    commands.spawn((camera_bundle, MainCamera));
}