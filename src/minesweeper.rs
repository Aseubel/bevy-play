use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellType {
    Empty,
    Mine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub cell_type: CellType,
    pub neighbor_mines: u8,
    pub is_revealed: bool,
    pub is_flagged: bool,
    pub entity: Option<Entity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, States, Default)]
pub enum GameStatus {
    #[default]
    Playing,
    Won,
    Lost,
}

#[derive(Resource)]
pub struct MinesweeperBoard {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub grid: Vec<Vec<Cell>>,
    pub state: GameStatus,
    pub flags_placed: usize,
    pub is_generated: bool,
}

#[derive(Resource)]
pub struct MinesweeperAssets {
    pub bomb: Handle<Image>,
}

#[derive(Component)]
pub struct MinesweeperCell {
    pub x: usize,
    pub y: usize,
}

#[derive(Component)]
pub struct MinesweeperEntity;

#[derive(Component)]
pub struct MinesweeperText;

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

fn is_solvable(
    width: usize,
    height: usize,
    start_x: usize,
    start_y: usize,
    grid_setup: &Vec<Vec<CellType>>,
) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum VirtualState {
        Hidden,
        Flagged,
        Revealed(u8),
    }

    let mut v_grid = vec![vec![VirtualState::Hidden; height]; width];

    let get_neighbors = |x: usize, y: usize| -> Vec<(usize, usize)> {
        let mut res = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    res.push((nx as usize, ny as usize));
                }
            }
        }
        res
    };

    let calc_neighbor_mines = |x: usize, y: usize, grid: &Vec<Vec<CellType>>| -> u8 {
        let mut count = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    if grid[nx as usize][ny as usize] == CellType::Mine {
                        count += 1;
                    }
                }
            }
        }
        count
    };

    let start_mines = calc_neighbor_mines(start_x, start_y, grid_setup);
    v_grid[start_x][start_y] = VirtualState::Revealed(start_mines);

    let mut reveal_queue = vec![(start_x, start_y)];
    while let Some((cx, cy)) = reveal_queue.pop() {
        let count = calc_neighbor_mines(cx, cy, grid_setup);
        v_grid[cx][cy] = VirtualState::Revealed(count);
        if count == 0 {
            for (nx, ny) in get_neighbors(cx, cy) {
                if v_grid[nx][ny] == VirtualState::Hidden && grid_setup[nx][ny] == CellType::Empty {
                    v_grid[nx][ny] = VirtualState::Revealed(calc_neighbor_mines(nx, ny, grid_setup));
                    reveal_queue.push((nx, ny));
                }
            }
        }
    }

    let mut progress = true;
    while progress {
        progress = false;

        for x in 0..width {
            for y in 0..height {
                if let VirtualState::Revealed(num) = v_grid[x][y] {
                    if num == 0 {
                        continue;
                    }

                    let neighbors = get_neighbors(x, y);
                    let mut hidden = Vec::new();
                    let mut flagged_count = 0;

                    for &(nx, ny) in &neighbors {
                        match v_grid[nx][ny] {
                            VirtualState::Hidden => hidden.push((nx, ny)),
                            VirtualState::Flagged => flagged_count += 1,
                            _ => {}
                        }
                    }

                    if hidden.is_empty() {
                        continue;
                    }

                    let remaining_mines = num as i32 - flagged_count;
                    if remaining_mines == hidden.len() as i32 {
                        for &(hx, hy) in &hidden {
                            v_grid[hx][hy] = VirtualState::Flagged;
                        }
                        progress = true;
                    }
                    else if remaining_mines == 0 {
                        let mut local_reveal = Vec::new();
                        for &(hx, hy) in &hidden {
                            v_grid[hx][hy] = VirtualState::Revealed(calc_neighbor_mines(hx, hy, grid_setup));
                            local_reveal.push((hx, hy));
                        }

                        while let Some((cx, cy)) = local_reveal.pop() {
                            if let VirtualState::Revealed(0) = v_grid[cx][cy] {
                                for (nx, ny) in get_neighbors(cx, cy) {
                                    if v_grid[nx][ny] == VirtualState::Hidden {
                                        v_grid[nx][ny] = VirtualState::Revealed(calc_neighbor_mines(nx, ny, grid_setup));
                                        local_reveal.push((nx, ny));
                                    }
                                }
                            }
                        }

                        progress = true;
                    }
                }
            }
        }
    }

    for x in 0..width {
        for y in 0..height {
            if grid_setup[x][y] == CellType::Empty {
                if let VirtualState::Revealed(_) = v_grid[x][y] {
                    // Ok
                } else {
                    return false;
                }
            }
        }
    }

    true
}

