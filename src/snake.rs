use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use crate::SnakeEntity;
use js_sys::wasm_bindgen::JsCast;

// ==========================================
// 1. States & Resources
// ==========================================

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SnakeState {
    #[default]
    WaitingToStart,
    Playing,
    Paused,
    GameOver,
}

#[derive(Resource)]
pub struct SnakeTickTimer(pub Timer);

impl Default for SnakeTickTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.14, TimerMode::Repeating))
    }
}

#[derive(Resource, Default)]
pub struct SnakeScore {
    pub score: u32,
    pub high_score: u32,
}

// ==========================================
// 2. Components
// ==========================================

#[derive(Component)]
pub struct SnakeHead;

#[derive(Component)]
pub struct SnakeSegment {
    pub index: usize,
}

#[derive(Component)]
pub struct GridPosition(pub IVec3);

#[derive(Component)]
pub struct PreviousGridPosition(pub IVec3);

#[derive(Component)]
pub struct MovementDirection {
    pub dir: IVec2,
    pub next_dir: IVec2,
}

#[derive(Component)]
pub struct SnakeFood;

#[derive(Component)]
pub struct SnakeHudText;

#[derive(Component)]
pub struct SnakeCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub auto_reset: bool,
}

#[derive(Component)]
pub struct UpdraftVent;

// ==========================================
// 3. Constants & Helpers
// ==========================================

const VENT_1: IVec2 = IVec2::new(7, 7);
const VENT_2: IVec2 = IVec2::new(-7, -7);
const VENT_3: IVec2 = IVec2::new(7, -7);
const VENT_4: IVec2 = IVec2::new(-7, 7);

fn get_random_value() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Math::random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed as f64) / (u64::MAX as f64)
    }
}

fn get_random_food_pos(segments: &[IVec3]) -> IVec3 {
    loop {
        let floor = if get_random_value() < 0.5 { 1 } else { 0 };

        let (x, z) = if floor == 1 {
            let rx = (get_random_value() * 19.0) as i32 - 9;
            let rz = (get_random_value() * 19.0) as i32 - 9;
            (rx, rz)
        } else {
            let rx = (get_random_value() * 31.0) as i32 - 15;
            let rz = (get_random_value() * 31.0) as i32 - 15;
            (rx, rz)
        };

        let pos = IVec3::new(x, floor, z);

        let on_vent = floor == 0
            && (IVec2::new(x, z) == VENT_1
                || IVec2::new(x, z) == VENT_2
                || IVec2::new(x, z) == VENT_3
                || IVec2::new(x, z) == VENT_4);

        let dist_to_head = if !segments.is_empty() {
            (pos - segments[0]).as_vec3().length()
        } else {
            999.0
        };

        if !segments.contains(&pos) && !on_vent && dist_to_head >= 5.0 {
            return pos;
        }
    }
}

fn update_hud_text(score: &SnakeScore, state: SnakeState, text: &mut Text) {
    let status_str = match state {
        SnakeState::WaitingToStart => "Press direction key to start",
        SnakeState::Playing => "PLAYING",
        SnakeState::Paused => "PAUSED (Right-drag mouse to orbit view)",
        SnakeState::GameOver => "GAME OVER! Press R to restart",
    };
    text.0 = format!(
        "Score: {}\nHigh Score: {}\nStatus: {}",
        score.score, score.high_score, status_str
    );
}

// ==========================================
// 4. Systems
// ==========================================

