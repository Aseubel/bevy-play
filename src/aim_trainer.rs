use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions};

#[derive(Resource)]
pub struct AimTrainerState {
    pub score: u32,
    pub shots_fired: u32,
    pub time_left: f32,
    pub game_over: bool,
}

#[derive(Resource)]
pub struct AimTrainerAssets {
    pub sphere_mesh: Handle<Mesh>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
pub struct Balloon {
    pub radius: f32,
}

#[derive(Component)]
pub struct AimTrainerCamera {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Component)]
pub struct AimTrainerCleanup;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct AccuracyText;

#[derive(Component)]
pub struct BpmText;

#[derive(Component)]
pub struct TimerText;

#[derive(Component)]
pub struct GameOverText;

#[derive(Component)]
pub struct FinalScoreText;

#[derive(Component)]
pub struct FinalAccuracyText;

#[derive(Component)]
pub struct FinalBpmText;

// Thread-safe random value generator compatible with both WebAssembly and Native.
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

pub fn setup_aim_trainer(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut window_query: Query<&mut CursorOptions, With<Window>>,
) {
    // 1. Initialize State Resource
    commands.insert_resource(AimTrainerState {
        score: 0,
        shots_fired: 0,
        time_left: 60.0,
        game_over: false,
    });

    // Pre-allocate target sphere mesh and materials to prevent runtime allocation stuttering.
    // The sphere mesh has a unit radius of 1.0; we scale the transform to set target sizes.
    let sphere_mesh = meshes.add(Sphere::new(1.0));
    let colors = [
        Color::srgb(0.95, 0.2, 0.2),   // Neon Red
        Color::srgb(0.2, 0.85, 0.2),   // Neon Green
        Color::srgb(0.2, 0.5, 0.95),   // Neon Blue
        Color::srgb(0.95, 0.8, 0.1),   // Neon Yellow
        Color::srgb(0.7, 0.2, 0.9),    // Neon Purple
    ];
    let mut pre_materials = Vec::new();
    for color in colors {
        let linear_color = LinearRgba::from(color);
        pre_materials.push(materials.add(StandardMaterial {
            base_color: color,
            emissive: linear_color * 0.25, // Sleek self-illumination glow
            perceptual_roughness: 0.15,
            metallic: 0.1,
            ..default()
        }));
    }
    commands.insert_resource(AimTrainerAssets {
        sphere_mesh,
        materials: pre_materials,
    });

    // 2. Configure Global Ambient Light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 450.0,
        ..default()
    });

    // 3. Spawn Directional Light
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        AimTrainerCleanup,
    ));

    // 4. Spawn 3D First Person Camera (positioned at eye height 1.8)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.8, 0.0),
        AimTrainerCamera {
            yaw: 0.0,
            pitch: 0.0,
        },
        AimTrainerCleanup,
    ));

    // 5. Hide and Lock Cursor immediately
    for mut options in &mut window_query {
        options.visible = false;
        options.grab_mode = CursorGrabMode::Locked;
    }

    // 6. Build Arena (Floor, Ceiling, Walls, Columns)
    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.15),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -5.0),
        AimTrainerCleanup,
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 0.1, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.18, 0.22),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 8.0, -5.0),
        AimTrainerCleanup,
    ));

    // Front Wall (target wall at Z = -20.0)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 8.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.22, 0.25, 0.28),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 4.0, -20.0),
        AimTrainerCleanup,
    ));

    // Left Wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 8.0, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.16, 0.18),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(-15.0, 4.0, -5.0),
        AimTrainerCleanup,
    ));

    // Right Wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 8.0, 30.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.16, 0.18),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(15.0, 4.0, -5.0),
        AimTrainerCleanup,
    ));

    // Back Wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(30.0, 8.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.16, 0.18),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 4.0, 10.0),
        AimTrainerCleanup,
    ));

    // Reference side columns
    // Left column
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 8.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.35, 0.65),
            perceptual_roughness: 0.4,
            ..default()
        })),
        Transform::from_xyz(-9.0, 4.0, -18.0),
        AimTrainerCleanup,
    ));

    // Right column
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 8.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.35, 0.65),
            perceptual_roughness: 0.4,
            ..default()
        })),
        Transform::from_xyz(9.0, 4.0, -18.0),
        AimTrainerCleanup,
    ));

    // 7. Spawn Green Crosshair UI (Single thick circular dot)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(6.0),
            height: Val::Px(6.0),
            margin: UiRect {
                left: Val::Px(-3.0),
                top: Val::Px(-3.0),
                ..default()
            },
            border_radius: BorderRadius::all(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.0, 1.0, 0.0)),
        AimTrainerCleanup,
    ));

    // 8. Spawn Scoreboard & Info HUD
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        AimTrainerCleanup,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Score: 0"),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::WHITE),
            ScoreText,
        ));
        parent.spawn((
            Text::new("Accuracy: 100.0%"),
            TextFont { font_size: FontSize::Px(20.0), ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            AccuracyText,
        ));
        parent.spawn((
            Text::new("BPM: 0.0"),
            TextFont { font_size: FontSize::Px(20.0), ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            BpmText,
        ));
        parent.spawn((
            Text::new("Time Left: 60.0s"),
            TextFont { font_size: FontSize::Px(20.0), ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TimerText,
        ));
        parent.spawn((
            Text::new("Aim & Left Click to shoot the suspended targets. Esc to unlock mouse."),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
    });

    // 9. Spawn Game Over UI (Hidden initially)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(40.0),
            width: Val::Px(340.0),
            height: Val::Px(200.0),
            margin: UiRect {
                left: Val::Px(-170.0),
                top: Val::Px(-100.0),
                ..default()
            },
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
        GameOverText,
        AimTrainerCleanup,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("GAME OVER"),
            TextFont { font_size: FontSize::Px(36.0), ..default() },
            TextColor(Color::srgb(0.95, 0.2, 0.2)),
        ));
        parent.spawn((
            Text::new("Final Score: 0"),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::WHITE),
            FinalScoreText,
        ));
        parent.spawn((
            Text::new("Accuracy: 100.0%"),
            TextFont { font_size: FontSize::Px(20.0), ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            FinalAccuracyText,
        ));
        parent.spawn((
            Text::new("BPM: 0.0"),
            TextFont { font_size: FontSize::Px(20.0), ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            FinalBpmText,
        ));
        parent.spawn((
            Text::new("Press R to Restart"),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
        ));
    });
}

