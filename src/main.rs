use std::f32::consts::PI;

use bevy::{input::keyboard, prelude::*, scene::ron::de, sprite::Mesh2dHandle, window::EnabledButtons};

const WINDOW_SIZE: (f32, f32) = (512f32, 512f32);
const PADDLE_SHAPE: Rectangle = Rectangle {
    half_size: Vec2 { x: 4f32, y: 32f32 }
};

const BALL_SHAPE: Rectangle = Rectangle {
    half_size: Vec2 { x: 4f32, y: 4f32 }
};
const BALL_SPEED: f32 = 256f32;

const PADDLE_SPEED: f32 = 128f32;

const COLLISION_MAX_ANGLE: f32 = PI/4f32;

const TEXT_OFFSET_X: f32 = 32f32;

const NEXT_ROUND_INTERVAL: f32 = 1f32;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Serving,
    Started,
    RoundOver,
    GameOver,
}

#[derive(Resource, Default)]
struct Score {
    player: i32,
    enemy: i32,
}

#[derive(Resource)]
struct NextRoundTimer(Timer);

impl Default for NextRoundTimer {
    fn default() -> Self {
        NextRoundTimer(Timer::from_seconds(NEXT_ROUND_INTERVAL, TimerMode::Once))
    }
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct Paddle {
    dir: i32,
}

impl Default for Paddle {
    fn default() -> Self {
        Paddle {
            dir: 0,
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Ball {
    vel: Vec2,
}

impl Default for Ball {
    fn default() -> Self {
        Ball {
            vel: Vec2::default()
        }
    }
}

fn clamp<T>(v: T, min: T, max: T) -> T
    where T: PartialOrd
{
    if v < min { min } else if v > max { max } else { v }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "KPong".into(),
                        resizable: false,
                        enabled_buttons: EnabledButtons {
                            maximize: false,
                            ..default()
                        },
                        present_mode: bevy::window::PresentMode::AutoNoVsync,
                        resolution: WINDOW_SIZE.into(),
                        ..default()
                    }),
                    ..default()
                })
        )
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (
                player_input,
                move_paddle,

                pre_serve.run_if(in_state(GameState::Serving)),
                enemy_ai.run_if(in_state(GameState::Started)),
                move_ball.run_if(in_state(GameState::Started)),
                round_over.run_if(in_state(GameState::RoundOver)),

                update_ui,
            )
        )
        .add_systems(
            OnEnter(GameState::Started),
            on_round_started
        )
        .add_systems(
            OnEnter(GameState::RoundOver),
            on_round_over
        )
        .add_systems(
            OnEnter(GameState::Serving),
            on_start_serving
        )
        .init_state::<GameState>()
        .init_resource::<Score>()
        .init_resource::<NextRoundTimer>()
        .run();
}

fn startup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
){
    let paddle_mesh = Mesh2dHandle(meshes.add(PADDLE_SHAPE));
    let paddle_mat = materials.add(Color::WHITE);

    cmd.spawn(Camera2dBundle::default());

    cmd.spawn((
        ColorMesh2dBundle {
            mesh: paddle_mesh.clone(),
            material: paddle_mat.clone(),
            transform: Transform::from_xyz(
                -WINDOW_SIZE.0/2f32 + PADDLE_SHAPE.half_size.x,
                0f32,
                0f32
            ),
            ..default()
        },
        Paddle::default(),
        Player
    ));

    cmd.spawn((
        ColorMesh2dBundle {
            mesh: paddle_mesh.clone(),
            material: paddle_mat.clone(),
            transform: Transform::from_xyz(
                WINDOW_SIZE.0/2f32 - PADDLE_SHAPE.half_size.x,
                0f32,
                0f32
            ),
            ..default()
        },
        Paddle::default(),
        Enemy{},
    ));

    cmd.spawn((
        ColorMesh2dBundle {
            mesh: Mesh2dHandle(meshes.add(BALL_SHAPE)),
            material: paddle_mat.clone(),
            transform: Transform::default(),
            ..default()
        },
        Ball::default(),
    ));

    const FONT_SIZE: f32 = 32f32;
    let text_style = TextStyle {
        font_size: FONT_SIZE,
        ..default()
    };
    cmd.spawn((
        Text2dBundle {
            text: Text::from_section("0", text_style.clone()),
            transform: Transform::from_xyz(-TEXT_OFFSET_X, WINDOW_SIZE.1/2f32 - FONT_SIZE, 0f32),
            ..default()
        },
        ScoreText,
        Enemy,
    ));
    cmd.spawn((
        Text2dBundle {
            text: Text::from_section("0", text_style.clone()),
            transform: Transform::from_xyz(TEXT_OFFSET_X, WINDOW_SIZE.1/2f32 - FONT_SIZE, 0f32),
            ..default()
        },
        ScoreText,
        Player,
    ));
}

fn pre_serve(
    keyboard_input_res: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input_res.pressed(KeyCode::KeyW) || keyboard_input_res.pressed(KeyCode::KeyS) {
        next_state.set(GameState::Started);
    }
}

fn on_round_started(
    mut balls: Query<&mut Ball>
){
    for mut ball in balls.iter_mut() {
        ball.vel = Vec2::new(-BALL_SPEED, 0f32);
    }
}

