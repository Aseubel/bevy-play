use crate::VoxelEntity;
use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::window::{CursorGrabMode, CursorOptions};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BlockType {
    Air,
    Grass,
    Dirt,
    Stone,
    Wood,
    Leaves,
}

impl BlockType {
    fn color(&self, direction: IVec3) -> [f32; 4] {
        let base = match self {
            BlockType::Air => [0.0, 0.0, 0.0, 0.0],
            BlockType::Grass => {
                if direction.y > 0 {
                    [0.34, 0.65, 0.28, 1.0] // Green top
                } else {
                    [0.45, 0.34, 0.22, 1.0] // Brown side/bottom
                }
            }
            BlockType::Dirt => [0.45, 0.34, 0.22, 1.0],
            BlockType::Stone => [0.55, 0.55, 0.55, 1.0],
            BlockType::Wood => [0.38, 0.26, 0.15, 1.0],
            BlockType::Leaves => [0.15, 0.45, 0.15, 1.0],
        };

        let factor = if direction.y > 0 {
            1.0
        } else if direction.y < 0 {
            0.4
        } else if direction.x != 0 {
            0.75
        } else {
            0.85
        };

        [
            base[0] * factor,
            base[1] * factor,
            base[2] * factor,
            base[3],
        ]
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, BlockType>,
    pub changed: bool,
}

#[derive(Resource)]
pub struct CurrentBlock {
    pub block_type: BlockType,
}

impl Default for CurrentBlock {
    fn default() -> Self {
        Self {
            block_type: BlockType::Grass,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Component)]
pub struct WorldMesh;

#[derive(Component)]
pub struct SelectedBlockText;

fn spawn_tree(world: &mut HashMap<IVec3, BlockType>, base: IVec3) {
    for h in 0..5 {
        world.insert(base + IVec3::new(0, h, 0), BlockType::Wood);
    }
    let leaves_center = base + IVec3::new(0, 5, 0);
    for dx in -2..=2 {
        for dy in 0..=2 {
            for dz in -2..=2 {
                let dist = dx * dx + dy * dy + dz * dz;
                if dist <= 5 {
                    let leaf_pos = leaves_center + IVec3::new(dx, dy, dz);
                    world.entry(leaf_pos).or_insert(BlockType::Leaves);
                }
            }
        }
    }
}

fn generate_mesh(world: &HashMap<IVec3, BlockType>) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let directions = [
        (
            IVec3::new(0, 1, 0),
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, -0.5],
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
            ],
        ),
        (
            IVec3::new(0, -1, 0),
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, 0.5],
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
            ],
        ),
        (
            IVec3::new(-1, 0, 0),
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
        ),
        (
            IVec3::new(1, 0, 0),
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
        ),
        (
            IVec3::new(0, 0, -1),
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
        ),
        (
            IVec3::new(0, 0, 1),
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
        ),
    ];

    for (&pos, &block_type) in world.iter() {
        if block_type == BlockType::Air {
            continue;
        }
        let pos_f = pos.as_vec3();

        for &(dir, normal, face_verts) in &directions {
            let neighbor_pos = pos + dir;
            let is_neighbor_solid = world
                .get(&neighbor_pos)
                .map_or(false, |&b| b != BlockType::Air);

            if !is_neighbor_solid {
                let start_idx = positions.len() as u32;
                for &vert in &face_verts {
                    positions.push([vert[0] + pos_f.x, vert[1] + pos_f.y, vert[2] + pos_f.z]);
                    normals.push(normal);
                    colors.push(block_type.color(dir));
                }
                indices.extend_from_slice(&[
                    start_idx,
                    start_idx + 1,
                    start_idx + 2,
                    start_idx,
                    start_idx + 2,
                    start_idx + 3,
                ]);
            }
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

pub fn setup_voxel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut world: ResMut<VoxelWorld>,
) {
    world.blocks.clear();
    world.changed = false;

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 600.0,
        ..default()
    });

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_xyz(40.0, 80.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        VoxelEntity,
    ));

    let size = 64;
    for x in -size / 2..size / 2 {
        for z in -size / 2..size / 2 {
            let x_f = x as f32;
            let z_f = z as f32;
            let height = (((x_f * 0.08).sin() + (z_f * 0.08).cos()) * 4.0
                + ((x_f * 0.02).cos() * (z_f * 0.02).sin()) * 8.0
                + 12.0) as i32;

            for y in 0..=height {
                let pos = IVec3::new(x, y, z);
                let block = if y == height {
                    BlockType::Grass
                } else if y >= height - 3 {
                    BlockType::Dirt
                } else {
                    BlockType::Stone
                };
                world.blocks.insert(pos, block);
            }
        }
    }

    let mut rng_seed = 42;
    for _ in 0..35 {
        rng_seed = (rng_seed * 1103515245 + 12345) & 0x7fffffff;
        let x = (rng_seed % size) - size / 2;
        rng_seed = (rng_seed * 1103515245 + 12345) & 0x7fffffff;
        let z = (rng_seed % size) - size / 2;

        let mut ground_y = -1;
        for y in (0..35).rev() {
            if let Some(&block) = world.blocks.get(&IVec3::new(x, y, z)) {
                if block == BlockType::Grass {
                    ground_y = y;
                    break;
                }
            }
        }

        if ground_y != -1 {
            spawn_tree(&mut world.blocks, IVec3::new(x, ground_y + 1, z));
        }
    }

    world.changed = true;

    let mesh = generate_mesh(&world.blocks);
    let mesh_handle = meshes.add(mesh);
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        WorldMesh,
        VoxelEntity,
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 22.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        Player {
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        },
        VoxelEntity,
    ));
}