pub fn cleanup_aim_trainer(
    mut commands: Commands,
    query: Query<Entity, With<AimTrainerCleanup>>,
    mut window_query: Query<&mut CursorOptions, With<Window>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<AimTrainerState>();
    commands.remove_resource::<AimTrainerAssets>();

    // Restore standard cursor options
    for mut options in &mut window_query {
        options.visible = true;
        options.grab_mode = CursorGrabMode::None;
    }
}

pub fn aim_trainer_mouse_look_system(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mut query: Query<(&mut AimTrainerCamera, &mut Transform)>,
    window: Single<&CursorOptions, With<Window>>,
) {
    // Only look around if mouse is locked
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

    for (mut camera, mut transform) in &mut query {
        let sensitivity = 0.0015;
        camera.yaw -= delta.x * sensitivity;
        camera.pitch -= delta.y * sensitivity;
        
        // Clamp pitch to prevent flipping upside down
        camera.pitch = camera.pitch.clamp(-1.54, 1.54);

        transform.rotation = Quat::from_axis_angle(Vec3::Y, camera.yaw)
            * Quat::from_axis_angle(Vec3::X, camera.pitch);
    }
}

pub fn aim_trainer_shoot_system(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<AimTrainerState>,
    camera_param: Single<&Transform, With<AimTrainerCamera>>,
    balloon_query: Query<(Entity, &Transform, &Balloon)>,
) {
    if state.game_over {
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        state.shots_fired += 1;

        let camera_transform = *camera_param;
        let ray_origin = camera_transform.translation;
        let ray_dir = camera_transform.forward();

        let mut closest_balloon: Option<(Entity, f32)> = None;

        for (balloon_entity, transform, balloon) in &balloon_query {
            let sphere_center = transform.translation;
            let sphere_radius = balloon.radius;

            let l = sphere_center - ray_origin;
            let t_ca = l.dot(*ray_dir);
            if t_ca < 0.0 {
                continue;
            }

            let d2 = l.dot(l) - t_ca * t_ca;
            let r2 = sphere_radius * sphere_radius;
            if d2 <= r2 {
                let t_hc = (r2 - d2).sqrt();
                let t = t_ca - t_hc;
                if closest_balloon.map_or(true, |(_, closest_t)| t < closest_t) {
                    closest_balloon = Some((balloon_entity, t));
                }
            }
        }

        if let Some((hit_entity, _)) = closest_balloon {
            commands.entity(hit_entity).despawn();
            state.score += 1;
        }
    }
}

