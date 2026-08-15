use avian3d::prelude::Collider;
use bevy::prelude::*;

pub const DEFAULT_WALK_SPEED: f32 = 16.0 * 0.28;
pub const GROUND_ACCEL: f32 = 60.0;
pub const GROUND_DECEL: f32 = 60.0;
pub const AIR_ACCEL: f32 = 30.0;
pub const AIR_DECEL: f32 = 45.0;
pub const HARD_SPEED_LIMIT: f32 = 40.0;
pub const MAX_FALL_SPEED: f32 = 14.0;
pub const TURN_SPEED: f32 = 25.0;
pub const JUMP_BUFFER_SECS: f32 = 0.15;
pub const COYOTE_TIME_SECS: f32 = 0.1;

pub const PLAYER_HALF_WIDTH: f32 = 1.0 * 0.28;
pub const PLAYER_HALF_HEIGHT: f32 = 2.5 * 0.28;
pub const PLAYER_HALF_DEPTH: f32 = 0.585 * 0.28;

pub const MAX_STEP_HEIGHT: f32 = 0.29;
pub const MIN_STEP_HEIGHT: f32 = 0.05;
pub const GROUND_SNAP_DIST: f32 = 0.15;
pub const MAX_SLOPE_COS: f32 = 0.7071;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementProbe {
    Ground,
    ForwardLow,
    ForwardHigh,
    StepDown,
}

pub fn probe_shape(probe: MovementProbe) -> Collider {
    let width = PLAYER_HALF_WIDTH * 1.8;
    let depth = PLAYER_HALF_DEPTH * 1.8;
    match probe {
        MovementProbe::Ground => Collider::cuboid(width, 0.04, depth),
        MovementProbe::ForwardLow | MovementProbe::ForwardHigh | MovementProbe::StepDown => {
            Collider::cuboid(width, 0.16, 0.02)
        }
    }
}