pub fn setup_snake(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut score: ResMut<SnakeScore>,
    mut next_state: ResMut<NextState<SnakeState>>,
) {
    // Reset state & score
    score.score = 0;
    next_state.set(SnakeState::WaitingToStart);
    commands.insert_resource(SnakeTickTimer(Timer::from_seconds(0.14, TimerMode::Repeating)));

    // 1. Lighting Setup
    commands.spawn((
        DirectionalLight {
            illuminance: 3200.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        SnakeEntity,
    ));

    // 2. Playfields (Lower & Upper Boards)
    let lower_board_mesh = meshes.add(Plane3d::default().mesh().size(31.0, 31.0));
    let grid_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.04, 0.08),
        perceptual_roughness: 0.7,
        metallic: 0.2,
        ..default()
    });
    commands.spawn((
        Mesh3d(lower_board_mesh),
        MeshMaterial3d(grid_material.clone()),
        Transform::from_xyz(0.0, -0.5, 0.0),
        SnakeEntity,
    ));

    let upper_board_mesh = meshes.add(Plane3d::default().mesh().size(19.0, 19.0));
    let upper_grid_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.08, 0.16, 0.35),
        perceptual_roughness: 0.1,
        metallic: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(upper_board_mesh),
        MeshMaterial3d(upper_grid_material),
        Transform::from_xyz(0.0, 2.5, 0.0),
        SnakeEntity,
    ));

    // 3. Neon Playfield Borders (Cylinders)
    let border_tube = meshes.add(Cylinder::new(0.08, 31.0));
    let neon_blue = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 1.0),
        emissive: LinearRgba::rgb(0.0, 2.5, 8.0),
        ..default()
    });

    let borders = [
        (Vec3::new(0.0, -0.5, 15.5), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(0.0, -0.5, -15.5), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(15.5, -0.5, 0.0), Quat::IDENTITY),
        (Vec3::new(-15.5, -0.5, 0.0), Quat::IDENTITY),
    ];
    for (trans, rot) in borders {
        commands.spawn((
            Mesh3d(border_tube.clone()),
            MeshMaterial3d(neon_blue.clone()),
            Transform::from_translation(trans).with_rotation(rot),
            SnakeEntity,
        ));
    }

    let upper_border_tube = meshes.add(Cylinder::new(0.08, 19.0));
    let neon_purple = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.0, 1.0),
        emissive: LinearRgba::rgb(6.0, 0.0, 8.0),
        ..default()
    });

    let upper_borders = [
        (Vec3::new(0.0, 2.5, 9.5), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(0.0, 2.5, -9.5), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(9.5, 2.5, 0.0), Quat::IDENTITY),
        (Vec3::new(-9.5, 2.5, 0.0), Quat::IDENTITY),
    ];
    for (trans, rot) in upper_borders {
        commands.spawn((
            Mesh3d(upper_border_tube.clone()),
            MeshMaterial3d(neon_purple.clone()),
            Transform::from_translation(trans).with_rotation(rot),
            SnakeEntity,
        ));
    }

    // 4. Updraft Vents Setup
    let vent_base_mesh = meshes.add(Cylinder::new(0.7, 0.1));
    let vent_base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        metallic: 0.8,
        perceptual_roughness: 0.2,
        ..default()
    });
    let wind_mesh = meshes.add(Cylinder::new(0.65, 3.0));
    let wind_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 1.0, 0.3, 0.08),
        emissive: LinearRgba::rgb(0.0, 0.4, 0.1),
        ..default()
    });

    for &vent_pos in &[VENT_1, VENT_2, VENT_3, VENT_4] {
        let world_x = vent_pos.x as f32;
        let world_z = vent_pos.y as f32;

        commands.spawn((
            Mesh3d(vent_base_mesh.clone()),
            MeshMaterial3d(vent_base_material.clone()),
            Transform::from_xyz(world_x, -0.5, world_z),
            SnakeEntity,
        ));

        commands.spawn((
            Mesh3d(wind_mesh.clone()),
            MeshMaterial3d(wind_material.clone()),
            Transform::from_xyz(world_x, 1.0, world_z),
            UpdraftVent,
            SnakeEntity,
        ));

        commands.spawn((
            PointLight {
                color: Color::srgb(0.0, 1.0, 0.3),
                intensity: 1500.0,
                range: 6.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(world_x, -0.3, world_z),
            SnakeEntity,
        ));
    }

    // 5. Spawn Snake Entities (Head + 2 Body Segments)
    let head_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.25, 0.8),
        emissive: LinearRgba::rgb(4.0, 0.3, 2.0),
        metallic: 1.0,
        perceptual_roughness: 0.05,
        ..default()
    });
    let head_mesh = meshes.add(Sphere::new(0.55));
    
    let head_entity = commands.spawn((
        Mesh3d(head_mesh),
        MeshMaterial3d(head_material),
        Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2), // Facing +X
            scale: Vec3::new(1.15, 0.9, 1.35),
        },
        SnakeHead,
        SnakeSegment { index: 0 },
        GridPosition(IVec3::new(0, 0, 0)),
        PreviousGridPosition(IVec3::new(0, 0, 0)),
        MovementDirection {
            dir: IVec2::new(1, 0),
            next_dir: IVec2::new(1, 0),
        },
        SnakeEntity,
    )).id();

    // Attach glowing cyber-eyes to the head
    let eye_mesh = meshes.add(Sphere::new(0.12));
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 1.0),
        emissive: LinearRgba::rgb(0.0, 8.0, 8.0),
        perceptual_roughness: 0.0,
        ..default()
    });
    commands.entity(head_entity).with_children(|parent| {
        parent.spawn((
            Mesh3d(eye_mesh.clone()),
            MeshMaterial3d(eye_material.clone()),
            Transform::from_xyz(-0.24, 0.22, 0.38),
        ));
        parent.spawn((
            Mesh3d(eye_mesh),
            MeshMaterial3d(eye_material),
            Transform::from_xyz(0.24, 0.22, 0.38),
        ));
    });

    // Spawn Segment 1
    let radius1 = (0.48f32 * (1.0f32 - 0.5f32 * 0.25f32)).max(0.32f32);
    let segment_mesh1 = meshes.add(Capsule3d::new(radius1, 1.0));
    let mat1 = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75 * 0.5, 0.15 * 0.5, 1.0 - 0.4 * 0.5),
        emissive: LinearRgba::rgb(0.75 * 0.5 * 2.5, 0.0, (1.0 - 0.4 * 0.5) * 3.0),
        metallic: 0.95,
        perceptual_roughness: 0.05,
        ..default()
    });
    commands.spawn((
        Mesh3d(segment_mesh1),
        MeshMaterial3d(mat1),
        Transform {
            translation: Vec3::new(-0.5, 0.0, 0.0),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::X),
            scale: Vec3::new(1.0, 1.0, 1.0),
        },
        SnakeSegment { index: 1 },
        GridPosition(IVec3::new(-1, 0, 0)),
        PreviousGridPosition(IVec3::new(-1, 0, 0)),
        SnakeEntity,
    ));

    // Spawn Segment 2
    let radius2 = (0.48f32 * (1.0f32 - 1.0f32 * 0.25f32)).max(0.32f32);
    let segment_mesh2 = meshes.add(Capsule3d::new(radius2, 1.0));
    let mat2 = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.0, 1.0 - 0.4),
        emissive: LinearRgba::rgb(0.0, 0.0, (1.0 - 0.4) * 3.0),
        metallic: 0.95,
        perceptual_roughness: 0.05,
        ..default()
    });
    commands.spawn((
        Mesh3d(segment_mesh2),
        MeshMaterial3d(mat2),
        Transform {
            translation: Vec3::new(-1.5, 0.0, 0.0),
            rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::X),
            scale: Vec3::new(1.0, 1.0, 1.0),
        },
        SnakeSegment { index: 2 },
        GridPosition(IVec3::new(-2, 0, 0)),
        PreviousGridPosition(IVec3::new(-2, 0, 0)),
        SnakeEntity,
    ));

    // 6. Spawn Food Entity (Gemstone)
    let initial_segments = [
        IVec3::new(0, 0, 0),
        IVec3::new(-1, 0, 0),
        IVec3::new(-2, 0, 0),
    ];
    let food_pos = get_random_food_pos(&initial_segments);
    let food_mesh = meshes.add(Cuboid::new(0.55, 0.55, 0.55));
    let food_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.15, 0.15),
        emissive: LinearRgba::rgb(8.0, 0.5, 0.5),
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    let food_world_y = food_pos.y as f32 * 3.0;
    let food_entity = commands.spawn((
        Mesh3d(food_mesh),
        MeshMaterial3d(food_material),
        Transform::from_xyz(food_pos.x as f32, food_world_y, food_pos.z as f32)
            .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.78, 0.78, 0.0)),
        SnakeFood,
        GridPosition(food_pos),
        SnakeEntity,
    )).id();

    // Attach glowing point light to the food entity as a child
    commands.entity(food_entity).with_children(|parent| {
        parent.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.15, 0.15),
                intensity: 1500.0,
                range: 7.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.5, 0.0),
        ));
    });

    // 7. Spawn Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 26.0, 24.0).looking_at(Vec3::ZERO, Vec3::Y),
        SnakeCamera {
            yaw: 0.0,
            pitch: 0.8,
            auto_reset: true,
        },
        SnakeEntity,
    ));
}

