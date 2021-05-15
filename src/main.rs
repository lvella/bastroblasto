use bevy::prelude::*;
use bevy::app::AppExit;
use rand::Rng;
use std::time::{Instant, Duration};

// Game constants:

const SHOT_TTL: Duration = Duration::from_secs(2);

const PLAYER_BBOX: f32 = 12.0;
const ROCK_BBOX: f32 = 12.0;
const SHOT_BBOX: f32 = 6.0;

const MAX_ROCK_VEL: f32 = 50.0;

/// How fast shots move.
const SHOT_SPEED: f32 = 200.0;
/// Angular velocity of how fast shots rotate.
const SHOT_ANG_VEL: f32 = 0.1;

/// Acceleration in pixels per second.
const PLAYER_THRUST: f32 = 100.0;
/// Rotation in radians per second.
const PLAYER_TURN_RATE: f32 = 3.0;
/// Refire delay between shots.
const PLAYER_SHOT_TIME: Duration = Duration::from_millis(500);

// Components:

struct Player {
    last_shot_time: Instant
}

struct Rock;

struct Shot {
    ttl: Duration,
}

struct BBox {
    velocity: Vec2,
    bbox_size: f32,
}

struct Spinner {
    ang_vel: f32
}

#[derive(Default)]
struct Level {
    level: u16,
    rock_kill_count: u16
}

#[derive(Default)]
struct Score {
    value: u32
}

// Entity bundles:

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    bbox: BBox,
    #[bundle]
    sprite: SpriteBundle,
}

#[derive(Bundle)]
struct RockBundle {
    rock: Rock,
    bbox: BBox,
    #[bundle]
    sprite: SpriteBundle,
}

#[derive(Bundle)]
struct ShotBundle {
    shot: Shot,
    bbox: BBox,
    spinner: Spinner,
    #[bundle]
    sprite: SpriteBundle,
}

#[derive(Bundle)]
struct LevelBundle {
    level: Level,
    #[bundle]
    text2d: Text2dBundle
}

#[derive(Bundle)]
struct ScoreBundle {
    score: Score,
    #[bundle]
    text2d: Text2dBundle
}

// Global resources:

#[derive(Default)]
struct PreLoadedAssets
{
    shot_mat: Handle<ColorMaterial>,
    rock_mat: Handle<ColorMaterial>,

    shot_sound: Handle<AudioSource>,
    hit_sound: Handle<AudioSource>,
}

impl Level {
    fn total_rock_count(&self) -> u16
    {
        self.level + 4
    }
}

// Free helper functions:

fn rand_orientation() -> Quat
{
    Quat::from_rotation_z(rand::thread_rng().gen_range(0.0_f32 .. (2.0_f32 * std::f32::consts::PI)))
}

fn test_hit(pa: Vec2, ra: f32, pb: Vec2, rb: f32) -> bool
{
    pa.distance_squared(pb) < (ra + rb).powi(2)
}

fn next_level(
    w: &Window,
    pre_loaded_assets: &PreLoadedAssets,
    commands: &mut Commands,
    level: &mut Level,
    level_text: &mut Text,
    exclusion: Vec2
) {
    let mut rng = rand::thread_rng();

    level.rock_kill_count = 0;
    level.level += 1;

    level_text.sections[0].value = format!("Level: {}", level.level);

    for _ in 0..level.total_rock_count() {
        let velocity = Vec2::from(rand_orientation().mul_vec3(
            Vec3::new(rng.gen_range(0.0..MAX_ROCK_VEL), 0.0, 0.0)
        ));

        let mut pos;
        while {
            pos = Vec2::new(
                rng.gen_range(0.0..w.width()),
                rng.gen_range(0.0..w.height())
            );
            test_hit(pos, ROCK_BBOX, exclusion, PLAYER_BBOX*3.0)
        } {};

        let translation = Vec3::from((pos, 0.0));

        (*commands).spawn_bundle(RockBundle{
            rock: Rock,
            bbox: BBox{
                velocity,
                bbox_size: ROCK_BBOX
            },
            sprite: SpriteBundle {
                material: pre_loaded_assets.rock_mat.clone(),
                transform: Transform{translation,..Default::default()},
                ..Default::default()
            },
        });
    }
}

fn write_score(score: &Score) -> String
{
    format!("Score: {}", score.value)
}

// Systems:

