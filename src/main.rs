// src/main.rs
use bevy::prelude::*;

mod survivor;
mod components;
mod horror;
// mod automatic_projectiles; // Removed
mod game;
// mod echoing_soul; // Removed
// mod upgrades; // Removed
// mod level_event_effects; // Removed
// mod weapons; // Removed
mod visual_effects;
mod audio;
mod camera_systems;
mod background;
mod debug_menu; // Kept for now
// mod skills; // Removed
// mod items; // Removed
// mod glyphs; // Removed

use survivor::SurvivorPlugin;
use horror::HorrorPlugin; // Kept
// use automatic_projectiles::AutomaticProjectilesPlugin; // Removed
// use level_event_effects::LevelEventEffectsPlugin; // Removed
// use weapons::WeaponsPlugin; // Removed
// use skills::SkillsPlugin; // Removed
// use items::{ItemsPlugin, AutomaticWeaponLibrary, AutomaticWeaponDefinition, AutomaticWeaponId}; // Removed
// use echoing_soul::EchoingSoulPlugin; // Removed
// Ensure no glyphs related use statements are present
use game::{GamePlugin, SCREEN_WIDTH, SCREEN_HEIGHT};
use visual_effects::VisualEffectsPlugin;
use audio::GameAudioPlugin;
use camera_systems::{CameraSystemsPlugin, MainCamera};
use background::BackgroundPlugin;
// use debug_menu::DebugMenuPlugin; // Kept for now


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
        // All .register_type calls for AutomaticWeaponId, AutomaticWeaponDefinition, AutomaticWeaponLibrary removed
        .add_plugins((
            GamePlugin,
            SurvivorPlugin,
            HorrorPlugin, // Kept
            // AutomaticProjectilesPlugin, // Removed
            // LevelEventEffectsPlugin, // Removed
            // WeaponsPlugin, // Removed
            // SkillsPlugin, // Removed
            // ItemsPlugin, // Removed
            // EchoingSoulPlugin, // Removed
            // GlyphsPlugin, // Removed
            VisualEffectsPlugin,
            GameAudioPlugin,
            CameraSystemsPlugin,
            BackgroundPlugin,
            // DebugMenuPlugin, // Kept for now
        ))
        .add_systems(Startup, setup_global_camera)
        .run();
}

fn setup_global_camera(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.transform.translation.z = 999.0;
    commands.spawn((camera_bundle, MainCamera));
}