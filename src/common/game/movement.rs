use avian3d::prelude::{
    AngularVelocity, Collider, CollidingEntities, ComputedMass, LinearVelocity, MoveAndSlide,
    MoveAndSlideConfig, MoveAndSlideHitData, MoveAndSlideHitResponse, RigidBody, SimpleCollider,
    SpatialQueryFilter,
};
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

pub const DEFAULT_WALK_SPEED: f32 = 16.0 * 0.28;
pub const DEFAULT_PLAYER_GRAVITY: f32 = 186.9 * 0.28;
pub const GROUND_ACCEL: f32 = 60.0;
pub const GROUND_DECEL: f32 = 60.0;
pub const AIR_ACCEL: f32 = 30.0;
pub const AIR_DECEL: f32 = 45.0;
pub const EXP_GROUND_RESPONSE: f32 = 12.0;
pub const EXP_AIR_RESPONSE: f32 = 5.0;
pub const HARD_SPEED_LIMIT: f32 = 40.0;
pub const MAX_FALL_SPEED: f32 = 14.0;
pub const TURN_SPEED: f32 = 25.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect, serde::Serialize, serde::Deserialize)]
pub enum SpeedResponse {
    #[default]
    Linear,
    Exponential,
}
pub const JUMP_BUFFER_SECS: f32 = 0.15;
pub const HELD_JUMP_REPEAT_GROUND_STEPS: u32 = 3;

pub const PLAYER_HALF_WIDTH: f32 = 1.0 * 0.28;
pub const PLAYER_HALF_HEIGHT: f32 = 2.5 * 0.28;
pub const PLAYER_HALF_DEPTH: f32 = 0.585 * 0.28;

pub const MAX_STEP_HEIGHT: f32 = 0.29;
pub const MIN_STEP_HEIGHT: f32 = 0.05;
pub const MAX_SLOPE_COS: f32 = 0.7071;
pub const GROUNDED_GRACE_SECS: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Resource)]
pub struct CharacterMoveState {
    pub horizontal_velocity: Vec2,
    pub jump_buffered_at: Option<f32>,
    pub prev_jump_held: bool,
    pub ground_contact_steps: u32,
    pub grounded_until: Option<f32>,
    pub support: Option<Entity>,
    pub support_velocity: Vec2,
}

