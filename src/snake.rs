use crate::SnakeEntity;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::wasm_bindgen::JsCast;

#[derive(Resource)]
pub struct SnakeGame {
    pub segments: Vec<IVec3>, // x, y (floor index: 0=lower, 1=upper), z
    pub direction: IVec2,
    pub next_direction: IVec2,
    pub food: IVec3,
    pub score: u32,
    pub high_score: u32,
    pub game_over: bool,
    pub paused: bool,
    pub waiting_to_start: bool,
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
    pub tick_timer: Timer,
    pub mesh_update_needed: bool,
}

#[derive(Component)]
pub struct SnakeBodySegment;

#[derive(Component)]
pub struct SnakeFood;

#[derive(Component)]
pub struct SnakeHudText;

#[derive(Component)]
pub struct SnakeCamera;

// 4 Vent locations on the lower floor
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
        // 50% probability to spawn on upper floor
        let floor = if get_random_value() < 0.5 { 1 } else { 0 };

        let (x, z) = if floor == 1 {
            // Upper floor boundary: -9 to 9
            let rx = (get_random_value() * 19.0) as i32 - 9;
            let rz = (get_random_value() * 19.0) as i32 - 9;
            (rx, rz)
        } else {
            // Lower floor boundary: -15 to 15
            let rx = (get_random_value() * 31.0) as i32 - 15;
            let rz = (get_random_value() * 31.0) as i32 - 15;
            (rx, rz)
        };

        let pos = IVec3::new(x, floor, z);

        // Prevent spawning food on the vents or on the snake
        let on_vent = floor == 0
            && (IVec2::new(x, z) == VENT_1
                || IVec2::new(x, z) == VENT_2
                || IVec2::new(x, z) == VENT_3
                || IVec2::new(x, z) == VENT_4);
        if !segments.contains(&pos) && !on_vent {
            return pos;
        }
    }
}