fn probe_half_height(probe: MovementProbe) -> f32 {
    match probe {
        MovementProbe::Ground => 0.02,
        MovementProbe::ForwardLow | MovementProbe::ForwardHigh | MovementProbe::StepDown => 0.08,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CastHit {
    pub distance: f32,
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Resource)]
pub struct CharacterMoveState {
    pub horizontal_velocity: Vec2,
    pub jump_buffered_at: Option<f32>,
    pub coyote_until: Option<f32>,
}

impl Default for CharacterMoveState {
    fn default() -> Self {
        Self {
            horizontal_velocity: Vec2::ZERO,
            jump_buffered_at: None,
            coyote_until: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterMoveParams {
    pub wish_direction: Vec2,
    pub jump_held: bool,
    pub speed: f32,
    pub jump_power: f32,
    pub dt: f32,
    pub elapsed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterMoveOutput {
    pub velocity: Vec3,
    pub position_y: Option<f32>,
    pub position_xz: Option<Vec2>,
    pub grounded: bool,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerMovementPlan {
    pub velocity: Vec3,
    pub position_y: Option<f32>,
    pub position_xz: Option<Vec2>,
    pub rotation: Option<Quat>,
}

pub fn character_move(
    center: Vec3,
    current_velocity: Vec3,
    yaw: f32,
    state: &mut CharacterMoveState,
    params: &CharacterMoveParams,
    mut cast: impl FnMut(MovementProbe, Vec3, Quat, Vec3, f32) -> Option<CastHit>,
) -> CharacterMoveOutput {
    let feet = center.y - PLAYER_HALF_HEIGHT;
    let rotation = Quat::from_rotation_y(yaw);
    let has_wish = params.wish_direction.length_squared() > 0.001;
    let dt = params.dt.max(0.0);

    let ground_hit = cast(
        MovementProbe::Ground,
        Vec3::new(center.x, feet + 0.1, center.z),
        rotation,
        Vec3::NEG_Y,
        0.1 + GROUND_SNAP_DIST,
    );
    let grounded = ground_hit.is_some_and(|hit| hit.normal.y >= MAX_SLOPE_COS);

    let target = if has_wish {
        params.wish_direction * params.speed
    } else {
        Vec2::ZERO
    };
    let rate = if grounded {
        if has_wish {
            GROUND_ACCEL
        } else {
            GROUND_DECEL
        }
    } else if has_wish {
        AIR_ACCEL
    } else {
        AIR_DECEL
    };
    state.horizontal_velocity = state.horizontal_velocity.move_towards(target, rate * dt);
    if state.horizontal_velocity.length() > HARD_SPEED_LIMIT {
        state.horizontal_velocity =
            state.horizontal_velocity.normalize() * HARD_SPEED_LIMIT;
    }

    let mut velocity = current_velocity;
    velocity.x = state.horizontal_velocity.x;
    velocity.z = state.horizontal_velocity.y;

    let mut position_y = None;
    let mut position_xz = None;

    if grounded {
        if let Some(hit) = ground_hit {
            if has_wish {
                let wish3 = Vec3::new(params.wish_direction.x, 0.0, params.wish_direction.y);
                let tangent = (wish3 - hit.normal * wish3.dot(hit.normal)).normalize_or_zero();
                velocity = tangent * params.speed;
            } else {
                velocity.y = 0.0;
            }

            let floor_top = feet + 0.1 - hit.distance - probe_half_height(MovementProbe::Ground);
            if floor_top - feet <= GROUND_SNAP_DIST && feet - floor_top > 0.001 {
                position_y = Some(floor_top + PLAYER_HALF_HEIGHT);
            }
        }

        if has_wish {
            let dir = Vec3::new(params.wish_direction.x, 0.0, params.wish_direction.y)
                .normalize_or_zero();
            if dir != Vec3::ZERO {
                let reach = PLAYER_HALF_DEPTH + 0.1;
                let forward_hit = cast(
                    MovementProbe::ForwardLow,
                    Vec3::new(center.x, feet + 0.1, center.z),
                    rotation,
                    dir,
                    reach,
                );
                let over_hit = cast(
                    MovementProbe::ForwardHigh,
                    Vec3::new(center.x, feet + MAX_STEP_HEIGHT + 0.1, center.z),
                    rotation,
                    dir,
                    reach,
                );
                if forward_hit.is_some() && over_hit.is_none() {
                    let land_origin = Vec3::new(
                        center.x + dir.x * (reach + 0.05),
                        feet + MAX_STEP_HEIGHT + 0.3,
                        center.z + dir.z * (reach + 0.05),
                    );
                    if let Some(land) = cast(
                        MovementProbe::StepDown,
                        land_origin,
                        rotation,
                        Vec3::NEG_Y,
                        MAX_STEP_HEIGHT + 0.4,
                    ) {
                        let step_top = land_origin.y
                            - land.distance
                            - probe_half_height(MovementProbe::StepDown);
                        let step_height = step_top - feet;
                        if step_height > MIN_STEP_HEIGHT && step_height <= MAX_STEP_HEIGHT {
                            let advance = reach + 0.05 - PLAYER_HALF_DEPTH;
                            position_xz = Some(Vec2::new(
                                center.x + dir.x * advance,
                                center.z + dir.z * advance,
                            ));
                            position_y = Some(step_top + PLAYER_HALF_HEIGHT);
                            velocity.y = velocity.y.max(0.0);
                        }
                    }
                }
            }
        }
    } else if velocity.y < -MAX_FALL_SPEED {
        velocity.y = -MAX_FALL_SPEED;
    }

    if grounded {
        state.coyote_until = Some(params.elapsed + COYOTE_TIME_SECS);
    } else if state.coyote_until.is_some_and(|deadline| params.elapsed > deadline) {
        state.coyote_until = None;
    }
    let can_jump =
        grounded || state.coyote_until.is_some_and(|deadline| params.elapsed <= deadline);

    if params.jump_held {
        state.jump_buffered_at = Some(params.elapsed);
    }
    if state
        .jump_buffered_at
        .is_some_and(|buffered| params.elapsed - buffered > JUMP_BUFFER_SECS)
    {
        state.jump_buffered_at = None;
    }
    if state.jump_buffered_at.is_some() && can_jump {
        velocity.y = params.jump_power;
        state.jump_buffered_at = None;
        state.coyote_until = None;
        position_y = None;
    }

    CharacterMoveOutput {
        velocity,
        position_y,
        position_xz,
        grounded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct World {
        floor_y: f32,
        floor_normal: Vec3,
        wall_z: Option<f32>,
        wall_top_y: f32,
    }

    fn flat_floor() -> World {
        World {
            floor_y: 0.0,
            floor_normal: Vec3::Y,
            wall_z: None,
            wall_top_y: 0.0,
        }
    }

    fn make_cast(world: &World) -> impl FnMut(MovementProbe, Vec3, Quat, Vec3, f32) -> Option<CastHit> + '_ {
        move |probe, origin, _rotation, direction, max_distance| {
            let dir = direction.normalize_or_zero();
            let hy = probe_half_height(probe);
            let hx = PLAYER_HALF_WIDTH * 0.9;
            let hz = match probe {
                MovementProbe::Ground => PLAYER_HALF_DEPTH * 0.9,
                _ => 0.01,
            };

            if dir.y < -0.5 {
                let bottom = origin.y - hy;
                let mut hit_y = world.floor_y;
                if let Some(wall_z) = world.wall_z {
                    if origin.z - hz <= wall_z && origin.z + hz >= wall_z - 0.5 {
                        hit_y = hit_y.max(world.wall_top_y);
                    }
                }
                let distance = bottom - hit_y;
                if distance >= 0.0 && distance <= max_distance {
                    return Some(CastHit {
                        distance,
                        normal: world.floor_normal,
                    });
                }
                return None;
            }

            if dir.z.abs() > 0.5 {
                let Some(wall_z) = world.wall_z else { return None };
                let leading = origin.z + dir.z * hz;
                let distance = if dir.z < 0.0 {
                    leading - wall_z
                } else {
                    wall_z - leading
                };
                if distance < 0.0 {
                    return None;
                }
                if origin.y - hy >= world.wall_top_y || origin.y + hy <= world.floor_y {
                    return None;
                }
                if distance <= max_distance {
                    return Some(CastHit {
                        distance,
                        normal: Vec3::Z * dir.z.signum(),
                    });
                }
                return None;
            }

            None
        }
    }

    fn params(wish: Vec2, jump_held: bool, elapsed: f32) -> CharacterMoveParams {
        CharacterMoveParams {
            wish_direction: wish,
            jump_held,
            speed: DEFAULT_WALK_SPEED,
            jump_power: 50.0 * 0.28,
            dt: 1.0 / 60.0,
            elapsed,
        }
    }

    fn run_once(
        world: &World,
        center: Vec3,
        velocity: Vec3,
        state: &mut CharacterMoveState,
        params: &CharacterMoveParams,
    ) -> CharacterMoveOutput {
        let mut cast = make_cast(world);
        character_move(center, velocity, 0.0, state, params, &mut cast)
    }

    #[test]
    fn grounded_on_flat_floor_and_snaps_to_it() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, false, 0.0),
        );
        assert!(output.grounded);
        assert_eq!(output.position_y, None);
        assert_eq!(output.velocity, Vec3::ZERO);
    }

    #[test]
    fn snaps_down_onto_the_floor_when_floating() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.72, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, false, 0.0),
        );
        assert!(output.grounded);
        assert_eq!(output.position_y, Some(0.7));
    }

    #[test]
    fn airborne_when_floor_is_beyond_snap_distance() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7 + 0.3, 0.0),
            Vec3::NEG_Y * 5.0,
            &mut state,
            &params(Vec2::ZERO, false, 0.0),
        );
        assert!(!output.grounded);
        assert_eq!(output.position_y, None);
    }

    #[test]
    fn accelerates_towards_walk_speed() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        for _ in 0..60 {
            run_once(
                &world,
                Vec3::new(0.0, 0.7, 0.0),
                Vec3::ZERO,
                &mut state,
                &params(Vec2::new(0.0, -1.0), false, 0.0),
            );
        }
        assert!((state.horizontal_velocity.y + DEFAULT_WALK_SPEED).abs() < 0.01);
    }

    #[test]
    fn decelerates_to_stop_without_input() {
        let world = flat_floor();
        let mut state = CharacterMoveState {
            horizontal_velocity: Vec2::new(0.0, -DEFAULT_WALK_SPEED),
            ..default()
        };
        for _ in 0..60 {
            run_once(
                &world,
                Vec3::new(0.0, 0.7, 0.0),
                Vec3::new(0.0, 0.0, -DEFAULT_WALK_SPEED),
                &mut state,
                &params(Vec2::ZERO, false, 0.0),
            );
        }
        assert!(state.horizontal_velocity.length() < 0.01);
    }

    #[test]
    fn steps_up_a_step_in_front() {
        let world = World {
            floor_y: 0.0,
            floor_normal: Vec3::Y,
            wall_z: Some(0.0),
            wall_top_y: 0.28,
        };
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.26),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::new(0.0, -1.0), false, 0.0),
        );
        let snapped = output.position_y.expect("expected a step-up snap");
        assert!((snapped - 0.98).abs() < 0.01, "snapped to: {}", snapped);
        let advanced = output.position_xz.expect("expected a step-up advance");
        assert!(
            (advanced.y - 0.11).abs() < 0.01,
            "advanced to: {}",
            advanced.y
        );
    }

    #[test]
    fn does_not_step_up_tall_walls() {
        let world = World {
            floor_y: 0.0,
            floor_normal: Vec3::Y,
            wall_z: Some(0.0),
            wall_top_y: 0.6,
        };
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.26),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::new(0.0, -1.0), false, 0.0),
        );
        assert_eq!(
            output.position_y,
            None,
            "player should neither snap nor step up a tall wall"
        );
    }

    #[test]
    fn does_not_step_up_while_airborne() {
        let world = World {
            floor_y: 0.0,
            floor_normal: Vec3::Y,
            wall_z: Some(0.0),
            wall_top_y: 0.28,
        };
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 1.2, 0.26),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::new(0.0, -1.0), false, 0.0),
        );
        assert_eq!(output.position_y, None);
    }

    #[test]
    fn not_grounded_on_steep_slopes() {
        let world = World {
            floor_y: 0.0,
            floor_normal: Vec3::new(0.5, 0.3, 0.0).normalize(),
            wall_z: None,
            wall_top_y: 0.0,
        };
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, false, 0.0),
        );
        assert!(!output.grounded);
    }

    #[test]
    fn follows_walkable_slopes() {
        let normal = Vec3::new(0.0, 1.0, 1.0).normalize();
        let world = World {
            floor_y: 0.0,
            floor_normal: normal,
            wall_z: None,
            wall_top_y: 0.0,
        };
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::new(0.0, -1.0), false, 0.0),
        );
        assert!(output.grounded);
        let tangent = (Vec3::new(0.0, 0.0, -1.0) - normal * Vec3::new(0.0, 0.0, -1.0).dot(normal))
            .normalize();
        assert!((output.velocity - tangent * DEFAULT_WALK_SPEED).length() < 0.01);
    }

    #[test]
    fn jumps_immediately_when_grounded() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, true, 0.0),
        );
        assert_eq!(output.velocity.y, 50.0 * 0.28);
    }

    #[test]
    fn buffered_jump_fires_on_landing() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let mut output = run_once(
            &world,
            Vec3::new(0.0, 0.7 + 0.3, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, true, 0.0),
        );
        assert_eq!(output.velocity.y, 0.0);
        output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, true, 0.1),
        );
        assert_eq!(output.velocity.y, 50.0 * 0.28);
    }

    #[test]
    fn stale_jump_buffer_expires() {
        let world = flat_floor();
        let mut state = CharacterMoveState {
            jump_buffered_at: Some(0.0),
            ..default()
        };
        let output = run_once(
            &world,
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::ZERO,
            &mut state,
            &params(Vec2::ZERO, false, JUMP_BUFFER_SECS + 0.1),
        );
        assert_eq!(output.velocity.y, 0.0);
    }

    #[test]
    fn clamps_fall_speed() {
        let world = flat_floor();
        let mut state = CharacterMoveState::default();
        let output = run_once(
            &world,
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, -MAX_FALL_SPEED * 3.0, 0.0),
            &mut state,
            &params(Vec2::ZERO, false, 0.0),
        );
        assert_eq!(output.velocity.y, -MAX_FALL_SPEED);
    }
}