pub fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(8.0),
            height: Val::Px(8.0),
            margin: UiRect {
                left: Val::Px(-4.0),
                right: Val::Px(0.0),
                top: Val::Px(-4.0),
                bottom: Val::Px(0.0),
            },
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
        BackgroundColor(Color::WHITE),
        VoxelEntity,
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        VoxelEntity,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Selected Block: Grass"),
            TextFont { font_size: FontSize::Px(22.0), ..default() },
            TextColor(Color::WHITE),
            SelectedBlockText,
        ));
        parent.spawn((
            Text::new("Controls:\n- WASD to Move\n- Space to Jump\n- Mouse to Look\n- Left Click to Break block\n- Right Click to Place block\n- 1-5 keys to change block type\n- Esc to release mouse pointer"),
            TextFont { font_size: FontSize::Px(15.0), ..default() },
            TextColor(Color::srgb(0.85, 0.85, 0.85)),
        ));
    });
}

pub fn grab_mouse(
    mut query: Query<&mut CursorOptions, With<Window>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for mut options in &mut query {
        if mouse.just_pressed(MouseButton::Left) {
            options.visible = false;
            options.grab_mode = CursorGrabMode::Locked;
        }
        if keys.just_pressed(KeyCode::Escape) {
            options.visible = true;
            options.grab_mode = CursorGrabMode::None;
        }
    }
}

pub fn player_look_system(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut query: Query<(&mut Player, &mut Transform)>,
    window: Single<&CursorOptions, With<Window>>,
) {
    if window.grab_mode == CursorGrabMode::None {
        return;
    }

    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    for (mut player, mut transform) in &mut query {
        let sensitivity = 0.0015;
        player.yaw -= delta.x * sensitivity;
        player.pitch -= delta.y * sensitivity;
        player.pitch = player.pitch.clamp(-1.54, 1.54);

        transform.rotation = Quat::from_axis_angle(Vec3::Y, player.yaw)
            * Quat::from_axis_angle(Vec3::X, player.pitch);
    }
}