pub fn setup_snake(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.2, 0.2, 0.4),
        brightness: 400.0,
        ..default()
    });

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 6000.0,
            ..default()
        },
        Transform::from_xyz(15.0, 45.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        SnakeEntity,
    ));

    // Default position pulled back to fit the wider 31x31 arena
    let default_pos = Vec3::new(0.0, 26.0, 24.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(default_pos.x, default_pos.y, default_pos.z)
            .looking_at(Vec3::ZERO, Vec3::Y),
        SnakeCamera,
        SnakeEntity,
    ));

    // ==========================================
    // 1. Lower Floor (Y = 0) - Size 31x31
    // ==========================================
    let lower_board_mesh = meshes.add(Cuboid::new(31.0, 0.1, 31.0));
    let lower_board_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.03, 0.04, 0.07),
        perceptual_roughness: 0.7,
        metallic: 0.3,
        ..default()
    });
    commands.spawn((
        Mesh3d(lower_board_mesh),
        MeshMaterial3d(lower_board_material),
        Transform::from_xyz(0.0, -0.55, 0.0),
        SnakeEntity,
    ));

    // Lower borders (Purple neon) - Adjusted for 31x31
    let border_mesh_h = meshes.add(Cuboid::new(31.2, 0.5, 0.2));
    let border_mesh_v = meshes.add(Cuboid::new(0.2, 0.5, 31.2));
    let border_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.8, 1.0),
        emissive: LinearRgba::rgb(2.5, 0.0, 5.0),
        perceptual_roughness: 0.1,
        ..default()
    });

    commands.spawn((
        Mesh3d(border_mesh_h.clone()),
        MeshMaterial3d(border_material.clone()),
        Transform::from_xyz(0.0, -0.3, -15.6),
        SnakeEntity,
    ));
    commands.spawn((
        Mesh3d(border_mesh_h),
        MeshMaterial3d(border_material.clone()),
        Transform::from_xyz(0.0, -0.3, 15.6),
        SnakeEntity,
    ));
    commands.spawn((
        Mesh3d(border_mesh_v.clone()),
        MeshMaterial3d(border_material.clone()),
        Transform::from_xyz(-15.6, -0.3, 0.0),
        SnakeEntity,
    ));
    commands.spawn((
        Mesh3d(border_mesh_v),
        MeshMaterial3d(border_material),
        Transform::from_xyz(15.6, -0.3, 0.0),
        SnakeEntity,
    ));

    // ==========================================
    // 2. Upper Floor (Y = 3.0) - Size 19x19
    // ==========================================
    let upper_board_mesh = meshes.add(Cuboid::new(19.0, 0.1, 19.0));
    let upper_board_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.05, 0.25, 0.45, 0.45),
        perceptual_roughness: 0.15,
        metallic: 0.2,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(upper_board_mesh),
        MeshMaterial3d(upper_board_material),
        Transform::from_xyz(0.0, 2.45, 0.0),
        SnakeEntity,
    ));

    // Upper borders (Cyan neon) - Open East & West for dropping down
    let upper_border_mesh_h = meshes.add(Cuboid::new(19.2, 0.3, 0.15));
    let upper_border_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.8),
        emissive: LinearRgba::rgb(0.0, 4.0, 3.2),
        perceptual_roughness: 0.1,
        ..default()
    });

    commands.spawn((
        Mesh3d(upper_border_mesh_h.clone()),
        MeshMaterial3d(upper_border_material.clone()),
        Transform::from_xyz(0.0, 2.6, -9.6),
        SnakeEntity,
    ));
    commands.spawn((
        Mesh3d(upper_border_mesh_h),
        MeshMaterial3d(upper_border_material),
        Transform::from_xyz(0.0, 2.6, 9.6),
        SnakeEntity,
    ));

    // ==========================================
    // 3. Updraft Steam Vents (Lower -> Upper)
    // ==========================================
    let vent_base_mesh = meshes.add(Cylinder::new(0.8, 0.1));
    let vent_base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.3),
        emissive: LinearRgba::rgb(0.0, 3.0, 0.5),
        ..default()
    });

    let wind_mesh = meshes.add(Cylinder::new(0.65, 3.0));
    let wind_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 1.0, 0.4, 0.15),
        emissive: LinearRgba::rgb(0.0, 3.0, 0.8),
        alpha_mode: AlphaMode::Blend,
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
            SnakeEntity,
        ));
    }

    // Initialize Game state
    let initial_segments = vec![
        IVec3::new(0, 0, 0),
        IVec3::new(-1, 0, 0),
        IVec3::new(-2, 0, 0),
    ];
    let food_pos = get_random_food_pos(&initial_segments);

    commands.insert_resource(SnakeGame {
        segments: initial_segments,
        direction: IVec2::new(1, 0),
        next_direction: IVec2::new(1, 0),
        food: food_pos,
        score: 0,
        high_score: 0,
        game_over: false,
        paused: false,
        waiting_to_start: true, // Wait for first keypress to begin moving
        orbit_yaw: 0.0,
        orbit_pitch: 0.8,
        tick_timer: Timer::from_seconds(0.14, TimerMode::Repeating),
        mesh_update_needed: true,
    });
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
    mut game: ResMut<SnakeGame>,
    mut hud_query: Query<&mut Text, With<SnakeHudText>>,
) {
    // 1. Spacebar Pause Toggle
    if keys.just_pressed(KeyCode::Space) {
        if !game.game_over && !game.waiting_to_start {
            game.paused = !game.paused;
            update_hud_text(&game, &mut hud_query);
        }
    }

    if game.game_over || game.paused {
        return;
    }

    // 2. Direction input detection
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
        if new_dir + game.direction != IVec2::ZERO {
            game.next_direction = new_dir;
            if game.waiting_to_start {
                game.waiting_to_start = false;
                update_hud_text(&game, &mut hud_query);
            }
        }
    }
}