fn generate_mines_and_solve(
    click_x: usize,
    click_y: usize,
    board: &mut MinesweeperBoard,
) {
    let width = board.width;
    let height = board.height;
    let mine_count = board.mine_count;

    let mut retries = 0;
    let max_retries = 1000;

    loop {
        let mut temp_grid = vec![vec![CellType::Empty; height]; width];

        let mut mines_placed = 0;
        while mines_placed < mine_count {
            let x = (get_random_value() * width as f64) as usize;
            let y = (get_random_value() * height as f64) as usize;

            let is_in_safe_zone = (x as i32 - click_x as i32).abs() <= 1 
                               && (y as i32 - click_y as i32).abs() <= 1;

            if !is_in_safe_zone && temp_grid[x][y] == CellType::Empty {
                temp_grid[x][y] = CellType::Mine;
                mines_placed += 1;
            }
        }

        retries += 1;
        if is_solvable(width, height, click_x, click_y, &temp_grid) || retries >= max_retries {
            for x in 0..width {
                for y in 0..height {
                    board.grid[x][y].cell_type = temp_grid[x][y];
                }
            }
            break;
        }
    }

    let width = board.width;
    let height = board.height;
    for x in 0..width {
        for y in 0..height {
            if board.grid[x][y].cell_type == CellType::Mine {
                continue;
            }
            let mut count = 0;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        if board.grid[nx as usize][ny as usize].cell_type == CellType::Mine {
                            count += 1;
                        }
                    }
                }
            }
            board.grid[x][y].neighbor_mines = count;
        }
    }

    board.is_generated = true;
}

pub fn setup_minesweeper(
    mut commands: Commands,
    mut board: ResMut<MinesweeperBoard>,
    asset_server: Res<AssetServer>,
) {
    let bomb_handle = asset_server.load("bomb.png");
    commands.insert_resource(MinesweeperAssets {
        bomb: bomb_handle,
    });

    // Spawn 2D Camera
    commands.spawn((
        Camera2d::default(),
        MinesweeperEntity,
    ));

    let width = 10;
    let height = 10;
    let mine_count = 15;

    let grid = vec![vec![Cell {
        cell_type: CellType::Empty,
        neighbor_mines: 0,
        is_revealed: false,
        is_flagged: false,
        entity: None,
    }; height]; width];

    board.width = width;
    board.height = height;
    board.mine_count = mine_count;
    board.grid = grid;
    board.state = GameStatus::Playing;
    board.flags_placed = 0;
    board.is_generated = false;

    let cell_size = 40.0;
    let gap = 4.0;
    let step = cell_size + gap;
    let start_x = -((width as f32 * step - gap) / 2.0) + cell_size / 2.0;
    let start_y = -((height as f32 * step - gap) / 2.0) + cell_size / 2.0;

    for x in 0..width {
        for y in 0..height {
            let px = start_x + x as f32 * step;
            let py = start_y + y as f32 * step;
            let cell_entity = commands.spawn((
                Sprite {
                    color: Color::srgb(0.35, 0.35, 0.35),
                    custom_size: Some(Vec2::new(cell_size, cell_size)),
                    ..default()
                },
                Transform::from_xyz(px, py, 0.0),
                MinesweeperCell { x, y },
                MinesweeperEntity,
            )).id();
            board.grid[x][y].entity = Some(cell_entity);
        }
    }

    commands.spawn((
        Text2d::new("Mines: 15 | Flags: 0\nStatus: Playing\nLeft Click: Reveal | Right Click: Flag | R: Restart"),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, start_y - 45.0, 1.0),
        MinesweeperText,
        MinesweeperEntity,
    ));
}

fn update_cell_visual(
    x: usize,
    y: usize,
    board: &MinesweeperBoard,
    commands: &mut Commands,
    query: &mut Query<(&mut Sprite, &MinesweeperCell)>,
) {
    let cell = &board.grid[x][y];
    if let Some(entity) = cell.entity {
        if let Ok((mut sprite, _)) = query.get_mut(entity) {
            sprite.color = Color::srgb(0.75, 0.75, 0.75); // Light gray for revealed
        }

        if cell.neighbor_mines > 0 {
            let color = match cell.neighbor_mines {
                1 => Color::srgb(0.2, 0.4, 0.9),    // Blue
                2 => Color::srgb(0.1, 0.6, 0.1),    // Green
                3 => Color::srgb(0.9, 0.2, 0.2),    // Red
                4 => Color::srgb(0.1, 0.1, 0.6),    // Dark Blue
                5 => Color::srgb(0.6, 0.1, 0.1),    // Dark Red
                _ => Color::srgb(0.1, 0.5, 0.5),    // Teal
            };

            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    Text2d::new(cell.neighbor_mines.to_string()),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(color),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    MinesweeperEntity,
                ));
            });
        }
    }
}