fn player_input(
    keyboard_input_res: Res<ButtonInput<KeyCode>>,
    mut paddle: Query<&mut Paddle, With<Player>>
) {
    let keyboard_input: &ButtonInput<KeyCode> = &keyboard_input_res;
    let move_dir = if keyboard_input.pressed(KeyCode::KeyS) { -1 }
        else if keyboard_input.pressed(KeyCode::KeyW) { 1 }
        else { 0 };

    for mut paddle in paddle.iter_mut() {
        paddle.dir = move_dir;
    }
}

fn enemy_ai(
    mut paddles: Query<(&mut Paddle, &Transform), With<Enemy>>,
    balls: Query<&Transform, With<Ball>>
) {
    match balls.get_single() {
        Ok(ball_trans) => {
            for (mut paddle, paddle_trans) in paddles.iter_mut() {
                // println!("{}", (ball_trans.translation.y - paddle_trans.translation.y).signum());
                paddle.dir = (ball_trans.translation.y - paddle_trans.translation.y).signum() as i32;
            }
        },
        _ => {}
    }
}

fn move_paddle(
    mut paddle: Query<(&Paddle, &mut Transform)>,
    time: Res<Time>,
) {
    for (paddle, mut transform) in paddle.iter_mut() {
        transform.translation.y += PADDLE_SPEED * paddle.dir as f32 * time.delta_seconds();
        transform.translation.y = clamp(
            transform.translation.y,
            -WINDOW_SIZE.1/2f32 + PADDLE_SHAPE.half_size.y,
            WINDOW_SIZE.1/2f32 - PADDLE_SHAPE.half_size.y,
        );
    }
}

fn move_ball(
    time: Res<Time>,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<GameState>>,
    mut balls: Query<(&mut Ball, &mut Transform), Without<Paddle>>,
    paddles: Query<&Transform, With<Paddle>>,
) {
    const MAX_BALL_Y: f32 = WINDOW_SIZE.1/2f32 - BALL_SHAPE.half_size.y;
    for (mut ball, mut transform) in balls.iter_mut() {
        let prev_x = transform.translation.x;
        transform.translation += Vec3::from((ball.vel * time.delta_seconds(), 0f32));
        if transform.translation.y > MAX_BALL_Y || transform.translation.y < -MAX_BALL_Y {
            ball.vel.y *= -1f32;
            transform.translation.y = clamp(transform.translation.y, -MAX_BALL_Y, MAX_BALL_Y);
        }
        let pos = transform.translation;
        for paddle_trans in paddles.iter() {
            let center = paddle_trans.translation;
            let top_wall_y = center.y + PADDLE_SHAPE.half_size.y + BALL_SHAPE.half_size.x;
            let bottom_wall_y = center.y - PADDLE_SHAPE.half_size.y - BALL_SHAPE.half_size.x;
            let left_wall_x = center.x - PADDLE_SHAPE.half_size.x - BALL_SHAPE.half_size.x;
            let right_wall_x = center.x + PADDLE_SHAPE.half_size.x + BALL_SHAPE.half_size.x;

            if pos.y > top_wall_y || pos.y < bottom_wall_y {
                continue;
            }

            let right_collision = prev_x > right_wall_x && pos.x < right_wall_x;
            let left_collision = prev_x < left_wall_x && pos.x > left_wall_x;

            if right_collision || left_collision {
                let percent_vertical = (pos.y - center.y)/PADDLE_SHAPE.half_size.y;
                ball.vel.x *= -1f32;
                ball.vel = Vec2::from_angle(COLLISION_MAX_ANGLE * percent_vertical).rotate(ball.vel);
            }
        }

        if pos.x - PADDLE_SHAPE.half_size.x <= -WINDOW_SIZE.1/2f32 {
            score.player += 1;
            next_state.set(GameState::RoundOver);
        }
        else if pos.x + PADDLE_SHAPE.half_size.x >= WINDOW_SIZE.1/2f32 {
            score.enemy += 1;
            next_state.set(GameState::RoundOver);
        }
    }
}

fn update_ui(
    score: Res<Score>,
    mut player_score: Query<&mut Text, (With<Player>, Without<Enemy>)>,
    mut enemy_score: Query<&mut Text, (With<Enemy>, Without<Player>)>,
){
    player_score.single_mut().sections[0].value = score.player.to_string();
    enemy_score.single_mut().sections[0].value = score.enemy.to_string();
}

fn on_start_serving(
    mut paddles: Query<(&mut Paddle, &mut Transform), Without<Ball>>,
    mut balls: Query<(&mut Ball, &mut Transform), Without<Paddle>>,
){
    let (mut ball, mut ball_trans) = balls.single_mut();
    ball.vel = Vec2::default();
    ball_trans.translation = Vec3::default();

    for (mut paddle, mut trans) in paddles.iter_mut() {
        paddle.dir = 0;
        trans.translation.y = 0f32;
    }
}

fn on_round_over(
    mut paddles: Query<&mut Paddle, With<Enemy>>,
    mut timer: ResMut<NextRoundTimer>,
){
    for mut paddle in paddles.iter_mut() {
        paddle.dir = 0;
    }

    timer.0.reset();
}

fn round_over(
    time: Res<Time>,
    mut timer: ResMut<NextRoundTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        next_state.set(GameState::Serving);
    }
}