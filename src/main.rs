use bevy::prelude::*;
use rand::Rng;
use std::time::{Instant, Duration};

const PLAYER_LIFE: f32 = 1.0;
const ROCK_LIFE: f32 = 1.0;
const SHOT_TTL: Duration = Duration::from_secs(2);

const PLAYER_BBOX: f32 = 12.0;
const ROCK_BBOX: f32 = 12.0;
const SHOT_BBOX: f32 = 6.0;

const MAX_ROCK_VEL: f32 = 50.0;

/// How fast a rock rotates
const ROCK_ANG_VEL: f32 = 0.5;

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

struct Player {
    last_shot_time: Instant
}
struct Rock;
struct Shot {
    ttl: Duration,
}

struct Box {
    velocity: Vec2,
    bbox_size: f32,
}

struct Spinner {
    ang_vel: f32
}

#[derive(Default)]
struct PreLoadedAssets
{
    shot_mat: Handle<ColorMaterial>,
    rock_mat: Handle<ColorMaterial>,

    shot_sound: Handle<AudioSource>,
    hit_sound: Handle<AudioSource>,
}

fn rand_orientation() -> Quat
{
    Quat::from_rotation_z(rand::thread_rng().gen_range(0.0_f32 .. (2.0_f32 * std::f32::consts::PI)))
}

fn test_hit(pa: Vec2, ra: f32, pb: Vec2, rb: f32) -> bool
{
    pa.distance_squared(pb) < (ra + rb).powi(2)
}

fn setup(
    windows: Res<Windows>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut pre_loaded_assets: ResMut<PreLoadedAssets>)
{
    // Load all assets
    pre_loaded_assets.shot_mat = materials.add(asset_server.load("shot.png").into());
    pre_loaded_assets.rock_mat = materials.add(asset_server.load("rock.png").into());

    pre_loaded_assets.shot_sound = asset_server.load("pew.ogg");
    pre_loaded_assets.hit_sound = asset_server.load("boom.ogg");

    let player_mat = materials.add(asset_server.load("player.png").into());

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn()
        .insert(Player{
            last_shot_time: Instant::now() - 2*PLAYER_SHOT_TIME
        })
        .insert(Box{
            velocity: Vec2::ZERO,
            bbox_size: PLAYER_BBOX,
        })
        .insert_bundle(
            SpriteBundle {
                material: player_mat,
                ..Default::default()
            }
        );

    let mut rng = rand::thread_rng();

    if let Some(w) = windows.get_primary() {
        for _ in 0..5 {
            let velocity = Vec2::from(rand_orientation().mul_vec3(
                Vec3::new(rng.gen_range(0.0..MAX_ROCK_VEL), 0.0, 0.0)
            ));

            let mut pos;
            while {
                pos = Vec2::new(
                    rng.gen_range(0.0..w.width()),
                    rng.gen_range(0.0..w.height())
                );
                test_hit(pos, ROCK_BBOX, Vec2::ZERO, PLAYER_BBOX)
            } {};

            let translation = Vec3::from((pos, 0.0));

            commands.spawn()
                .insert(Rock)
                .insert(Box{
                    velocity,
                    bbox_size: ROCK_BBOX
                })
                .insert_bundle(
                    SpriteBundle {
                        material: pre_loaded_assets.rock_mat.clone_weak(),
                        transform: Transform{translation,..Default::default()},
                        ..Default::default()
                    }
                );
        }
    } else {
        panic!("No window found!");
    }
}

fn control(mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>, audio: Res<Audio>,
    pre_loaded_assets: Res<PreLoadedAssets>,
    mut query: Query<(&mut Player, &mut Transform, &mut Box)>)
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

    match query.single_mut() {
        Ok((mut player, mut t, mut bx)) => {
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

                        commands.spawn()
                            .insert(Shot{
                                ttl: SHOT_TTL
                            })
                            .insert(Box{
                                bbox_size: SHOT_BBOX,
                                velocity
                            })
                            .insert(Spinner{
                                ang_vel: SHOT_ANG_VEL
                            })
                            .insert_bundle(
                                SpriteBundle {
                                    material: pre_loaded_assets.shot_mat.clone_weak(),
                                    transform: *t,
                                    ..Default::default()
                                }
                            );
                        audio.play(pre_loaded_assets.shot_sound.clone_weak());
                    }
                }
            }
        }
        _ => panic!("Player must always exist!")
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

fn update_box_position(windows: Res<Windows>, time: Res<Time>, mut query: Query<(&mut Transform, &mut Box)>)
{
    if let Some(window) = windows.get_primary() {
        let dt = time.delta_seconds();

        for (mut t, bx) in query.iter_mut() {
            // Translate it:
            let dv = dt * bx.velocity;
            t.translation += Vec3::from((dv, 0.0));

            wrap_actor_position(&mut *t, window.width(), window.height());
        }
    } else {
        panic!("No window found!");
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
        .run();
}
