use bevy::prelude::*;
use std::collections::HashMap;

mod snake;
mod voxel;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameMode {
    #[default]
    Voxel,
    Snake,
}

fn get_starting_mode() -> GameMode {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(search) = location.search() {
                if search.contains("game=snake") {
                    return GameMode::Snake;
                }
            }
        }
    }
    GameMode::Voxel
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Web Arcade".into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameMode>()
        .insert_state(get_starting_mode())
        // Voxel Resources
        .init_resource::<voxel::CurrentBlock>()
        .insert_resource(voxel::VoxelWorld {
            blocks: HashMap::new(),
            changed: false,
        })
        // Shared Exit Cleanup systems
        .add_systems(OnExit(GameMode::Voxel), cleanup_state::<VoxelEntity>)
        .add_systems(OnExit(GameMode::Snake), cleanup_state::<SnakeEntity>)
        // Voxel Game Mode systems
        .add_systems(
            OnEnter(GameMode::Voxel),
            (voxel::setup_voxel, voxel::spawn_hud),
        )
        .add_systems(
            Update,
            (
                voxel::grab_mouse,
                voxel::player_look_system,
                voxel::player_move_system,
                voxel::handle_block_selection,
                voxel::handle_block_interaction,
                voxel::update_world_mesh_system,
            )
                .run_if(in_state(GameMode::Voxel)),
        )
        // Snake Game Mode systems
        .add_systems(
            OnEnter(GameMode::Snake),
            (snake::setup_snake, snake::spawn_snake_hud),
        )
        .add_systems(
            Update,
            (
                snake::snake_input_system,
                snake::snake_tick_system,
                snake::snake_camera_system,
                snake::snake_render_update_system,
                snake::snake_restart_system,
            )
                .run_if(in_state(GameMode::Snake)),
        )
        .run();
}

// Tag components to clean up entities on game mode switch
#[derive(Component)]
pub struct VoxelEntity;

#[derive(Component)]
pub struct SnakeEntity;

// Shared cleanup helper to clear state entities on transition
fn cleanup_state<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