pub fn spawn_snake_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(15.0),
                left: Val::Px(15.0),
                ..default()
            },
            SnakeEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Score: 0\nHigh Score: 0\nPress direction key to start"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                SnakeHudText,
            ));
        });
}

pub fn snake_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<SnakeState>>,
    mut next_state: ResMut<NextState<SnakeState>>,
    mut head_query: Query<&mut MovementDirection, With<SnakeHead>>,
    mut hud_query: Query<&mut Text, With<SnakeHudText>>,
    score: Res<SnakeScore>,
    mut commands: Commands,
    snake_entities: Query<Entity, With<SnakeEntity>>,
) {
    let current_state = *state.get();

    // Spacebar Pause Toggle
    if keys.just_pressed(KeyCode::Space) {
        match current_state {
            SnakeState::Playing => {
                next_state.set(SnakeState::Paused);
                if let Ok(mut text) = hud_query.single_mut() {
                    update_hud_text(&score, SnakeState::Paused, &mut text);
                }
            }
            SnakeState::Paused => {
                next_state.set(SnakeState::Playing);
                if let Ok(mut text) = hud_query.single_mut() {
                    update_hud_text(&score, SnakeState::Playing, &mut text);
                }
            }
            _ => {}
        }
    }

    // Restart triggered on GameOver
    if current_state == SnakeState::GameOver && keys.just_pressed(KeyCode::KeyR) {
        for entity in &snake_entities {
            commands.entity(entity).despawn();
        }
        next_state.set(SnakeState::WaitingToStart);
    }

    if current_state != SnakeState::Playing && current_state != SnakeState::WaitingToStart {
        return;
    }

    // Direction Key Detections
    let mut dir = None;
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) {
        dir = Some(IVec2::new(-1, 0));
    } else if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) {
        dir = Some(IVec2::new(1, 0));
    } else if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) {
        dir = Some(IVec2::new(0, -1));
    } else if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) {
        dir = Some(IVec2::new(0, 1));
    }

    if let Some(new_dir) = dir {
        if current_state == SnakeState::WaitingToStart {
            next_state.set(SnakeState::Playing);
            if let Ok(mut text) = hud_query.single_mut() {
                update_hud_text(&score, SnakeState::Playing, &mut text);
            }
        }

        if let Ok(mut head_dir) = head_query.single_mut() {
            // Prevent 180-degree self-collision turns
            if new_dir.x != -head_dir.dir.x || new_dir.y != -head_dir.dir.y {
                head_dir.next_dir = new_dir;
            }
        }
    }
}