fn toggle_flag(
    x: usize,
    y: usize,
    board: &mut MinesweeperBoard,
    query: &mut Query<(&mut Sprite, &MinesweeperCell)>,
) {
    let cell = &mut board.grid[x][y];
    if cell.is_revealed {
        return;
    }

    cell.is_flagged = !cell.is_flagged;
    if cell.is_flagged {
        board.flags_placed += 1;
    } else {
        board.flags_placed = board.flags_placed.saturating_sub(1);
    }

    if let Some(entity) = cell.entity {
        if let Ok((mut sprite, _)) = query.get_mut(entity) {
            if cell.is_flagged {
                sprite.color = Color::srgb(1.0, 0.6, 0.0); // Orange for flagged
            } else {
                sprite.color = Color::srgb(0.35, 0.35, 0.35); // Reset to covered dark gray
            }
        }
    }
}

fn reveal_all_mines(
    board: &MinesweeperBoard,
    query: &mut Query<(&mut Sprite, &MinesweeperCell)>,
    bomb_handle: &Handle<Image>,
) {
    for x in 0..board.width {
        for y in 0..board.height {
            let cell = &board.grid[x][y];
            if cell.cell_type == CellType::Mine {
                if let Some(entity) = cell.entity {
                    if let Ok((mut sprite, _)) = query.get_mut(entity) {
                        sprite.image = bomb_handle.clone();
                        sprite.color = Color::srgb(1.0, 1.0, 1.0); // Reset color to White so the bomb image shows in original colors!
                    }
                }
            }
        }
    }
}