pub fn aim_trainer_balloon_maintainer_system(
    mut commands: Commands,
    assets: Res<AimTrainerAssets>,
    balloon_query: Query<(), With<Balloon>>,
    state: Res<AimTrainerState>,
) {
    if state.game_over {
        return;
    }

    let count = balloon_query.iter().count();
    if count < 3 {
        for _ in 0..(3 - count) {
            // Spawn a static sphere at a random position in front of the target wall
            let x = (get_random_value() - 0.5) * 11.0;   // range: -5.5 to 5.5 (fits within horizontal FOV)
            let y = 1.5 + get_random_value() * 3.0;      // range: 1.5 to 4.5 (centered around eye height 1.8)
            let z = -18.0;                               // fixed depth (consistent targeting depth)
            
            // Random size for the targets (diameter on screen matches standard mouse cursor: radius between 0.18 and 0.23)
            let radius = 0.18 + get_random_value() * 0.05;
            
            // Pick a random pre-allocated material to avoid shader recompilation stutters
            let mat_index = (get_random_value() * assets.materials.len() as f32).floor() as usize;
            let sphere_mat = assets.materials[mat_index.clamp(0, assets.materials.len() - 1)].clone();

            commands.spawn((
                Mesh3d(assets.sphere_mesh.clone()),
                MeshMaterial3d(sphere_mat),
                // We use transform scale to represent the target radius, which is extremely high performance
                Transform::from_xyz(x, y, z).with_scale(Vec3::splat(radius)),
                Balloon { radius },
                AimTrainerCleanup,
            ));
        }
    }
}

pub fn aim_trainer_hud_update_system(
    state: Res<AimTrainerState>,
    mut score_query: Query<&mut Text, (With<ScoreText>, Without<AccuracyText>, Without<BpmText>, Without<TimerText>)>,
    mut accuracy_query: Query<&mut Text, (With<AccuracyText>, Without<ScoreText>, Without<BpmText>, Without<TimerText>)>,
    mut bpm_query: Query<&mut Text, (With<BpmText>, Without<ScoreText>, Without<AccuracyText>, Without<TimerText>)>,
    mut timer_query: Query<&mut Text, (With<TimerText>, Without<ScoreText>, Without<AccuracyText>, Without<BpmText>)>,
) {
    if state.game_over {
        return;
    }

    for mut text in &mut score_query {
        *text = Text::new(format!("Score: {}", state.score));
    }

    let accuracy = if state.shots_fired > 0 {
        (state.score as f32 / state.shots_fired as f32) * 100.0
    } else {
        100.0
    };
    for mut text in &mut accuracy_query {
        *text = Text::new(format!("Accuracy: {:.1}%", accuracy));
    }

    let elapsed = 60.0 - state.time_left;
    let bpm = if elapsed > 0.5 {
        (state.score as f32 / elapsed) * 60.0
    } else {
        0.0
    };
    for mut text in &mut bpm_query {
        *text = Text::new(format!("BPM: {:.1}", bpm));
    }

    for mut text in &mut timer_query {
        *text = Text::new(format!("Time Left: {:.1}s", state.time_left));
    }
}