pub fn snake_tick_system(
    time: Res<Time>,
    mut timer: ResMut<SnakeTickTimer>,
    state: Res<State<SnakeState>>,
    mut next_state: ResMut<NextState<SnakeState>>,
    mut head_query: Query<(&mut GridPosition, &mut MovementDirection), With<SnakeHead>>,
    mut body_query: Query<(&mut GridPosition, &mut PreviousGridPosition, &SnakeSegment), Without<SnakeHead>>,
    mut all_segments_pos_query: Query<(&GridPosition, &mut PreviousGridPosition), With<SnakeSegment>>,
    mut food_query: Query<(Entity, &mut GridPosition, &mut Transform), With<SnakeFood>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut score: ResMut<SnakeScore>,
    mut hud_query: Query<&mut Text, With<SnakeHudText>>,
) {
    if *state.get() != SnakeState::Playing {
        return;
    }

    timer.0.tick(time.delta());
    if !timer.0.is_finished() {
        return;
    }

    // 1. Fetch Head & Food details
    let Ok((mut head_pos, mut head_dir)) = head_query.single_mut() else { return; };
    let Ok((_food_entity, mut food_pos, mut food_transform)) = food_query.single_mut() else { return; };
    let old_head_pos = head_pos.0;

    // 2. Set PreviousGridPosition = GridPosition for all segments
    for (grid_pos, mut prev_pos) in &mut all_segments_pos_query {
        prev_pos.0 = grid_pos.0;
    }

    // 3. Update Heading Direction
    head_dir.dir = head_dir.next_dir;

    // 4. Calculate proposed Head Position
    let mut new_head = old_head_pos + IVec3::new(head_dir.dir.x, 0, head_dir.dir.y);

    // 5. Updraft & Floor Height Mechanics
    if old_head_pos.y == 1 {
        // Upper boundaries
        if new_head.x < -9 || new_head.x > 9 || new_head.z < -9 || new_head.z > 9 {
            new_head.y = 0;
        }
    }

    if new_head.y == 0 {
        // Lower boundaries
        if new_head.x < -15 || new_head.x > 15 || new_head.z < -15 || new_head.z > 15 {
            next_state.set(SnakeState::GameOver);
            if let Ok(mut text) = hud_query.single_mut() {
                update_hud_text(&score, SnakeState::GameOver, &mut text);
            }
            return;
        }

        // Updraft vents launch check
        let xz = IVec2::new(new_head.x, new_head.z);
        if xz == VENT_1 || xz == VENT_2 || xz == VENT_3 || xz == VENT_4 {
            new_head.y = 1;
        }
    }

    // 6. Eat Food & Self-Collision check
    let is_eating = new_head == food_pos.0;

    let body_positions: Vec<IVec3> = body_query.iter().map(|(g, _, _)| g.0).collect();
    let collision_list = if is_eating {
        &body_positions[..]
    } else if body_positions.is_empty() {
        &[]
    } else {
        &body_positions[..body_positions.len() - 1]
    };

    if collision_list.contains(&new_head) {
        next_state.set(SnakeState::GameOver);
        if let Ok(mut text) = hud_query.single_mut() {
            update_hud_text(&score, SnakeState::GameOver, &mut text);
        }
        return;
    }

    // 7. Shift body positions
    let mut sorted_body: Vec<_> = body_query.iter_mut().collect();
    sorted_body.sort_by_key(|(_, _, seg)| seg.index);

    let old_tail_grid = if let Some(last_seg) = sorted_body.last() {
        last_seg.0.0
    } else {
        old_head_pos
    };

    for i in (1..sorted_body.len()).rev() {
        let ahead_pos = sorted_body[i - 1].0.0;
        sorted_body[i].0.0 = ahead_pos;
    }
    if let Some(first_seg) = sorted_body.first_mut() {
        first_seg.0.0 = old_head_pos;
    }

    // 8. Move Head
    head_pos.0 = new_head;

    // 9. Handle Growth
    if is_eating {
        score.score += 10;
        if score.score > score.high_score {
            score.high_score = score.score;
        }
        if let Ok(mut text) = hud_query.single_mut() {
            update_hud_text(&score, SnakeState::Playing, &mut text);
        }

        // Spawn new Capsule body segment at the old tail position
        let current_length = sorted_body.len() + 1; // index count (including head)
        let t = (current_length as f32) / (current_length as f32 + 1.0);
        let radius = (0.48 * (1.0 - t * 0.25)).max(0.32);
        let segment_mesh = meshes.add(Capsule3d::new(radius, 1.0));

        let r = 0.75 * (1.0 - t);
        let g = 0.15 * (1.0 - t);
        let b = 1.0 - 0.4 * t;
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::rgb(r * 2.5, 0.0, b * 3.0),
            metallic: 0.95,
            perceptual_roughness: 0.05,
            ..default()
        });

        let world_tail = Vec3::new(old_tail_grid.x as f32, old_tail_grid.y as f32 * 3.0, old_tail_grid.z as f32);

        commands.spawn((
            Mesh3d(segment_mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(world_tail),
            SnakeSegment { index: current_length },
            GridPosition(old_tail_grid),
            PreviousGridPosition(old_tail_grid),
            SnakeEntity,
        ));

        // Relocate Food
        let mut all_segs = vec![new_head];
        all_segs.extend(body_positions.iter());
        let new_food_grid = get_random_food_pos(&all_segs);

        food_pos.0 = new_food_grid;
        let food_world_y = new_food_grid.y as f32 * 3.0;
        food_transform.translation = Vec3::new(new_food_grid.x as f32, food_world_y, new_food_grid.z as f32);
    }
}