fn setup(
    mut commands: Commands,
    windows: Res<Windows>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut pre_loaded_assets: ResMut<PreLoadedAssets>)
{
    // Load all assets
    pre_loaded_assets.shot_mat = materials.add(asset_server.load("shot.png").into());
    pre_loaded_assets.rock_mat = materials.add(asset_server.load("rock.png").into());

    pre_loaded_assets.shot_sound = asset_server.load("pew.ogg");
    pre_loaded_assets.hit_sound = asset_server.load("boom.ogg");

    let font = asset_server.load("LiberationMono-Regular.ttf");

    // Create camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    // Create Level
    let w = windows.get_primary().expect("Window must exist!");
    let to_top_left = Vec3::new(-w.width() * 0.5, w.height() * 0.5, 0.0);

    let mut level: Level = Default::default();
    let mut level_text = Text::with_section(
        "",
        TextStyle {
            font: font.clone(),
            font_size: 32.0,
            color: Color::WHITE,
        },
        TextAlignment {
            vertical: VerticalAlign::Bottom,
            horizontal: HorizontalAlign::Right,
        },
    );

    next_level(w, &pre_loaded_assets, &mut commands, &mut level, &mut level_text, Vec2::ZERO);

    commands.spawn_bundle(LevelBundle{
        level,
        text2d: Text2dBundle{
            text: level_text,
            transform: Transform {
                translation: Vec3::new(10.0, -10.0, 0.0) + to_top_left,
                ..Default::default()
            },
            ..Default::default()
        }
    });

    // Create score
    let score = Score{ value: 0 };
    let score_text = write_score(&score);
    commands.spawn_bundle(ScoreBundle{
        score,
        text2d: Text2dBundle{
            text: Text::with_section(
                score_text,
                TextStyle {
                    font,
                    font_size: 32.0,
                    color: Color::WHITE,
                },
                TextAlignment {
                    vertical: VerticalAlign::Bottom,
                    horizontal: HorizontalAlign::Right,
                },
            ),
            transform: Transform {
                translation: Vec3::new(200.0, -10.0, 0.0) + to_top_left,
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let player_mat = materials.add(asset_server.load("player.png").into());
    commands.spawn_bundle(PlayerBundle{
        player: Player{
            last_shot_time: Instant::now() - 2*PLAYER_SHOT_TIME
        },
        bbox: BBox{
            velocity: Vec2::ZERO,
            bbox_size: PLAYER_BBOX,
        },
        sprite: SpriteBundle {
            material: player_mat,
            ..Default::default()
        }
    });
}

fn control(mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>, audio: Res<Audio>,
    pre_loaded_assets: Res<PreLoadedAssets>,
    mut query: Query<(&mut Player, &mut Transform, &mut BBox)>)
{
    let mut direction = 0.0;
    if keyboard_input.pressed(KeyCode::Left) {
        direction += 1.0;
    }
    if keyboard_input.pressed(KeyCode::Right) {
        direction -= 1.0;
    }

    let thrust = keyboard_input.pressed(KeyCode::Up);

    let shot = keyboard_input.pressed(KeyCode::Space);

    let dt = time.delta_seconds();

    let (mut player, mut t, mut bx) = query.single_mut().expect("Player must exist!");

    // First rotate the Player:
    t.rotate(Quat::from_rotation_z(dt * PLAYER_TURN_RATE * direction));

    // Then accelerate player in thrust direction:
    let forward_dir = Vec2::from(t.rotation.mul_vec3(Vec3::Y));
    if thrust {
        let thrust_delta = dt * PLAYER_THRUST * forward_dir;
        bx.velocity += thrust_delta;

        // Clamp the velocity to the max efficiently
        let norm_sq = bx.velocity.length_squared();
        if norm_sq > MAX_PHYSICS_VEL.powi(2) {
            bx.velocity = bx.velocity / norm_sq.sqrt() * MAX_PHYSICS_VEL;
        }
    }

    // If possible, shot
    if shot {
        if let Some(now) = time.last_update() {
            if now.saturating_duration_since(player.last_shot_time) > PLAYER_SHOT_TIME {
                player.last_shot_time = now;

                let velocity = SHOT_SPEED * forward_dir + bx.velocity;

                commands.spawn_bundle(ShotBundle {
                    shot: Shot{
                        ttl: SHOT_TTL
                    },
                    bbox: BBox{
                        bbox_size: SHOT_BBOX,
                        velocity
                    },
                    spinner: Spinner{
                        ang_vel: SHOT_ANG_VEL
                    },
                    sprite: SpriteBundle {
                        material: pre_loaded_assets.shot_mat.clone(),
                        transform: *t,
                        ..Default::default()
                    }
                });
                audio.play(pre_loaded_assets.shot_sound.clone());
            }
        }
    }
}

const MAX_PHYSICS_VEL: f32 = 250.0;

/// Takes an actor and wraps its position to the bounds of the
/// screen, so if it goes off the left side of the screen it
/// will re-enter on the right side and so on.
fn wrap_actor_position(t: &mut Transform, sx: f32, sy: f32) {
    // Wrap screen
    let screen_x_bounds = sx / 2.0;
    let screen_y_bounds = sy / 2.0;
    if t.translation.x > screen_x_bounds {
        t.translation.x -= sx;
    } else if t.translation.x < -screen_x_bounds {
        t.translation.x += sx;
    };
    if t.translation.y > screen_y_bounds {
        t.translation.y -= sy;
    } else if t.translation.y < -screen_y_bounds {
        t.translation.y += sy;
    };
}

fn update_box_position(windows: Res<Windows>, time: Res<Time>, mut query: Query<(&mut Transform, &mut BBox)>)
{
    let window = windows.get_primary().expect("Window must exist!");
    let dt = time.delta_seconds();

    for (mut t, bx) in query.iter_mut() {
        // Translate it:
        let dv = dt * bx.velocity;
        t.translation += Vec3::from((dv, 0.0));

        wrap_actor_position(&mut *t, window.width(), window.height());
    }
}

fn update_spinner_spin(time: Res<Time>, mut query: Query<(&mut Transform, &mut Spinner)>)
{
    let dt = time.delta_seconds();

    for (mut t, sp) in query.iter_mut() {
        t.rotate(Quat::from_rotation_z(dt * sp.ang_vel));
    }
}

fn update_shot_ttl(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &mut Shot)>)
{
    let dt = time.delta();

    for (entity, mut shot) in query.iter_mut() {
        if let Some(new_ttl) = shot.ttl.checked_sub(dt) {
            shot.ttl = new_ttl;
        } else {
            commands.entity(entity).despawn();
        }
    }
}

fn player_rock_collision(
    mut exit: EventWriter<AppExit>,
    player_query: Query<(&Transform, &BBox), With<Player>>,
    rock_query: Query<(&Transform, &BBox), With<Rock>>)
{
    let (pt, pbox) = player_query.single().expect("Player must exist!");

    for (rt, rbox) in rock_query.iter() {
        if test_hit(Vec2::from(pt.translation), pbox.bbox_size,
                    Vec2::from(rt.translation), rbox.bbox_size) {
            exit.send(AppExit);
        }
    }
}

fn rock_shot_collision(
    mut commands: Commands,
    windows: Res<Windows>,
    audio: Res<Audio>,
    pre_loaded_assets: Res<PreLoadedAssets>,
    mut text_elems: QuerySet<(
        Query<(&mut Level, &mut Text)>,
        Query<(&mut Score, &mut Text)>,
    )>,
    player_query: Query<&Transform, With<Player>>,
    rock_query: Query<(Entity, &Transform, &BBox), With<Rock>>,
    shot_query: Query<(Entity, &Transform, &BBox), With<Shot>>)
{
    for (re, rt, rbox) in shot_query.iter() {
        for (se, st, sbox) in rock_query.iter() {
            if test_hit(Vec2::from(st.translation), sbox.bbox_size,
                        Vec2::from(rt.translation), rbox.bbox_size) {
                commands.entity(se).despawn();
                commands.entity(re).despawn();
                audio.play(pre_loaded_assets.hit_sound.clone());

                // Update level status
                {
                    let (mut level, _) = text_elems.q0_mut().single_mut().expect("Level must exist!");
                    level.rock_kill_count += 1;
                }

                // Update score:
                {
                    let (mut score, mut score_text) =
                        text_elems.q1_mut().single_mut().expect("Score must exist!");
                    score.value += 1;
                    score_text.sections[0].value = write_score(&score);
                }
            }
        }
    }

    let (mut level, mut level_text) = text_elems.q0_mut().single_mut().expect("Level must exist!");
    if level.rock_kill_count == level.total_rock_count() {
        // Next level:
        let window = windows.get_primary().expect("Window must exist!");
        let player = player_query.single().expect("Player must exist!");
        next_level(window, &pre_loaded_assets, &mut commands,
            &mut level, &mut level_text, Vec2::from(player.translation));
    }
}

fn main()
{
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Bastroblasto!".to_string(),
            width: 800.,
            height: 600.,
            vsync: true,
            ..Default::default()
        })
        .insert_resource(PreLoadedAssets{..Default::default()})
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(control.system())
        .add_system(update_box_position.system())
        .add_system(update_spinner_spin.system())
        .add_system(update_shot_ttl.system())
        .add_system(player_rock_collision.system())
        .add_system(rock_shot_collision.system())
        .run();
}
