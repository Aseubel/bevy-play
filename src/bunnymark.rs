use bevy::prelude::*;

#[derive(Component)]
pub struct SpriteVelocity(pub Vec2);

#[derive(Component)]
pub struct Bunny;

#[derive(Component)]
pub struct BunnyLife {
    pub energy: f32,       // Ranges from 1.0 (spawn) down to 0.0 (death)
    pub decay_rate: f32,   // Rate of energy decay per second
}

#[derive(Component)]
pub struct BunnymarkCamera;

#[derive(Component)]
pub struct BunnymarkHud;

#[derive(Component)]
pub struct BunnymarkCounter;

// Thread-safe random value generator compatible with both native and WebAssembly.
fn get_random_value() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Math::random() as f32
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEED: AtomicU64 = AtomicU64::new(987654321);
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

// Generate random velocity vector with speed between 150.0 and 500.0
fn random_velocity() -> Vec2 {
    let angle = get_random_value() * std::f32::consts::TAU;
    let speed = 150.0 + get_random_value() * 350.0;
    Vec2::new(angle.cos() * speed, angle.sin() * speed)
}

pub fn setup_bunnymark(
    mut commands: Commands,
    window: Single<&Window>,
) {
    // 1. Spawn the 2D Camera
    commands.spawn((
        Camera2d::default(),
        BunnymarkCamera,
    ));

    // Get current window bounds to distribute initial bunnies
    let width = window.width();
    let height = window.height();

    // 2. Spawn 1,000 initial bunnies (as White Sprite blocks)
    for _ in 0..1000 {
        let x = (get_random_value() - 0.5) * width;
        let y = (get_random_value() - 0.5) * height;
        let decay_rate = 1.0 / (2.0 + get_random_value() * 3.5); // life span: 2.0 to 5.5s
        
        commands.spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(8.0, 8.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            SpriteVelocity(random_velocity()),
            BunnyLife {
                energy: 1.0,
                decay_rate,
            },
            Bunny,
        ));
    }

    // 3. Spawn the HUD
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        BunnymarkHud,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Bunnies: 1000"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
            BunnymarkCounter,
        ));
        parent.spawn((
            Text::new("Click Left Click to spawn 1000 bunnies at cursor"),
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
                Text::new("Clear All (C)"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

pub fn cleanup_bunnymark(
    mut commands: Commands,
    bunnies: Query<Entity, With<Bunny>>,
    camera: Query<Entity, With<BunnymarkCamera>>,
    hud: Query<Entity, With<BunnymarkHud>>,
) {
    for entity in &bunnies {
        commands.entity(entity).despawn();
    }
    for entity in &camera {
        commands.entity(entity).despawn();
    }
    for entity in &hud {
        commands.entity(entity).despawn();
    }
}

pub fn move_bunnies_system(
    time: Res<Time>,
    window: Single<&Window>,
    par_commands: ParallelCommands,
    mut query: Query<(Entity, &mut SpriteVelocity, &mut Transform, &mut Sprite, &mut BunnyLife), With<Bunny>>,
) {
    let dt = time.delta_secs();
    let width = window.width();
    let height = window.height();
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    
    // Base half size of bunny sprite is 4.0
    let base_half_bunny = 4.0;

    // Use par_iter_mut to run parallel physics updates across all available cores
    query.par_iter_mut().for_each(|(entity, mut vel, mut transform, mut sprite, mut life)| {
        // 1. Decay the life energy over time
        life.energy -= life.decay_rate * dt;

        // 2. Despawn the entity safely using ParallelCommands if life is depleted
        if life.energy <= 0.0 {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).despawn();
            });
            return;
        }

        // 3. Physical laws simulation (Gravity + Air Drag + Bounce restitution)
        const GRAVITY: f32 = 700.0;       // Pulls bunnies down
        const DRAG: f32 = 0.2;           // Gentle air resistance
        const RESTITUTION: f32 = 0.82;   // Bounces lose 18% of kinetic energy

        // Apply gravity
        vel.0.y -= GRAVITY * dt;

        // Apply drag
        vel.0 *= 1.0 - DRAG * dt;

        // Update positions
        transform.translation.x += vel.0.x * dt;
        transform.translation.y += vel.0.y * dt;

        // Scale bounding box with bunny size
        let half_bunny = base_half_bunny * life.energy;

        // Bounce off X bounds
        if transform.translation.x > half_w - half_bunny {
            transform.translation.x = half_w - half_bunny;
            vel.0.x = -vel.0.x.abs() * RESTITUTION;
        } else if transform.translation.x < -half_w + half_bunny {
            transform.translation.x = -half_w + half_bunny;
            vel.0.x = vel.0.x.abs() * RESTITUTION;
        }

        // Bounce off Y bounds
        if transform.translation.y > half_h - half_bunny {
            transform.translation.y = half_h - half_bunny;
            vel.0.y = -vel.0.y.abs() * RESTITUTION;
        } else if transform.translation.y < -half_h + half_bunny {
            transform.translation.y = -half_h + half_bunny;
            vel.0.y = vel.0.y.abs() * RESTITUTION;
        }

        // 4. Shrink size and fade transparency (change Alpha)
        let size = 8.0 * life.energy;
        sprite.custom_size = Some(Vec2::new(size, size));
        
        // Update color alpha for gradual fade out
        sprite.color = Color::srgba(1.0, 1.0, 1.0, life.energy);
    });
}

pub fn handle_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_param: Single<(&Camera, &GlobalTransform), With<BunnymarkCamera>>,
    mut commands: Commands,
) {
    // Check if the left mouse button is just pressed
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(cursor_pos) = window.cursor_position() {
            let (camera, camera_transform) = *camera_param;
            if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                for _ in 0..1000 {
                    let decay_rate = 1.0 / (2.0 + get_random_value() * 3.5);
                    commands.spawn((
                        Sprite {
                            color: Color::WHITE,
                            custom_size: Some(Vec2::new(8.0, 8.0)),
                            ..default()
                        },
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                        SpriteVelocity(random_velocity()),
                        BunnyLife {
                            energy: 1.0,
                            decay_rate,
                        },
                        Bunny,
                    ));
                }
            }
        }
    }
}

pub fn update_hud_system(
    bunnies: Query<(), With<Bunny>>,
    mut query: Query<&mut Text, With<BunnymarkCounter>>,
) {
    let count = bunnies.iter().count();
    for mut text in &mut query {
        *text = Text::new(format!("Bunnies: {}", count));
    }
}

pub fn bunnymark_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    bunnies: Query<Entity, With<Bunny>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    let mut should_clear = keys.just_pressed(KeyCode::KeyC);

    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));
                should_clear = true;
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
            }
        }
    }

    if should_clear {
        for entity in &bunnies {
            commands.entity(entity).despawn();
        }
    }
}

pub struct BunnymarkPlugin;

impl Plugin for BunnymarkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::GameMode::Bunnymark),
            setup_bunnymark,
        )
        .add_systems(
            OnExit(crate::GameMode::Bunnymark),
            cleanup_bunnymark,
        )
        .add_systems(
            Update,
            (
                move_bunnies_system,
                handle_click_system,
                update_hud_system,
                bunnymark_input_system,
            )
                .run_if(in_state(crate::GameMode::Bunnymark)),
        );
    }
}