impl Default for CharacterMoveState {
    fn default() -> Self {
        Self {
            horizontal_velocity: Vec2::ZERO,
            jump_buffered_at: None,
            prev_jump_held: false,
            ground_contact_steps: 0,
            grounded_until: None,
            support: None,
            support_velocity: Vec2::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterMoveParams {
    pub wish_direction: Vec2,
    pub jump_held: bool,
    pub speed: f32,
    pub jump_power: f32,
    pub gravity: f32,
    pub speed_response: SpeedResponse,
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
    pub grounded: bool,
}

pub fn apply_gravity(velocity: Vec3, gravity: f32, dt: f32) -> Vec3 {
    let mut velocity = velocity;
    velocity.y -= gravity * dt;
    if velocity.y < -MAX_FALL_SPEED {
        velocity.y = -MAX_FALL_SPEED;
    }
    velocity
}

pub fn horizontal_acceleration(
    horizontal: Vec2,
    grounded: bool,
    wish: Vec2,
    speed: f32,
    dt: f32,
    response: SpeedResponse,
) -> Vec2 {
    let has_wish = wish.length_squared() > 0.001;
    let target = if has_wish { wish * speed } else { Vec2::ZERO };
    let mut horizontal = match response {
        SpeedResponse::Linear => {
            let rate = if grounded {
                if has_wish { GROUND_ACCEL } else { GROUND_DECEL }
            } else if has_wish {
                AIR_ACCEL
            } else {
                AIR_DECEL
            };
            horizontal.move_towards(target, rate * dt)
        }
        SpeedResponse::Exponential => {
            let rate = if grounded {
                EXP_GROUND_RESPONSE
            } else {
                EXP_AIR_RESPONSE
            };
            let t = 1.0 - (-rate * dt).exp();
            horizontal.lerp(target, t)
        }
    };
    let limit = speed.max(HARD_SPEED_LIMIT);
    if horizontal.length() > limit {
        horizontal = horizontal.normalize() * limit;
    }
    horizontal
}

pub fn update_jump(
    state: &mut CharacterMoveState,
    on_ground: bool,
    jump_held: bool,
    elapsed: f32,
) -> bool {
    if on_ground {
        state.ground_contact_steps = state.ground_contact_steps.saturating_add(1);
    } else {
        state.ground_contact_steps = 0;
    }
    let just_pressed = jump_held && !state.prev_jump_held;
    state.prev_jump_held = jump_held;
    let hold_repeat = jump_held
        && state.ground_contact_steps >= HELD_JUMP_REPEAT_GROUND_STEPS;
    if just_pressed || hold_repeat {
        state.jump_buffered_at = Some(elapsed);
    }
    if state
        .jump_buffered_at
        .is_some_and(|buffered| elapsed - buffered > JUMP_BUFFER_SECS)
    {
        state.jump_buffered_at = None;
    }
    if state.jump_buffered_at.is_some() && on_ground {
        state.jump_buffered_at = None;
        return true;
    }
    false
}

pub fn character_move(
    center: Vec3,
    current_velocity: Vec3,
    yaw: f32,
    state: &mut CharacterMoveState,
    params: &CharacterMoveParams,
    collider: &Collider,
    move_and_slide: &MoveAndSlide,
    filter: &SpatialQueryFilter,
    support_velocity: Option<Vec3>,
) -> CharacterMoveOutput {
    let rotation = Quat::from_rotation_y(yaw);
    let has_wish = params.wish_direction.length_squared() > 0.001;
    let dt = params.dt.max(0.0);
    let dt_duration = std::time::Duration::from_secs_f32(dt);
    let elapsed = params.elapsed;

    let mut velocity = apply_gravity(current_velocity, params.gravity, dt);

    let grounded_before = state
        .grounded_until
        .is_some_and(|deadline| elapsed <= deadline);

    state.horizontal_velocity = horizontal_acceleration(
        state.horizontal_velocity,
        grounded_before,
        params.wish_direction,
        params.speed,
        dt,
        params.speed_response,
    );
    let mut horizontal = state.horizontal_velocity;

    if let Some(support) = support_velocity {
        state.support_velocity = Vec2::new(support.x, support.z);
    } else {
        state.support_velocity = Vec2::ZERO;
    }
    if grounded_before {
        horizontal += state.support_velocity;
    }
    let limit = params.speed.max(HARD_SPEED_LIMIT);
    if horizontal.length() > limit {
        horizontal = horizontal.normalize() * limit;
    }
    velocity.x = horizontal.x;
    velocity.z = horizontal.y;

    let (position, projected, on_ground, support_hit) = {
        let mut on_ground = false;
        let mut support_hit: Option<Entity> = None;
        let config = MoveAndSlideConfig {
            skin_width: 0.02,
            ..default()
        };
        let mut slide = |slide_velocity: Vec3, slide_position: Vec3| -> (Vec3, Vec3, bool) {
            let mut wall_hit = false;
            let out = move_and_slide.move_and_slide(
                collider,
                slide_position,
                rotation,
                slide_velocity,
                dt_duration,
                &config,
                filter,
                |hit: MoveAndSlideHitData| {
                    let normal = hit.normal.as_vec3();
                    if normal.y >= MAX_SLOPE_COS {
                        on_ground = true;
                        support_hit = Some(hit.entity);
                    } else {
                        wall_hit = true;
                    }
                    MoveAndSlideHitResponse::Accept
                },
            );
            (out.position, out.projected_velocity, wall_hit)
        };

        let (slide_position, slide_velocity, initial_wall_hit) = slide(velocity, center);
        let input_horizontal = horizontal.length();
        let blocked =
            input_horizontal > 0.0001 && slide_velocity.xz().length() < input_horizontal * 0.7;

        if has_wish && dt > 0.0 && grounded_before && (blocked || initial_wall_hit) {
            let (up_position, _, _) = slide(Vec3::Y * (MAX_STEP_HEIGHT / dt), slide_position);
            let forward_velocity = Vec3::new(horizontal.x, 0.0, horizontal.y);
            let (fwd_position, _, _) = slide(forward_velocity, up_position);
            let (down_position, down_velocity, _) =
                slide(Vec3::NEG_Y * ((MAX_STEP_HEIGHT + 0.15) / dt), fwd_position);
            let rise = down_position.y - slide_position.y;
            if rise >= MIN_STEP_HEIGHT {
                (down_position, down_velocity, on_ground, support_hit)
            } else {
                (slide_position, slide_velocity, on_ground, support_hit)
            }
        } else {
            (slide_position, slide_velocity, on_ground, support_hit)
        }
    };

    let grounded = on_ground
        || (projected.y <= 0.5
            && state
                .grounded_until
                .is_some_and(|deadline| elapsed <= deadline));
    if on_ground {
        state.grounded_until = Some(elapsed + GROUNDED_GRACE_SECS);
    } else if state
        .grounded_until
        .is_some_and(|deadline| elapsed > deadline)
    {
        state.grounded_until = None;
        state.support_velocity = Vec2::ZERO;
    }

    if let Some(hit) = support_hit {
        state.support = Some(hit);
    }

    let mut output_velocity = Vec3::new(velocity.x, projected.y, velocity.z);
    if update_jump(state, on_ground, params.jump_held, elapsed) {
        output_velocity.y = params.jump_power;
    }

    CharacterMoveOutput {
        velocity: output_velocity,
        position_y: Some(position.y),
        position_xz: Some(Vec2::new(position.x, position.z)),
        grounded,
    }
}

pub const PUSH_FACING_MIN_DOT: f32 = 0.4;
pub const PUSH_FORCE: f32 = 260.0;
pub const MAX_PUSH_ACCEL: f32 = 110.0;
pub const PUSH_ANGULAR_GAIN: f32 = 3.0;
pub const PUSH_MAX_ANGULAR_SPEED: f32 = 5.0;

pub fn push_collided_dynamic_bodies<F: QueryFilter>(
    player_position: Vec3,
    player_horizontal_velocity: Vec2,
    colliding_entities: &CollidingEntities,
    bodies: &mut Query<
        (
            Entity,
            &Transform,
            &RigidBody,
            &mut LinearVelocity,
            &mut AngularVelocity,
            &ComputedMass,
            &Collider,
        ),
        F,
    >,
    dt: f32,
) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    let push_speed = player_horizontal_velocity.length();
    if !push_speed.is_finite() || push_speed < 0.1 {
        return;
    }
    let push_dir = player_horizontal_velocity / push_speed;
    if !push_dir.is_finite() {
        return;
    }
    let player_feet_y = player_position.y - PLAYER_HALF_HEIGHT;
    for &other in colliding_entities.0.iter() {
        let Ok((_, body_transform, body, mut lin_vel, mut ang_vel, mass, collider)) =
            bodies.get_mut(other)
        else {
            continue;
        };
        if !body.is_dynamic() {
            continue;
        }
        let mass_value = mass.value();
        if !mass_value.is_finite() || mass_value < 1e-4 {
            continue;
        }
        let body_top = collider
            .aabb(body_transform.translation, body_transform.rotation)
            .max
            .y;
        if body_top <= player_feet_y + MAX_STEP_HEIGHT + 0.01 {
            continue;
        }
        let to_body = body_transform.translation.xz() - player_position.xz();
        if to_body.length_squared() < 1e-8 {
            continue;
        }
        let facing_dot = to_body.normalize_or_zero().dot(push_dir);
        if !facing_dot.is_finite() || facing_dot < PUSH_FACING_MIN_DOT {
            continue;
        }
        let old_horizontal = lin_vel.0.xz();
        if !old_horizontal.is_finite() {
            continue;
        }
        let along = old_horizontal.dot(push_dir);
        let facing_strength = 0.4 + 0.6 * facing_dot.clamp(0.0, 1.0);
        if along < push_speed {
            let acceleration =
                (PUSH_FORCE / mass_value.max(f32::EPSILON)).min(MAX_PUSH_ACCEL) * facing_strength;
            let new_along = (along + acceleration * dt).min(push_speed);
            let new_horizontal = push_dir * new_along + (old_horizontal - push_dir * along);
            if new_horizontal.is_finite() {
                lin_vel.0.x = new_horizontal.x;
                lin_vel.0.z = new_horizontal.y;
            }
        }
        let yaw_torque = push_dir.y * to_body.x - push_dir.x * to_body.y;
        if yaw_torque.is_finite() && yaw_torque.abs() > 1e-4 {
            let angular_push =
                (yaw_torque * PUSH_ANGULAR_GAIN * facing_strength / mass_value) * dt;
            let new_yaw = (ang_vel.0.y + angular_push).clamp(
                -PUSH_MAX_ANGULAR_SPEED,
                PUSH_MAX_ANGULAR_SPEED,
            );
            if new_yaw.is_finite() {
                ang_vel.0.y = new_yaw;
            }
        }
    }
}

//bunch of vars
//move validation : basically, this is the server-side part that checks if a client is somehow cheating bc we are now client authoritative
//we have a lot of tests because these values are very sensitive: can either fuck up movement or fuck up validation !!!
pub const VALIDATION_SUPPORT_ALLOWANCE: f32 = 8.0;
pub const VALIDATION_RISE_ALLOWANCE: f32 = 12.0;
pub const VALIDATION_HORIZ_GRACE: f32 = 0.7;
pub const VALIDATION_VERT_GRACE: f32 = 0.35;
pub const VALIDATION_BUDGET_SLACK: f32 = 2.5;
pub const VALIDATION_BUDGET_MAX: f32 = 4.0;
pub const VALIDATION_MAX_DT: f32 = 1.0;
pub const VALIDATION_VELOCITY_TOLERANCE: f32 = 1.5;
pub const VALIDATION_VIOLATION_LOG_THRESHOLD: u32 = 40;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveValidationConfig {
    pub hard_speed_limit: f32,
    pub max_fall_speed: f32,
    pub jump_power: f32,
    pub gravity_mag: f32,
}

impl MoveValidationConfig {
    pub fn new(
        hard_speed_limit: f32,
        max_fall_speed: f32,
        jump_power: f32,
        gravity_mag: f32,
    ) -> Self {
        Self {
            hard_speed_limit,
            max_fall_speed,
            jump_power,
            gravity_mag,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientMoveClaim {
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub yaw: f32,
    pub in_first_person: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidatedMove {
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub yaw: f32,
    pub in_first_person: bool,
    pub accepted: bool,
}
pub fn validate_client_move(
    previous: Option<&ValidatedMove>,
    claim: ClientMoveClaim,
    dt: f32,
    config: &MoveValidationConfig,
    budget: &mut f32,
) -> ValidatedMove {
    let mut rejected = !claim.position.is_finite() || !claim.velocity.is_finite();
    let mut position = claim.position;
    let mut velocity = claim.velocity;

    if rejected {
        if let Some(prev) = previous {
            position = prev.position;
            velocity = prev.velocity;
        } else {
            position = Vec3::ZERO;
            velocity = Vec3::ZERO;
        }
    }

    let dt = dt.clamp(0.0, VALIDATION_MAX_DT);
    if let Some(prev) = previous {
        if dt > 0.0 {
            let capacity = config.hard_speed_limit + VALIDATION_SUPPORT_ALLOWANCE;
            let horiz_limit = capacity * dt + VALIDATION_HORIZ_GRACE;
            let max_rise =
                (config.jump_power + VALIDATION_RISE_ALLOWANCE) * dt + VALIDATION_VERT_GRACE;
            let max_fall = config.max_fall_speed * dt
                + 0.5 * config.gravity_mag * dt * dt
                + VALIDATION_VERT_GRACE;

            let mut delta = position - prev.position;
            let horiz_len = Vec2::new(delta.x, delta.z).length();

            *budget = (*budget + capacity * dt).min(VALIDATION_BUDGET_MAX);
            *budget -= horiz_len;
            if *budget < -VALIDATION_BUDGET_SLACK {
                let surplus = -*budget - VALIDATION_BUDGET_SLACK;
                let allowed = (horiz_len - surplus).max(0.0);
                if horiz_len > 1e-6 {
                    position.x = prev.position.x + delta.x / horiz_len * allowed;
                    position.z = prev.position.z + delta.z / horiz_len * allowed;
                }
                *budget = -VALIDATION_BUDGET_SLACK;
                rejected = true;
            }

            if position.y > prev.position.y + max_rise {
                position.y = prev.position.y + max_rise;
                rejected = true;
            } else if position.y < prev.position.y - max_fall {
                position.y = prev.position.y - max_fall;
                rejected = true;
            }

            delta = position - prev.position;
            let horiz_len = Vec2::new(delta.x, delta.z).length();
            if horiz_len > horiz_limit {
                let scale = horiz_limit / horiz_len;
                position.x = prev.position.x + delta.x * scale;
                position.z = prev.position.z + delta.z * scale;
                rejected = true;
            }
        }

        let max_horizontal = (config.hard_speed_limit + VALIDATION_SUPPORT_ALLOWANCE)
            * VALIDATION_VELOCITY_TOLERANCE;
        let horiz_velocity = Vec2::new(velocity.x, velocity.z);
        if horiz_velocity.length() > max_horizontal {
            let scale = max_horizontal / horiz_velocity.length();
            velocity.x *= scale;
            velocity.z *= scale;
            rejected = true;
        }
        if velocity.y > config.jump_power * VALIDATION_VELOCITY_TOLERANCE {
            velocity.y = config.jump_power * VALIDATION_VELOCITY_TOLERANCE;
            rejected = true;
        } else if velocity.y < -config.max_fall_speed * VALIDATION_VELOCITY_TOLERANCE {
            velocity.y = -config.max_fall_speed * VALIDATION_VELOCITY_TOLERANCE;
            rejected = true;
        }
    }

    ValidatedMove {
        position,
        velocity,
        grounded: claim.grounded,
        yaw: claim.yaw,
        in_first_person: claim.in_first_person,
        accepted: !rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MoveValidationConfig {
        MoveValidationConfig::new(HARD_SPEED_LIMIT, MAX_FALL_SPEED, 14.0, 52.332)
    }

    fn claim(position: Vec3) -> ClientMoveClaim {
        ClientMoveClaim {
            position,
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
        }
    }

    fn accepted(m: &ValidatedMove) -> &ValidatedMove {
        assert!(m.accepted, "move was rejected: {m:?}");
        m
    }

    #[test]
    fn accepts_step_up_rise_even_when_messages_arrive_in_bursts() {
        let mut budget = VALIDATION_BUDGET_MAX;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(0.0, 0.98, 0.0)),
            0.0,
            &config(),
            &mut budget,
        );
        accepted(&out);
    }

    #[test]
    fn accepts_max_speed_displacement_even_when_messages_arrive_in_bursts() {
        let mut budget = VALIDATION_BUDGET_MAX;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(0.0, 0.7, -HARD_SPEED_LIMIT / 60.0)),
            0.0,
            &config(),
            &mut budget,
        );
        accepted(&out);
    }

    #[test]
    fn accepts_normal_walking() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::new(0.0, 0.0, -4.48),
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let mut last = prev;
        for _ in 0..60 {
            let next = validate_client_move(
                Some(&last),
                claim(Vec3::new(0.0, 0.7, last.position.z - 4.48 / 60.0)),
                1.0 / 60.0,
                &config(),
                &mut budget,
            );
            accepted(&next);
            last = next;
        }
        assert!((last.position.z - (-4.48)).abs() < 0.01);
    }

    #[test]
    fn accepts_jump_rise_and_fall() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let mut last = prev;
        let rise = validate_client_move(
            Some(&last),
            claim(Vec3::new(0.0, 0.7 + 14.0 / 60.0, 0.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        accepted(&rise);
        last = rise;
        let fall = validate_client_move(
            Some(&last),
            claim(Vec3::new(0.0, last.position.y - MAX_FALL_SPEED / 60.0, 0.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        accepted(&fall);
    }

    #[test]
    fn rejects_teleport() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(50.0, 0.7, 0.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        assert!(!out.accepted);
        let capacity = HARD_SPEED_LIMIT + VALIDATION_SUPPORT_ALLOWANCE;
        assert!(out.position.x <= capacity / 60.0 + VALIDATION_HORIZ_GRACE + 1e-3);
    }

    #[test]
    fn rejects_sustained_overspeed() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::new(0.0, 0.0, -HARD_SPEED_LIMIT * 2.0),
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let mut last = prev;
        let mut rejected = 0;
        for _ in 0..120 {
            let next = validate_client_move(
                Some(&last),
                claim(Vec3::new(
                    0.0,
                    0.7,
                    last.position.z - HARD_SPEED_LIMIT * 2.0 / 60.0,
                )),
                1.0 / 60.0,
                &config(),
                &mut budget,
            );
            if !next.accepted {
                rejected += 1;
            }
            last = next;
        }
        assert!(
            rejected > 0,
            "sustained 2x overspeed must eventually be clamped"
        );
        assert!(
            last.position.z > -HARD_SPEED_LIMIT * 2.0 * 2.0,
            "cheater traveled too far: z={}",
            last.position.z
        );
    }

    #[test]
    fn clamps_super_jump_rise() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(0.0, 0.7 + 100.0, 0.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        assert!(!out.accepted);
        let max_rise = (14.0 + VALIDATION_RISE_ALLOWANCE) / 60.0 + VALIDATION_VERT_GRACE;
        assert!((out.position.y - (0.7 + max_rise)).abs() < 1e-3);
    }

    #[test]
    fn clamps_terminal_fall() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 50.0, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(0.0, 0.0, 0.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        assert!(!out.accepted);
        let max_fall = MAX_FALL_SPEED / 60.0 + 0.5 * 52.332 / 3600.0 + VALIDATION_VERT_GRACE;
        assert!((out.position.y - (50.0 - max_fall)).abs() < 1e-3);
    }

    #[test]
    fn rejects_nan_and_reuses_last_position() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(3.0, 0.7, -2.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(f32::NAN, 0.7, -2.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        assert!(!out.accepted);
        assert_eq!(out.position, prev.position);
    }

    #[test]
    fn accepts_small_bursts_within_budget() {
        let mut budget = VALIDATION_BUDGET_MAX;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let out = validate_client_move(
            Some(&prev),
            claim(Vec3::new(0.0, 0.7, -1.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        assert!(
            out.accepted,
            "a small burst within the budget must be accepted"
        );
    }

    #[test]
    fn clamps_overspeed_velocity() {
        let mut budget = 0.0;
        let prev = ValidatedMove {
            position: Vec3::new(0.0, 0.7, 0.0),
            velocity: Vec3::ZERO,
            grounded: true,
            yaw: 0.0,
            in_first_person: false,
            accepted: true,
        };
        let mut claim = claim(Vec3::new(0.0, 0.7, 0.0));
        claim.velocity = Vec3::new(0.0, 0.0, -500.0);
        let out = validate_client_move(Some(&prev), claim, 1.0 / 60.0, &config(), &mut budget);
        assert!(!out.accepted);
        let max_horizontal =
            (HARD_SPEED_LIMIT + VALIDATION_SUPPORT_ALLOWANCE) * VALIDATION_VELOCITY_TOLERANCE;
        assert!((out.velocity.z.abs() - max_horizontal).abs() < 1e-3);
    }

    #[test]
    fn first_move_is_accepted_unconditionally() {
        let mut budget = 0.0;
        let out = validate_client_move(
            None,
            claim(Vec3::new(42.0, 3.0, -7.0)),
            1.0 / 60.0,
            &config(),
            &mut budget,
        );
        accepted(&out);
        assert_eq!(out.position, Vec3::new(42.0, 3.0, -7.0));
    }

    #[test]
    fn gravity_pulls_velocity_down() {
        let velocity = apply_gravity(Vec3::ZERO, 52.332, 1.0 / 60.0);
        assert!((velocity.y + 52.332 / 60.0).abs() < 1e-4);
    }

    #[test]
    fn gravity_scales_with_player_gravity() {
        let velocity = apply_gravity(Vec3::ZERO, 2.0 * 52.332, 1.0 / 60.0);
        assert!((velocity.y + 2.0 * 52.332 / 60.0).abs() < 1e-4);
    }

    #[test]
    fn gravity_clamps_fall_speed() {
        let velocity = apply_gravity(
            Vec3::NEG_Y * (MAX_FALL_SPEED * 3.0),
            52.332,
            1.0 / 60.0,
        );
        assert_eq!(velocity.y, -MAX_FALL_SPEED);
    }

    #[test]
    fn accelerates_towards_walk_speed() {
        let mut horizontal = Vec2::ZERO;
        for _ in 0..120 {
            horizontal = horizontal_acceleration(
                horizontal,
                true,
                Vec2::new(0.0, -1.0),
                DEFAULT_WALK_SPEED,
                1.0 / 60.0,
                SpeedResponse::Linear,
            );
        }
        assert!((horizontal.y + DEFAULT_WALK_SPEED).abs() < 0.01);
    }

    #[test]
    fn exponential_acceleration_converges_to_walk_speed() {
        let mut horizontal = Vec2::ZERO;
        for _ in 0..240 {
            horizontal = horizontal_acceleration(
                horizontal,
                true,
                Vec2::new(0.0, -1.0),
                DEFAULT_WALK_SPEED,
                1.0 / 60.0,
                SpeedResponse::Exponential,
            );
        }
        assert!((horizontal.y + DEFAULT_WALK_SPEED).abs() < 0.05);
    }

    #[test]
    fn exponential_acceleration_is_smooth() {
        let first = horizontal_acceleration(
            Vec2::ZERO,
            true,
            Vec2::new(0.0, -1.0),
            DEFAULT_WALK_SPEED,
            1.0 / 60.0,
            SpeedResponse::Exponential,
        );
        let second = horizontal_acceleration(
            first,
            true,
            Vec2::new(0.0, -1.0),
            DEFAULT_WALK_SPEED,
            1.0 / 60.0,
            SpeedResponse::Exponential,
        );
        assert!(second.length() > first.length());
        assert!(second.length() < DEFAULT_WALK_SPEED);
    }

    #[test]
    fn decelerates_to_stop_without_input() {
        let mut horizontal = Vec2::new(0.0, -DEFAULT_WALK_SPEED);
        for _ in 0..120 {
            horizontal = horizontal_acceleration(
                horizontal,
                true,
                Vec2::ZERO,
                DEFAULT_WALK_SPEED,
                1.0 / 60.0,
                SpeedResponse::Linear,
            );
        }
        assert!(horizontal.length() < 0.01);
    }

    #[test]
    fn air_acceleration_is_slower_than_ground() {
        let air = horizontal_acceleration(
            Vec2::ZERO,
            false,
            Vec2::new(0.0, -1.0),
            DEFAULT_WALK_SPEED,
            1.0 / 60.0,
            SpeedResponse::Linear,
        );
        let ground = horizontal_acceleration(
            Vec2::ZERO,
            true,
            Vec2::new(0.0, -1.0),
            DEFAULT_WALK_SPEED,
            1.0 / 60.0,
            SpeedResponse::Linear,
        );
        assert!(air.length() < ground.length());
    }

    #[test]
    fn clamps_to_hard_speed_limit() {
        let horizontal = horizontal_acceleration(
            Vec2::new(HARD_SPEED_LIMIT * 2.0, 0.0),
            true,
            Vec2::new(1.0, 0.0),
            HARD_SPEED_LIMIT,
            1.0 / 60.0,
            SpeedResponse::Linear,
        );
        assert!((horizontal.length() - HARD_SPEED_LIMIT).abs() < 1e-4);
    }

    #[test]
    fn high_configured_speed_is_not_clamped_to_hard_limit() {
        let fast = HARD_SPEED_LIMIT * 3.0;
        let mut horizontal = Vec2::ZERO;
        for _ in 0..600 {
            horizontal = horizontal_acceleration(
                horizontal,
                true,
                Vec2::new(1.0, 0.0),
                fast,
                1.0 / 60.0,
                SpeedResponse::Linear,
            );
        }
        assert!((horizontal.length() - fast).abs() < 0.05);
    }

    #[test]
    fn jumps_immediately_when_grounded() {
        let mut state = CharacterMoveState::default();
        assert!(update_jump(&mut state, true, true, 0.0));
        assert_eq!(state.jump_buffered_at, None);
    }

    #[test]
    fn buffered_jump_fires_on_landing() {
        let mut state = CharacterMoveState::default();
        assert!(!update_jump(&mut state, false, true, 0.0));
        assert!(update_jump(&mut state, true, true, 0.1));
    }

    #[test]
    fn stale_jump_buffer_expires() {
        let mut state = CharacterMoveState {
            jump_buffered_at: Some(0.0),
            ..default()
        };
        assert!(!update_jump(
            &mut state,
            true,
            false,
            JUMP_BUFFER_SECS + 0.1
        ));
    }

    #[test]
    fn no_jump_after_leaving_ground() {
        let mut state = CharacterMoveState::default();
        assert!(!update_jump(&mut state, true, false, 0.0));
        assert!(!update_jump(&mut state, false, true, 0.05));
    }

    #[test]
    fn buffered_press_still_fires_on_landing_but_not_mid_air() {
        let mut state = CharacterMoveState::default();
        assert!(!update_jump(&mut state, true, false, 0.0));
        assert!(!update_jump(&mut state, false, true, 0.3));
        assert!(update_jump(&mut state, true, false, 0.35));
    }

    #[test]
    fn held_jump_repeats_only_after_landing_beat() {
        let mut state = CharacterMoveState {
            prev_jump_held: true,
            ..default()
        };
        assert!(!update_jump(&mut state, true, true, 0.0));
        assert!(!update_jump(&mut state, true, true, 0.016));
        assert!(update_jump(&mut state, true, true, 0.032));
    }

    #[test]
    fn held_jump_never_fires_mid_air() {
        let mut state = CharacterMoveState {
            prev_jump_held: true,
            ..default()
        };
        for step in 0..5 {
            assert!(
                !update_jump(&mut state, false, true, step as f32 * 0.016),
                "held jump fired mid-air on step {step}"
            );
        }
    }
}
