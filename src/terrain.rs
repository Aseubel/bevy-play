use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;

#[derive(Resource)]
pub struct TerrainSeed(pub f32);

#[derive(Component)]
pub struct TerrainMeshMarker;

#[derive(Component)]
pub struct TerrainCleanup;

#[derive(Component)]
pub struct TerrainCamera;

// Thread-safe random value generator compatible with both native and WebAssembly.
fn get_random_value() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Math::random() as f32
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEED: AtomicU64 = AtomicU64::new(876543210);
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

// Multi-layered pseudo-random sine/cosine noise function to simulate Perlin noise.
fn get_height(x: f32, z: f32, seed: f32) -> f32 {
    // Scale horizontal frequency based on 64x64 grid dimensions
    let nx = x * 0.08 + seed;
    let nz = z * 0.08 + seed * 1.618;
    
    let h1 = (nx.sin() + nz.cos()) * 4.5;
    let h2 = ((nx * 2.1).sin() + (nz * 2.1).cos()) * 2.2;
    let h3 = ((nx * 4.7).sin() + (nz * 4.7).cos()) * 0.9;
    let h4 = ((nx * 9.3).sin() + (nz * 9.3).cos()) * 0.3;
    
    h1 + h2 + h3 + h4
}

// Map heights to nice low-poly colors (Water -> Beach -> Grass -> Rock -> Snow)
fn get_color(y: f32) -> [f32; 4] {
    if y < -1.5 {
        // Deep water to shallow water transition
        let t = ((y + 4.0) / 2.5).clamp(0.0, 1.0);
        [0.05, 0.15 + 0.45 * t, 0.45 + 0.35 * t, 1.0]
    } else if y < -0.8 {
        // Beach sand
        let t = ((y + 1.5) / 0.7).clamp(0.0, 1.0);
        [0.0 + 0.85 * t, 0.6 + 0.2 * t, 0.8 - 0.2 * t, 1.0]
    } else if y < 2.0 {
        // Lush grass
        let t = ((y + 0.8) / 2.8).clamp(0.0, 1.0);
        [0.85 - 0.6 * t, 0.8 - 0.15 * t, 0.6 - 0.32 * t, 1.0]
    } else if y < 4.5 {
        // Rocky mountainside
        let t = ((y - 2.0) / 2.5).clamp(0.0, 1.0);
        [0.25 + 0.2 * t, 0.65 - 0.2 * t, 0.28 + 0.17 * t, 1.0]
    } else {
        // Snow peaks
        let t = ((y - 4.5) / 2.0).clamp(0.0, 1.0);
        [0.45 + 0.5 * t, 0.45 + 0.5 * t, 0.45 + 0.53 * t, 1.0]
    }
}