pub fn aim_trainer_timer_system(
    time: Res<Time>,
    mut state: ResMut<AimTrainerState>,
    mut game_over_query: Query<&mut Visibility, With<GameOverText>>,
    mut final_score_query: Query<&mut Text, (With<FinalScoreText>, Without<FinalAccuracyText>, Without<FinalBpmText>)>,
    mut final_accuracy_query: Query<&mut Text, (With<FinalAccuracyText>, Without<FinalScoreText>, Without<FinalBpmText>)>,
    mut final_bpm_query: Query<&mut Text, (With<FinalBpmText>, Without<FinalScoreText>, Without<FinalAccuracyText>)>,
    mut window_query: Query<&mut CursorOptions, With<Window>>,
) {
    if state.game_over {
        return;
    }

    state.time_left -= time.delta_secs();
    if state.time_left <= 0.0 {
        state.time_left = 0.0;
        state.game_over = true;

        // Show game over overlay
        for mut visibility in &mut game_over_query {
            *visibility = Visibility::Visible;
        }

        // Calculate final stats
        let accuracy = if state.shots_fired > 0 {
            (state.score as f32 / state.shots_fired as f32) * 100.0
        } else {
            100.0
        };
        let elapsed = 60.0; // standard time limit is 60s
        let bpm = (state.score as f32 / elapsed) * 60.0;

        // Display final scoreboard
        for mut text in &mut final_score_query {
            *text = Text::new(format!("Final Score: {}", state.score));
        }
        for mut text in &mut final_accuracy_query {
            *text = Text::new(format!("Accuracy: {:.1}%", accuracy));
        }
        for mut text in &mut final_bpm_query {
            *text = Text::new(format!("BPM: {:.1}", bpm));
        }

        // Release cursor grab
        for mut options in &mut window_query {
            options.visible = true;
            options.grab_mode = CursorGrabMode::None;
        }
    }
}

pub fn aim_trainer_cursor_lock_system(
    state: Res<AimTrainerState>,
    mut window_query: Query<&mut CursorOptions, With<Window>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if state.game_over {
        return;
    }

    for mut options in &mut window_query {
        // Lock cursor on left click
        if mouse.just_pressed(MouseButton::Left) {
            options.visible = false;
            options.grab_mode = CursorGrabMode::Locked;
        }
        // Unlock cursor on Escape
        if keys.just_pressed(KeyCode::Escape) {
            options.visible = true;
            options.grab_mode = CursorGrabMode::None;
        }
    }
}

pub fn aim_trainer_restart_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<AimTrainerState>,
    mut game_over_query: Query<&mut Visibility, With<GameOverText>>,
    mut window_query: Query<&mut CursorOptions, With<Window>>,
    balloon_query: Query<Entity, With<Balloon>>,
) {
    if state.game_over && keys.just_pressed(KeyCode::KeyR) {
        // Despawn existing spheres
        for entity in &balloon_query {
            commands.entity(entity).despawn();
        }

        // Reset state values
        state.score = 0;
        state.shots_fired = 0;
        state.time_left = 60.0;
        state.game_over = false;

        // Hide Game Over UI panel
        for mut visibility in &mut game_over_query {
            *visibility = Visibility::Hidden;
        }

        // Relock cursor
        for mut options in &mut window_query {
            options.visible = false;
            options.grab_mode = CursorGrabMode::Locked;
        }
    }
}

pub struct AimTrainerPlugin;

impl Plugin for AimTrainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::GameMode::AimTrainer),
            setup_aim_trainer,
        )
        .add_systems(
            OnExit(crate::GameMode::AimTrainer),
            cleanup_aim_trainer,
        )
        .add_systems(
            Update,
            (
                aim_trainer_mouse_look_system,
                aim_trainer_shoot_system,
                aim_trainer_balloon_maintainer_system,
                aim_trainer_hud_update_system,
                aim_trainer_timer_system,
                aim_trainer_cursor_lock_system,
                aim_trainer_restart_system,
            )
                .run_if(in_state(crate::GameMode::AimTrainer)),
        );
    }
}