fn update_hud_text(game: &SnakeGame, hud_query: &mut Query<&mut Text, With<SnakeHudText>>) {
    for mut text in hud_query {
        if game.game_over {
            *text = Text::new(format!(
                "GAME OVER\nScore: {}\nHigh Score: {}\nPress 'R' to Restart",
                game.score, game.high_score
            ));
        } else if game.waiting_to_start {
            *text = Text::new(format!(
                "READY\nPress W/A/S/D or Arrows to Start\nScore: {}\nHigh Score: {}",
                game.score, game.high_score
            ));
        } else if game.paused {
            *text = Text::new(format!(
                "PAUSED (Orbit Mode)\nDrag Mouse to Rotate View\nPress [Space] to Resume\nScore: {}\nHigh Score: {}",
                game.score, game.high_score
            ));
        } else {
            *text = Text::new(format!(
                "Score: {}\nHigh Score: {}\n[Space] Pause & Orbit view",
                game.score, game.high_score
            ));
        }
    }
}

pub fn snake_tick_system(
    time: Res<Time>,
    mut game: ResMut<SnakeGame>,
    mut hud_query: Query<&mut Text, With<SnakeHudText>>,
) {
    if game.game_over || game.paused || game.waiting_to_start {
        return;
    }

    game.tick_timer.tick(time.delta());
    if !game.tick_timer.is_finished() {
        return;
    }

    game.direction = game.next_direction;

    let head = game.segments[0];

    // Planar step forward
    let mut new_head = IVec3::new(head.x + game.direction.x, head.y, head.z + game.direction.y);

    // ==========================================
    // Height & Fall/Launch Mechanics
    // ==========================================
    if head.y == 1 {
        // Upper floor boundary: -9 to 9
        if new_head.x < -9 || new_head.x > 9 || new_head.z < -9 || new_head.z > 9 {
            new_head.y = 0;
        }
    }

    if new_head.y == 0 {
        // Lower floor boundary: -15 to 15
        if new_head.x < -15 || new_head.x > 15 || new_head.z < -15 || new_head.z > 15 {
            game.game_over = true;
            game.mesh_update_needed = true;
            update_hud_text(&game, &mut hud_query);
            return;
        }

        // Check if landing on any of the 4 Updraft Vents
        let new_head_xz = IVec2::new(new_head.x, new_head.z);
        if new_head_xz == VENT_1
            || new_head_xz == VENT_2
            || new_head_xz == VENT_3
            || new_head_xz == VENT_4
        {
            new_head.y = 1;
        }
    }

    let is_eating = new_head == game.food;
    let check_segments = if is_eating {
        &game.segments[..]
    } else {
        &game.segments[..game.segments.len() - 1]
    };

    // Self-collision
    if check_segments.contains(&new_head) {
        game.game_over = true;
        game.mesh_update_needed = true;
        update_hud_text(&game, &mut hud_query);
        return;
    }

    // Move snake
    game.segments.insert(0, new_head);
    if is_eating {
        game.score += 10;
        if game.score > game.high_score {
            game.high_score = game.score;
        }
        game.food = get_random_food_pos(&game.segments);
    } else {
        game.segments.pop();
    }

    game.mesh_update_needed = true;
    update_hud_text(&game, &mut hud_query);
}