pub fn snake_movement_interpolation_system(
    timer: Res<SnakeTickTimer>,
    state: Res<State<SnakeState>>,
    mut query: Query<(&mut Transform, &SnakeSegment, &GridPosition, &PreviousGridPosition)>,
    head_query: Query<&MovementDirection, With<SnakeHead>>,
) {
    let current_state = *state.get();
    let t = if current_state == SnakeState::WaitingToStart {
        0.0
    } else {
        timer.0.fraction()
    };

    let Ok(head_dir) = head_query.single() else { return; };
    let head_angle = f32::atan2(head_dir.dir.x as f32, head_dir.dir.y as f32);
    let head_rotation = Quat::from_rotation_y(head_angle);

    // Collect all LERPed positions into a lookup map to position the capsules perfectly
    let mut interp_positions = vec![Vec3::ZERO; query.iter().count()];

    for (_, segment, grid_pos, prev_pos) in &query {
        let prev_world = Vec3::new(prev_pos.0.x as f32, prev_pos.0.y as f32 * 3.0, prev_pos.0.z as f32);
        let curr_world = Vec3::new(grid_pos.0.x as f32, grid_pos.0.y as f32 * 3.0, grid_pos.0.z as f32);
        let lerped = prev_world.lerp(curr_world, t);

        let idx = segment.index;
        if idx < interp_positions.len() {
            interp_positions[idx] = lerped;
        }
    }

    // Apply translations, rotations, and scales
    for (mut transform, segment, _, _) in &mut query {
        let idx = segment.index;
        if idx == 0 {
            // Head
            transform.translation = interp_positions[0];
            transform.rotation = head_rotation;
            transform.scale = Vec3::new(1.15, 0.9, 1.35);
        } else {
            // Body Capsule connecting segment idx to idx-1
            if idx < interp_positions.len() && (idx - 1) < interp_positions.len() {
                let world_i = interp_positions[idx];
                let world_prev = interp_positions[idx - 1];

                transform.translation = (world_i + world_prev) / 2.0;

                let dir = world_prev - world_i;
                let dist = dir.length();
                if dist > 0.001 {
                    transform.rotation = Quat::from_rotation_arc(Vec3::Y, dir.normalize());
                    transform.scale = Vec3::new(1.0, dist, 1.0);
                } else {
                    transform.scale = Vec3::new(1.0, 0.001, 1.0);
                }
            }
        }
    }
}