fn reveal_cell(
    x: usize,
    y: usize,
    board: &mut MinesweeperBoard,
    commands: &mut Commands,
    query: &mut Query<(&mut Sprite, &MinesweeperCell)>,
    assets: &MinesweeperAssets,
) {
    if board.grid[x][y].is_revealed || board.grid[x][y].is_flagged {
        return;
    }

    board.grid[x][y].is_revealed = true;

    if board.grid[x][y].cell_type == CellType::Mine {
        board.state = GameStatus::Lost;
        reveal_all_mines(board, query, &assets.bomb);
        return;
    }

    update_cell_visual(x, y, board, commands, query);

    if board.grid[x][y].neighbor_mines == 0 {
        let mut to_reveal = vec![(x, y)];
        while let Some((cx, cy)) = to_reveal.pop() {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx >= 0 && nx < board.width as i32 && ny >= 0 && ny < board.height as i32 {
                        let n_x = nx as usize;
                        let n_y = ny as usize;
                        if !board.grid[n_x][n_y].is_revealed 
                            && !board.grid[n_x][n_y].is_flagged 
                            && board.grid[n_x][n_y].cell_type == CellType::Empty 
                        {
                            board.grid[n_x][n_y].is_revealed = true;
                            update_cell_visual(n_x, n_y, board, commands, query);
                            if board.grid[n_x][n_y].neighbor_mines == 0 {
                                to_reveal.push((n_x, n_y));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if won
    let mut unrevealed_safe_cells = 0;
    for rx in 0..board.width {
        for ry in 0..board.height {
            let rc = &board.grid[rx][ry];
            if !rc.is_revealed && rc.cell_type == CellType::Empty {
                unrevealed_safe_cells += 1;
            }
        }
    }

    if unrevealed_safe_cells == 0 {
        board.state = GameStatus::Won;
        for rx in 0..board.width {
            for ry in 0..board.height {
                let rc = &mut board.grid[rx][ry];
                if rc.cell_type == CellType::Mine && !rc.is_flagged {
                    rc.is_flagged = true;
                    if let Some(entity) = rc.entity {
                        if let Ok((mut sprite, _)) = query.get_mut(entity) {
                            sprite.color = Color::srgb(0.2, 0.6, 1.0); // Blue for flagged mines on win
                        }
                    }
                }
            }
        }
    }
}

fn update_hud(
    board: &MinesweeperBoard,
    text_query: &mut Query<&mut Text2d, With<MinesweeperText>>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        let status_str = match board.state {
            GameStatus::Playing => "Playing",
            GameStatus::Won => "YOU WON! Press R to play again",
            GameStatus::Lost => "GAME OVER! Press R to restart",
        };
        let remaining_mines = (board.mine_count as i32 - board.flags_placed as i32).max(0);
        text.0 = format!(
            "Mines Left: {} | Flags Placed: {}\nStatus: {}\nLeft Click: Reveal | Right Click: Flag | R: Restart",
            remaining_mines, board.flags_placed, status_str
        );
    }
}

pub fn minesweeper_click_system(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut board: ResMut<MinesweeperBoard>,
    mut commands: Commands,
    mut query: Query<(&mut Sprite, &MinesweeperCell)>,
    mut text_query: Query<&mut Text2d, With<MinesweeperText>>,
    assets: Res<MinesweeperAssets>,
) {
    if board.state != GameStatus::Playing {
        return;
    }

    let left_clicked = mouse_button_input.just_pressed(MouseButton::Left);
    let right_clicked = mouse_button_input.just_pressed(MouseButton::Right);

    if !left_clicked && !right_clicked {
        return;
    }

    // Since we only query for window, single() is safe
    let Ok(window) = window_query.single() else { return; };
    let Ok((camera, camera_transform)) = camera_query.single() else { return; };

    if let Some(cursor_position) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
            let cell_size = 40.0;
            let gap = 4.0;
            let step = cell_size + gap;
            let start_x = -((board.width as f32 * step - gap) / 2.0) + cell_size / 2.0;
            let start_y = -((board.height as f32 * step - gap) / 2.0) + cell_size / 2.0;

            let grid_x = ((world_pos.x - start_x + step / 2.0) / step).floor() as i32;
            let grid_y = ((world_pos.y - start_y + step / 2.0) / step).floor() as i32;

            if grid_x >= 0 && grid_x < board.width as i32 && grid_y >= 0 && grid_y < board.height as i32 {
                let gx = grid_x as usize;
                let gy = grid_y as usize;

                if left_clicked {
                    if !board.is_generated {
                        generate_mines_and_solve(gx, gy, &mut board);
                    }
                    reveal_cell(gx, gy, &mut board, &mut commands, &mut query, &assets);
                } else if right_clicked {
                    toggle_flag(gx, gy, &mut board, &mut query);
                }

                update_hud(&board, &mut text_query);
            }
        }
    }
}

pub fn minesweeper_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut board: ResMut<MinesweeperBoard>,
    mut commands: Commands,
    entities: Query<Entity, With<MinesweeperEntity>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        for entity in &entities {
            commands.entity(entity).despawn();
        }

        let width = 10;
        let height = 10;
        let grid = vec![vec![Cell {
            cell_type: CellType::Empty,
            neighbor_mines: 0,
            is_revealed: false,
            is_flagged: false,
            entity: None,
        }; height]; width];

        board.grid = grid;
        board.state = GameStatus::Playing;
        board.flags_placed = 0;
        board.is_generated = false;

        commands.spawn((
            Camera2d::default(),
            MinesweeperEntity,
        ));

        let cell_size = 40.0;
        let gap = 4.0;
        let step = cell_size + gap;
        let start_x = -((width as f32 * step - gap) / 2.0) + cell_size / 2.0;
        let start_y = -((height as f32 * step - gap) / 2.0) + cell_size / 2.0;

        for x in 0..width {
            for y in 0..height {
                let px = start_x + x as f32 * step;
                let py = start_y + y as f32 * step;
                let cell_entity = commands.spawn((
                    Sprite {
                        color: Color::srgb(0.35, 0.35, 0.35),
                        custom_size: Some(Vec2::new(cell_size, cell_size)),
                        ..default()
                    },
                    Transform::from_xyz(px, py, 0.0),
                    MinesweeperCell { x, y },
                    MinesweeperEntity,
                )).id();
                board.grid[x][y].entity = Some(cell_entity);
            }
        }

        commands.spawn((
            Text2d::new("Mines: 15 | Flags: 0\nStatus: Playing\nLeft Click: Reveal | Right Click: Flag | R: Restart"),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(0.0, start_y - 45.0, 1.0),
            MinesweeperText,
            MinesweeperEntity,
        ));
    }
}

pub fn cleanup_minesweeper(
    mut commands: Commands,
    query: Query<Entity, With<MinesweeperEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<MinesweeperAssets>();
}

pub struct MinesweeperPlugin;

impl Plugin for MinesweeperPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MinesweeperBoard {
            width: 0,
            height: 0,
            mine_count: 0,
            grid: Vec::new(),
            state: GameStatus::Playing,
            flags_placed: 0,
            is_generated: false,
        })
        .add_systems(
            OnEnter(crate::GameMode::Minesweeper),
            setup_minesweeper,
        )
        .add_systems(
            OnExit(crate::GameMode::Minesweeper),
            cleanup_minesweeper,
        )
        .add_systems(
            Update,
            (
                minesweeper_click_system,
                minesweeper_input_system,
            )
                .run_if(in_state(crate::GameMode::Minesweeper)),
        );
    }
}