// Generate the 64x64 grid mesh. 
// Uses unshared vertices to calculate flat face normals for a crispy Low-Poly render.
pub fn generate_terrain_mesh(seed: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for x in 0..64 {
        for z in 0..64 {
            let px0 = x as f32 - 32.0;
            let pz0 = z as f32 - 32.0;
            let px1 = (x + 1) as f32 - 32.0;
            let pz1 = (z + 1) as f32 - 32.0;

            let h0 = get_height(px0, pz0, seed);
            let h1 = get_height(px1, pz0, seed);
            let h2 = get_height(px0, pz1, seed);
            let h3 = get_height(px1, pz1, seed);

            let p0 = Vec3::new(px0, h0, pz0);
            let p1 = Vec3::new(px1, h1, pz0);
            let p2 = Vec3::new(px0, h2, pz1);
            let p3 = Vec3::new(px1, h3, pz1);

            // Triangle 1 (p0 -> p2 -> p1) - Face Winding CCW from top
            let n1 = (p2 - p0).cross(p1 - p0).normalize();
            let avg_h1 = (h0 + h2 + h1) / 3.0;
            let color1 = get_color(avg_h1);

            let start_idx1 = positions.len() as u32;
            positions.push(p0.to_array());
            positions.push(p2.to_array());
            positions.push(p1.to_array());

            for _ in 0..3 {
                normals.push(n1.to_array());
                colors.push(color1);
            }
            indices.push(start_idx1);
            indices.push(start_idx1 + 1);
            indices.push(start_idx1 + 2);

            // Triangle 2 (p1 -> p3 -> p2) - Face Winding CCW from top
            let n2 = (p3 - p1).cross(p2 - p1).normalize();
            let avg_h2 = (h1 + h3 + h2) / 3.0;
            let color2 = get_color(avg_h2);

            let start_idx2 = positions.len() as u32;
            positions.push(p1.to_array());
            positions.push(p3.to_array());
            positions.push(p2.to_array());

            for _ in 0..3 {
                normals.push(n2.to_array());
                colors.push(color2);
            }
            indices.push(start_idx2);
            indices.push(start_idx2 + 1);
            indices.push(start_idx2 + 2);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Configure Global Ambient Light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
        ..default()
    });

    // 2. Spawn Directional Light for Crisp Shadowing
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_xyz(30.0, 50.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        TerrainCleanup,
    ));

    // 3. Spawn Orbital View Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(30.0, 20.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        TerrainCamera,
        TerrainCleanup,
    ));

    // 4. Generate & Spawn Procedural Terrain Mesh Entity
    let seed_val = get_random_value() * 5000.0;
    let mesh = generate_terrain_mesh(seed_val);
    let mesh_handle = meshes.add(mesh);
    let material_handle = materials.add(StandardMaterial {
        perceptual_roughness: 0.85,
        metallic: 0.1,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, -1.0, 0.0), // Center at zero
        TerrainMeshMarker,
        TerrainCleanup,
    ));

    // 5. Save the seed resource
    commands.insert_resource(TerrainSeed(seed_val));

    // 6. Spawn Bevy UI HUD controls & Refresh button
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        TerrainCleanup,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Low-Poly Landscape"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        parent.spawn((
            Text::new("Procedural 3D Terrain generated dynamically inside browser"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
        ));
        parent.spawn((
            Button,
            Node {
                width: Val::Px(120.0),
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
                Text::new("Refresh (R)"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

pub fn cleanup_terrain(
    mut commands: Commands,
    query: Query<Entity, With<TerrainCleanup>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<TerrainSeed>();
}

pub fn terrain_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut seed: ResMut<TerrainSeed>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain_query: Query<Entity, With<TerrainMeshMarker>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    let mut should_refresh = keys.just_pressed(KeyCode::KeyR);

    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));
                should_refresh = true;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
        }
    }

    if should_refresh {
        // Compute new random seed and rebuild procedural mesh
        seed.0 = get_random_value() * 10000.0;
        let new_mesh = generate_terrain_mesh(seed.0);
        let new_mesh_handle = meshes.add(new_mesh);

        // Update the terrain entity standard mesh component
        for entity in &terrain_query {
            commands.entity(entity).insert(Mesh3d(new_mesh_handle.clone()));
        }
    }
}

pub fn camera_spin_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<TerrainCamera>>,
) {
    let speed = 0.08; // Slow elegant rotating orbit
    let elapsed = time.elapsed_secs() * speed;
    let radius = 38.0;
    
    let x = radius * elapsed.cos();
    let z = radius * elapsed.sin();
    let y = 18.0 + (elapsed * 0.4).sin() * 4.0; // Dynamic height wave
    
    for mut transform in &mut query {
        *transform = Transform::from_xyz(x, y, z).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::GameMode::Terrain),
            setup_terrain,
        )
        .add_systems(
            OnExit(crate::GameMode::Terrain),
            cleanup_terrain,
        )
        .add_systems(
            Update,
            (
                terrain_input_system,
                camera_spin_system,
            )
                .run_if(in_state(crate::GameMode::Terrain)),
        );
    }
}