pub fn snake_camera_system(
    mut camera_query: Query<(&mut Transform, &mut SnakeCamera)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    state: Res<State<SnakeState>>,
) {
    let Ok((mut transform, mut cam)) = camera_query.single_mut() else { return; };
    let current_state = *state.get();

    let mut auto_reset = cam.auto_reset;
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(el) = document.get_element_by_id("auto-reset-checkbox") {
                    if let Ok(input_el) = el.dyn_into::<web_sys::HtmlInputElement>() {
                        auto_reset = input_el.checked();
                        cam.auto_reset = auto_reset;
                    }
                }
            }
        }
    }

    let is_paused = current_state == SnakeState::Paused;

    // Rotate with right mouse drag when paused
    if is_paused && mouse_input.pressed(MouseButton::Right) {
        for ev in mouse_motion.read() {
            cam.yaw -= ev.delta.x * 0.005;
            cam.pitch = (cam.pitch - ev.delta.y * 0.005).clamp(0.1, 1.4);
        }
    } else if auto_reset && current_state != SnakeState::Paused {
        // Smoothly restore defaults when unpaused
        cam.yaw = cam.yaw * 0.9;
        cam.pitch = cam.pitch + (0.8 - cam.pitch) * 0.1;
    }

    let distance = 35.0;
    let pivot = Vec3::new(0.0, 0.0, 0.0);

    if is_paused || auto_reset {
        let yaw_quat = Quat::from_axis_angle(Vec3::Y, cam.yaw);
        let pitch_quat = Quat::from_axis_angle(Vec3::X, -cam.pitch);
        let rotation = yaw_quat * pitch_quat;
        let offset = rotation * Vec3::new(0.0, 0.0, distance);

        transform.translation = pivot + offset;
        *transform = transform.looking_at(pivot, Vec3::Y);
    }
}

pub fn cleanup_snake(
    mut commands: Commands,
    query: Query<Entity, With<SnakeEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

pub fn reset_snake_state(mut next_state: ResMut<NextState<SnakeState>>) {
    next_state.set(SnakeState::WaitingToStart);
}

// ==========================================
// 5. Bevy Plugin Implementation
// ==========================================

pub struct SnakePlugin;

impl Plugin for SnakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnakeScore>()
            .init_resource::<SnakeTickTimer>()
            .init_state::<SnakeState>()
            .add_systems(
                OnEnter(crate::GameMode::Snake),
                (setup_snake, spawn_snake_hud),
            )
            .add_systems(
                OnExit(crate::GameMode::Snake),
                (cleanup_snake, reset_snake_state),
            )
            .add_systems(
                Update,
                (
                    snake_input_system,
                    snake_tick_system,
                    snake_movement_interpolation_system.after(snake_tick_system),
                    snake_camera_system,
                )
                    .run_if(in_state(crate::GameMode::Snake)),
            );
    }
}