fn check_collision(pos: Vec3, radius: f32, height: f32, world: &VoxelWorld) -> bool {
    let min_x = (pos.x - radius).floor() as i32;
    let max_x = (pos.x + radius).floor() as i32;
    let min_y = pos.y.floor() as i32;
    let max_y = (pos.y + height).floor() as i32;
    let min_z = (pos.z - radius).floor() as i32;
    let max_z = (pos.z + radius).floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                if let Some(&block) = world.blocks.get(&IVec3::new(x, y, z)) {
                    if block != BlockType::Air {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_on_ground(pos: &Vec3, world: &VoxelWorld) -> bool {
    check_collision(*pos + Vec3::new(0.0, -0.05, 0.0), 0.35, 0.1, world)
}

pub fn player_move_system(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<VoxelWorld>,
    mut query: Query<(&mut Player, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut player, mut transform) in &mut query {
        let forward = transform.forward();
        let mut forward_xz = Vec3::new(forward.x, 0.0, forward.z);
        forward_xz = forward_xz.normalize_or_zero();

        let right = transform.right();
        let mut right_xz = Vec3::new(right.x, 0.0, right.z);
        right_xz = right_xz.normalize_or_zero();

        let mut direction = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            direction += forward_xz;
        }
        if keys.pressed(KeyCode::KeyS) {
            direction -= forward_xz;
        }
        if keys.pressed(KeyCode::KeyA) {
            direction -= right_xz;
        }
        if keys.pressed(KeyCode::KeyD) {
            direction += right_xz;
        }

        let speed = 9.0;
        let move_vel = direction.normalize_or_zero() * speed;

        player.velocity.x = move_vel.x;
        player.velocity.z = move_vel.z;

        let gravity = -22.0;
        player.velocity.y += gravity * dt;

        if keys.just_pressed(KeyCode::Space) && is_on_ground(&transform.translation, &world) {
            player.velocity.y = 8.5;
        }

        let new_pos = transform.translation + player.velocity * dt;
        let player_radius = 0.38;
        let player_height = 1.8;

        if check_collision(new_pos, player_radius, player_height, &world) {
            let mut test_pos = transform.translation;
            test_pos.x = new_pos.x;
            if check_collision(test_pos, player_radius, player_height, &world) {
                player.velocity.x = 0.0;
            } else {
                transform.translation.x = new_pos.x;
            }

            test_pos = transform.translation;
            test_pos.z = new_pos.z;
            if check_collision(test_pos, player_radius, player_height, &world) {
                player.velocity.z = 0.0;
            } else {
                transform.translation.z = new_pos.z;
            }

            test_pos = transform.translation;
            test_pos.y = new_pos.y;
            if check_collision(test_pos, player_radius, player_height, &world) {
                player.velocity.y = 0.0;
            } else {
                transform.translation.y = new_pos.y;
            }
        } else {
            transform.translation = new_pos;
        }
    }
}

pub fn handle_block_selection(
    keys: Res<ButtonInput<KeyCode>>,
    mut current: ResMut<CurrentBlock>,
    mut query: Query<&mut Text, With<SelectedBlockText>>,
) {
    let mut changed = false;
    if keys.just_pressed(KeyCode::Digit1) {
        current.block_type = BlockType::Grass;
        changed = true;
    } else if keys.just_pressed(KeyCode::Digit2) {
        current.block_type = BlockType::Dirt;
        changed = true;
    } else if keys.just_pressed(KeyCode::Digit3) {
        current.block_type = BlockType::Stone;
        changed = true;
    } else if keys.just_pressed(KeyCode::Digit4) {
        current.block_type = BlockType::Wood;
        changed = true;
    } else if keys.just_pressed(KeyCode::Digit5) {
        current.block_type = BlockType::Leaves;
        changed = true;
    }

    if changed {
        for mut text in &mut query {
            *text = Text::new(format!("Selected Block: {:?}", current.block_type));
        }
    }
}

struct RaycastResult {
    hit_block: IVec3,
    place_block: IVec3,
}

fn raycast(
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
    world: &VoxelWorld,
) -> Option<RaycastResult> {
    let step = 0.04;
    let mut current = origin;
    let mut last_voxel = origin.floor().as_ivec3();

    let mut dist = 0.0;
    while dist < max_dist {
        current += direction * step;
        dist += step;

        let current_voxel = current.floor().as_ivec3();
        if current_voxel != last_voxel {
            if let Some(&block) = world.blocks.get(&current_voxel) {
                if block != BlockType::Air {
                    return Some(RaycastResult {
                        hit_block: current_voxel,
                        place_block: last_voxel,
                    });
                }
            }
            last_voxel = current_voxel;
        }
    }
    None
}

pub fn handle_block_interaction(
    mouse: Res<ButtonInput<MouseButton>>,
    camera_transform: Single<&Transform, With<Player>>,
    mut world: ResMut<VoxelWorld>,
    current_block: Res<CurrentBlock>,
) {
    let mut clicked = false;
    let mut is_break = false;
    if mouse.just_pressed(MouseButton::Left) {
        clicked = true;
        is_break = true;
    } else if mouse.just_pressed(MouseButton::Right) {
        clicked = true;
        is_break = false;
    }

    if clicked {
        let origin = camera_transform.translation;
        let direction = camera_transform.forward();
        if let Some(hit) = raycast(origin, *direction, 6.0, &world) {
            if is_break {
                world.blocks.remove(&hit.hit_block);
                world.changed = true;
            } else {
                let player_pos = origin;
                let player_radius = 0.38;
                let player_height = 1.8;

                let min_p = player_pos - Vec3::new(player_radius, 0.0, player_radius);
                let max_p = player_pos + Vec3::new(player_radius, player_height, player_radius);

                let min_b = hit.place_block.as_vec3();
                let max_b = min_b + Vec3::ONE;

                let overlap = min_p.x < max_b.x
                    && max_p.x > min_b.x
                    && min_p.y < max_b.y
                    && max_p.y > min_b.y
                    && min_p.z < max_b.z
                    && max_p.z > min_b.z;

                if !overlap {
                    world
                        .blocks
                        .insert(hit.place_block, current_block.block_type);
                    world.changed = true;
                }
            }
        }
    }
}

pub fn update_world_mesh_system(
    mut world: ResMut<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_3d: Single<&Mesh3d, With<WorldMesh>>,
) {
    if !world.changed {
        return;
    }
    world.changed = false;

    if let Some(mut mesh) = meshes.get_mut(&mesh_3d.0) {
        let new_mesh = generate_mesh(&world.blocks);
        *mesh = new_mesh;
    }
}