// Camera controller with DOM settings inspection
pub fn snake_camera_system(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut game: ResMut<SnakeGame>,
    mut camera_query: Query<&mut Transform, With<SnakeCamera>>,
) {
    let default_pos = Vec3::new(0.0, 26.0, 24.0);
    let pivot = Vec3::new(0.0, 1.0, 0.0);
    let distance = (default_pos - pivot).length();

    // Read DOM to check whether auto-reset checkbox is checked
    #[cfg(target_arch = "wasm32")]
    let auto_reset = {
        let mut val = true;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(el) = document.get_element_by_id("camera-reset-checkbox") {
                    if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                        val = input.checked();
                    }
                }
            }
        }
        val
    };
    #[cfg(not(target_arch = "wasm32"))]
    let auto_reset = true;

    if game.paused && !game.game_over {
        // 1. Mouse Drag/Motion orbit camera rotation
        let mut delta = Vec2::ZERO;
        for event in mouse_motion_events.read() {
            delta += event.delta;
        }

        if delta != Vec2::ZERO {
            let sensitivity = 0.005;
            game.orbit_yaw -= delta.x * sensitivity;
            game.orbit_pitch -= delta.y * sensitivity;
            // Clamp pitch so the camera never goes below the floor or crosses the vertical poles
            game.orbit_pitch = game.orbit_pitch.clamp(0.1, 1.4);
        }
    } else if auto_reset {
        // 2. Smoothly restore angles to defaults when unpaused & auto_reset is true
        game.orbit_yaw = game.orbit_yaw * 0.9;
        game.orbit_pitch = game.orbit_pitch + (0.8 - game.orbit_pitch) * 0.1;
    }

    // Apply the yaw/pitch to position and rotate the camera
    if (game.paused && !game.game_over) || auto_reset {
        for mut transform in &mut camera_query {
            let yaw_quat = Quat::from_axis_angle(Vec3::Y, game.orbit_yaw);
            let pitch_quat = Quat::from_axis_angle(Vec3::X, -game.orbit_pitch);
            let rotation = yaw_quat * pitch_quat;
            let offset = rotation * Vec3::new(0.0, 0.0, distance);

            transform.translation = pivot + offset;
            *transform = transform.looking_at(pivot, Vec3::Y);
        }
    }
}

pub fn snake_render_update_system(
    mut commands: Commands,
    mut game: ResMut<SnakeGame>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    segments_query: Query<Entity, With<SnakeBodySegment>>,
    food_query: Query<Entity, With<SnakeFood>>,
) {
    if !game.mesh_update_needed {
        return;
    }
    game.mesh_update_needed = false;

    for entity in &segments_query {
        commands.entity(entity).despawn();
    }
    for entity in &food_query {
        commands.entity(entity).despawn();
    }

    let segment_mesh = meshes.add(Cuboid::new(0.9, 0.9, 0.9));
    let head_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.9,
        ..default()
    });
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.1, 0.8),
        perceptual_roughness: 0.2,
        metallic: 0.8,
        ..default()
    });

    for (i, &pos) in game.segments.iter().enumerate() {
        let mat = if i == 0 {
            head_material.clone()
        } else {
            body_material.clone()
        };
        let world_y = pos.y as f32 * 3.0;
        commands.spawn((
            Mesh3d(segment_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(pos.x as f32, world_y, pos.z as f32),
            SnakeBodySegment,
            SnakeEntity,
        ));
    }

    let food_mesh = meshes.add(Sphere::new(0.45));
    let food_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.1),
        emissive: LinearRgba::rgb(3.0, 0.0, 0.0),
        perceptual_roughness: 0.0,
        ..default()
    });

    let food_world_y = game.food.y as f32 * 3.0;
    commands.spawn((
        Mesh3d(food_mesh),
        MeshMaterial3d(food_material),
        Transform::from_xyz(game.food.x as f32, food_world_y, game.food.z as f32),
        SnakeFood,
        SnakeEntity,
    ));
}

pub fn snake_restart_system(
    keys: Res<ButtonInput<KeyCode>>,
    game: Option<ResMut<SnakeGame>>,
    mut hud_query: Query<&mut Text, With<SnakeHudText>>,
) {
    let Some(mut game) = game else {
        return;
    };
    if game.game_over && keys.just_pressed(KeyCode::KeyR) {
        let initial_segments = vec![
            IVec3::new(0, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(-2, 0, 0),
        ];
        let food_pos = get_random_food_pos(&initial_segments);
        game.segments = initial_segments;
        game.direction = IVec2::new(1, 0);
        game.next_direction = IVec2::new(1, 0);
        game.food = food_pos;
        game.score = 0;
        game.game_over = false;
        game.paused = false;
        game.waiting_to_start = true;
        game.orbit_yaw = 0.0;
        game.orbit_pitch = 0.8;
        game.mesh_update_needed = true;
        update_hud_text(&game, &mut hud_query);
    }
}
