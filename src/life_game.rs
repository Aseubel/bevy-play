use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource)]
pub struct LifeGrid {
    pub cells: HashSet<IVec3>,
}

#[derive(Resource)]
pub struct LifeTimer(pub Timer);

#[derive(Component)]
pub struct CellEntity {
    pub pos: IVec3,
}

#[derive(Component)]
pub struct LifeGameEntity;

#[derive(Component)]
pub struct LifeCamera;

#[derive(Component)]
pub struct LifeHud;

#[derive(Component)]
pub struct LifeCounter;

// Thread-safe random value generator compatible with both native and WebAssembly.
fn get_random_value() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Math::random() as f32
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEED: AtomicU64 = AtomicU64::new(135792468);
        let old = SEED.fetch_add(1, Ordering::Relaxed);
        let mut x = old.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d049bb133111eb);
        x ^= x >> 31;
        (x as f32) / (u64::MAX as f32)
    }
}

pub fn setup_life_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Configure Global Ambient Light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 450.0,
        ..default()
    });

    // 2. Spawn Directional Light
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_xyz(25.0, 45.0, 25.0).looking_at(Vec3::new(7.5, 7.5, 7.5), Vec3::Y),
        LifeGameEntity,
    ));

    // 3. Spawn 3D Orbital Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(16.0, 20.0, 36.0).looking_at(Vec3::new(7.5, 7.5, 7.5), Vec3::Y),
        LifeCamera,
        LifeGameEntity,
    ));

    // 4. Pre-spawn all 16x16x16 = 4096 Cell Entities with Visibility::Hidden
    // Use a slightly smaller cuboid size (0.8x0.8x0.8) to create clear visual spacing between cells
    let mesh_handle = meshes.add(Cuboid::new(0.8, 0.8, 0.8));
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.85, 1.0), // Vibrant technology cyan
        emissive: LinearRgba::rgb(0.0, 0.15, 0.2), // Subtle glow
        perceptual_roughness: 0.1,
        metallic: 0.7,
        ..default()
    });

    for x in 0..16 {
        for y in 0..16 {
            for z in 0..16 {
                let pos = IVec3::new(x, y, z);
                commands.spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(material_handle.clone()),
                    Transform::from_xyz(x as f32, y as f32, z as f32),
                    Visibility::Hidden,
                    CellEntity { pos },
                    LifeGameEntity,
                ));
            }
        }
    }

    // 5. Initialize the LifeGrid with a random 25% occupancy seed
    let mut initial_cells = HashSet::new();
    for x in 0..16 {
        for y in 0..16 {
            for z in 0..16 {
                if get_random_value() < 0.25 {
                    initial_cells.insert(IVec3::new(x, y, z));
                }
            }
        }
    }

    commands.insert_resource(LifeGrid { cells: initial_cells });
    commands.insert_resource(LifeTimer(Timer::from_seconds(0.2, TimerMode::Repeating)));

    // 6. Spawn the HUD
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        LifeHud,
        LifeGameEntity,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Active Cells: 0"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
            LifeCounter,
        ));
        parent.spawn((
            Text::new("Grid Size: 16 x 16 x 16"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
        ));
        parent.spawn((
            Button,
            Node {
                width: Val::Px(110.0),
                height: Val::Px(35.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|button_parent| {
            button_parent.spawn((
                Text::new("Replay (R)"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

pub fn cleanup_life_game(
    mut commands: Commands,
    query: Query<Entity, With<LifeGameEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LifeGrid>();
    commands.remove_resource::<LifeTimer>();
}

pub fn evolve_life_system(
    time: Res<Time>,
    mut timer: ResMut<LifeTimer>,
    mut grid: ResMut<LifeGrid>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut next_cells = HashSet::new();

        // Check each possible position in the 16x16x16 grid
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let pos = IVec3::new(x, y, z);
                    
                    // Count 26 neighbors
                    let mut alive_neighbors = 0;
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            for dz in -1..=1 {
                                if dx == 0 && dy == 0 && dz == 0 {
                                    continue;
                                }
                                let neighbor = pos + IVec3::new(dx, dy, dz);
                                if grid.cells.contains(&neighbor) {
                                    alive_neighbors += 1;
                                }
                            }
                        }
                    }

                    let is_alive = grid.cells.contains(&pos);
                    if is_alive {
                        // Survival rules: survives with 4 or 5 neighbors
                        if alive_neighbors == 4 || alive_neighbors == 5 {
                            next_cells.insert(pos);
                        }
                    } else {
                        // Birth rules: born with exactly 5 neighbors
                        if alive_neighbors == 5 {
                            next_cells.insert(pos);
                        }
                    }
                }
            }
        }

        grid.cells = next_cells;
    }
}

pub fn update_cell_visibility_system(
    grid: Res<LifeGrid>,
    mut query: Query<(&CellEntity, &mut Visibility)>,
) {
    // Only update visibility if the LifeGrid resource was modified (evolved)
    if grid.is_changed() {
        query.par_iter_mut().for_each(|(cell, mut visibility)| {
            if grid.cells.contains(&cell.pos) {
                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        });
    }
}

pub fn camera_spin_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<LifeCamera>>,
) {
    let speed = 0.12; // Slow elegant orbit rotation speed
    let elapsed = time.elapsed_secs() * speed;
    
    let center = Vec3::new(7.5, 7.5, 7.5);
    let radius = 26.0;
    
    // Orbit equations around the center of the grid
    let x = center.x + radius * elapsed.cos();
    let z = center.z + radius * elapsed.sin();
    let y = center.y + 8.0 + (elapsed * 0.4).sin() * 4.0; // gentle vertical wave
    
    for mut transform in &mut query {
        *transform = Transform::from_xyz(x, y, z).looking_at(center, Vec3::Y);
    }
}

pub struct LifeGamePlugin;

pub fn update_hud_system(
    grid: Res<LifeGrid>,
    mut query: Query<&mut Text, With<LifeCounter>>,
) {
    let count = grid.cells.len();
    for mut text in &mut query {
        *text = Text::new(format!("Active Cells: {}", count));
    }
}

pub fn life_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<LifeGrid>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    let mut should_reset = keys.just_pressed(KeyCode::KeyR);

    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));
                should_reset = true;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
        }
    }

    if should_reset {
        let mut next_cells = HashSet::new();
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    if get_random_value() < 0.25 {
                        next_cells.insert(IVec3::new(x, y, z));
                    }
                }
            }
        }
        grid.cells = next_cells;
    }
}

impl Plugin for LifeGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::GameMode::LifeGame),
            setup_life_game,
        )
        .add_systems(
            OnExit(crate::GameMode::LifeGame),
            cleanup_life_game,
        )
        .add_systems(
            Update,
            (
                evolve_life_system,
                update_cell_visibility_system,
                camera_spin_system,
                update_hud_system,
                life_input_system,
            )
                .run_if(in_state(crate::GameMode::LifeGame)),
        );
    }
}
