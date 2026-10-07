use bevy::prelude::*;

pub const AVATAR_ROOT_OFFSET_Y: f32 = -0.7;
pub const AVATAR_STUD_SCALE: f32 = 0.28;

pub const LEFT_ARM_PIVOT: Vec3 = Vec3::new(-1.5, 3.9, 0.0);
pub const RIGHT_ARM_PIVOT: Vec3 = Vec3::new(1.5, 3.9, 0.0);
pub const LEFT_LEG_PIVOT: Vec3 = Vec3::new(-0.5, 2.0, 0.0);
pub const RIGHT_LEG_PIVOT: Vec3 = Vec3::new(0.5, 2.0, 0.0);
pub const HEAD_PIVOT: Vec3 = Vec3::new(0.0, 3.9, 0.0);

const WALK_SPEED_STUDS: f32 = 1.0;
const PHASE_PER_STUD: f32 = 0.8;
const POSE_TWEEN_RATE: f32 = 10.0;
const VELOCITY_SMOOTHING: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarLimbKind {
    Head,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl AvatarLimbKind {
    pub const ALL: [AvatarLimbKind; 5] = [
        AvatarLimbKind::Head,
        AvatarLimbKind::LeftArm,
        AvatarLimbKind::RightArm,
        AvatarLimbKind::LeftLeg,
        AvatarLimbKind::RightLeg,
    ];

    pub fn pivot(&self) -> Vec3 {
        match self {
            AvatarLimbKind::Head => HEAD_PIVOT,
            AvatarLimbKind::LeftArm => LEFT_ARM_PIVOT,
            AvatarLimbKind::RightArm => RIGHT_ARM_PIVOT,
            AvatarLimbKind::LeftLeg => LEFT_LEG_PIVOT,
            AvatarLimbKind::RightLeg => RIGHT_LEG_PIVOT,
        }
    }

    pub fn part_name(&self) -> &'static str {
        match self {
            AvatarLimbKind::Head => "Head",
            AvatarLimbKind::LeftArm => "LeftArm",
            AvatarLimbKind::RightArm => "RightArm",
            AvatarLimbKind::LeftLeg => "LeftLeg",
            AvatarLimbKind::RightLeg => "RightLeg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarLocomotion {
    #[default]
    Idle,
    Walk,
    Jump,
    Fall,
}

#[derive(Component)]
pub struct AvatarLimb {
    pub kind: AvatarLimbKind,
    pub current: Vec2,
}

#[derive(Component)]
pub struct AvatarRig {
    pub limbs: [Entity; 5],
}

#[derive(Component)]
pub struct AvatarAnimState {
    pub locomotion: AvatarLocomotion,
    pub phase: f32,
    pub velocity: Vec3,
    pub last_position: Vec3,
    pub initialized: bool,
}

impl Default for AvatarAnimState {
    fn default() -> Self {
        Self {
            locomotion: AvatarLocomotion::Idle,
            phase: 0.0,
            velocity: Vec3::ZERO,
            last_position: Vec3::ZERO,
            initialized: false,
        }
    }
}

fn damp(current: f32, target: f32, rate: f32, delta_secs: f32) -> f32 {
    current + (target - current) * (1.0 - (-rate * delta_secs).exp())
}

fn limb_pose_target(kind: AvatarLimbKind, locomotion: AvatarLocomotion, phase: f32, t: f32) -> Vec2 {
    match locomotion {
        AvatarLocomotion::Idle => {
            let sway = (t * 1.6).sin() * 0.03;
            match kind {
                AvatarLimbKind::Head => Vec2::new(sway * 0.5, 0.0),
                AvatarLimbKind::LeftArm | AvatarLimbKind::RightArm => {
                    let outward = match kind {
                        AvatarLimbKind::LeftArm => -0.02,
                        _ => 0.02,
                    };
                    Vec2::new(0.02 + sway, outward)
                }
                AvatarLimbKind::LeftLeg | AvatarLimbKind::RightLeg => Vec2::ZERO,
            }
        }
        AvatarLocomotion::Walk => {
            let swing = phase.sin();
            match kind {
                AvatarLimbKind::Head => Vec2::new((phase * 2.0).sin() * 0.02, 0.0),
                AvatarLimbKind::LeftArm => Vec2::new(-swing * 0.55, -0.02),
                AvatarLimbKind::RightArm => Vec2::new(swing * 0.55, 0.02),
                AvatarLimbKind::LeftLeg => Vec2::new(swing * 0.65, 0.0),
                AvatarLimbKind::RightLeg => Vec2::new(-swing * 0.65, 0.0),
            }
        }
        AvatarLocomotion::Jump => match kind {
            AvatarLimbKind::Head => Vec2::new(-0.1, 0.0),
            AvatarLimbKind::LeftArm => Vec2::new(-2.85, -0.02),
            AvatarLimbKind::RightArm => Vec2::new(-2.85, 0.02),
            AvatarLimbKind::LeftLeg => Vec2::new(0.1, -0.05),
            AvatarLimbKind::RightLeg => Vec2::new(0.1, 0.05),
        },
        AvatarLocomotion::Fall => {
            let dangle = (t * 2.2).sin() * 0.03;
            match kind {
                AvatarLimbKind::Head => Vec2::new(0.2 + dangle, 0.0),
                AvatarLimbKind::LeftArm => Vec2::new(-2.85 + dangle, -0.04),
                AvatarLimbKind::RightArm => Vec2::new(-2.85 + dangle, 0.04),
                AvatarLimbKind::LeftLeg => Vec2::new(0.1 + dangle, -0.04),
                AvatarLimbKind::RightLeg => Vec2::new(0.1 + dangle, 0.04),
            }
        }
    }
}

fn locomotion_for(grounded: bool, velocity_studs: Vec3) -> AvatarLocomotion {
    if !grounded {
        if velocity_studs.y > 1.5 {
            return AvatarLocomotion::Jump;
        }
        return AvatarLocomotion::Fall;
    }
    if Vec2::new(velocity_studs.x, velocity_studs.z).length() > WALK_SPEED_STUDS {
        return AvatarLocomotion::Walk;
    }
    AvatarLocomotion::Idle
}

pub fn update_avatar_anim_state(
    mut roots: Query<(&crate::client::PlayerVisualChild, &mut AvatarAnimState)>,
    players: Query<(
        &crate::common::net::components::Player,
        &GlobalTransform,
        Option<&crate::client::LocalPlayer>,
        Option<&crate::common::game::movement::PlayerMovementPlan>,
    )>,
    time: Res<Time>,
) {
    let delta_secs = time.delta_secs();
    for (visual_child, mut anim) in &mut roots {
        let Ok((_, transform, local_opt, plan_opt)) = players.get(visual_child.parent) else {
            continue;
        };
        let position = transform.translation();
        if !anim.initialized {
            anim.last_position = position;
            anim.initialized = true;
        }

        let (velocity_ms, grounded) = match (local_opt, plan_opt) {
            (Some(_), Some(plan)) => (plan.velocity, plan.grounded),
            _ => {
                let raw = if delta_secs > 0.0001 {
                    (position - anim.last_position) / delta_secs
                } else {
                    Vec3::ZERO
                };
                let smoothed = anim.velocity.lerp(raw, VELOCITY_SMOOTHING);
                (smoothed, smoothed.y.abs() < 1.5 * AVATAR_STUD_SCALE)
            }
        };
        anim.velocity = velocity_ms;
        anim.last_position = position;

        let velocity_studs = velocity_ms / AVATAR_STUD_SCALE;
        anim.locomotion = locomotion_for(grounded, velocity_studs);

        if anim.locomotion == AvatarLocomotion::Walk {
            let speed = Vec2::new(velocity_studs.x, velocity_studs.z).length();
            anim.phase += delta_secs * speed * PHASE_PER_STUD;
        }
    }
}

pub fn pose_avatar_limbs(
    mut roots: Query<(&AvatarRig, &mut AvatarAnimState, &mut Transform)>,
    mut limbs: Query<(&mut AvatarLimb, &mut Transform), Without<AvatarRig>>,
    time: Res<Time>,
) {
    let delta_secs = time.delta_secs();
    let now = time.elapsed_secs();
    for (rig, anim, mut root_transform) in &mut roots {
        let mut bob = 0.0;
        if anim.locomotion == AvatarLocomotion::Walk {
            bob = (anim.phase * 2.0).sin() * 0.004;
        }
        root_transform.translation.y = AVATAR_ROOT_OFFSET_Y + bob;

        for (index, kind) in AvatarLimbKind::ALL.iter().enumerate() {
            let Ok((mut limb, mut limb_transform)) = limbs.get_mut(rig.limbs[index]) else {
                continue;
            };
            let target = limb_pose_target(*kind, anim.locomotion, anim.phase, now);
            limb.current.x = damp(limb.current.x, target.x, POSE_TWEEN_RATE, delta_secs);
            limb.current.y = damp(limb.current.y, target.y, POSE_TWEEN_RATE, delta_secs);
            limb_transform.rotation =
                Quat::from_euler(EulerRot::XYZ, limb.current.x, 0.0, limb.current.y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_limbs_rest_near_neutral() {
        for kind in AvatarLimbKind::ALL {
            let pose = limb_pose_target(kind, AvatarLocomotion::Idle, 0.0, 0.0);
            assert!(pose.x.abs() < 0.1, "{:?} idle rx too large", kind);
            assert!(pose.y.abs() < 0.1, "{:?} idle rz too large", kind);
        }
    }

    #[test]
    fn walk_swings_opposite_limbs_symmetrically() {
        let phase = 1.0;
        let left_arm = limb_pose_target(AvatarLimbKind::LeftArm, AvatarLocomotion::Walk, phase, 0.0);
        let right_arm =
            limb_pose_target(AvatarLimbKind::RightArm, AvatarLocomotion::Walk, phase, 0.0);
        let left_leg =
            limb_pose_target(AvatarLimbKind::LeftLeg, AvatarLocomotion::Walk, phase, 0.0);
        let right_leg =
            limb_pose_target(AvatarLimbKind::RightLeg, AvatarLocomotion::Walk, phase, 0.0);
        assert!((left_arm.x + right_arm.x).abs() < 1e-6);
        assert!((left_leg.x + right_leg.x).abs() < 1e-6);
        assert!(left_arm.x * left_leg.x < 0.0);
        assert!(left_arm.x.abs() > 0.3);
        assert!(left_leg.x.abs() > 0.3);
    }

    #[test]
    fn jump_raises_arms_straight_up() {
        for kind in [AvatarLimbKind::LeftArm, AvatarLimbKind::RightArm] {
            let pose = limb_pose_target(kind, AvatarLocomotion::Jump, 0.0, 0.0);
            assert!(pose.x < -2.0, "{:?} arms not raised", kind);
            assert!(pose.y.abs() <= 0.03, "{:?} arms too splayed", kind);
        }
        for kind in [AvatarLimbKind::LeftLeg, AvatarLimbKind::RightLeg] {
            let pose = limb_pose_target(kind, AvatarLocomotion::Jump, 0.0, 0.0);
            assert!(pose.y.abs() <= 0.1, "{:?} legs too spread", kind);
            assert!(pose.y.abs() > 0.0, "{:?} legs not spread", kind);
        }
    }

    #[test]
    fn fall_dangles_limbs_with_raised_arms() {
        for kind in [AvatarLimbKind::LeftArm, AvatarLimbKind::RightArm] {
            let pose = limb_pose_target(kind, AvatarLocomotion::Fall, 0.0, 0.0);
            assert!(pose.x < -2.0, "{:?} fall arms not raised", kind);
            assert!(pose.y.abs() <= 0.05, "{:?} fall arms too splayed", kind);
        }
        for kind in [AvatarLimbKind::LeftLeg, AvatarLimbKind::RightLeg] {
            let pose = limb_pose_target(kind, AvatarLocomotion::Fall, 0.0, 0.0);
            assert!(pose.y.abs() <= 0.06, "{:?} fall legs too spread", kind);
            assert!(pose.y.abs() > 0.0, "{:?} fall legs not spread", kind);
        }
        let head = limb_pose_target(AvatarLimbKind::Head, AvatarLocomotion::Fall, 0.0, 0.0);
        assert!(head.x > 0.0, "fall head not drooping");
    }

    #[test]
    fn locomotion_selects_expected_states() {
        assert_eq!(
            locomotion_for(true, Vec3::ZERO),
            AvatarLocomotion::Idle
        );
        assert_eq!(
            locomotion_for(true, Vec3::new(8.0, 0.0, 0.0)),
            AvatarLocomotion::Walk
        );
        assert_eq!(
            locomotion_for(false, Vec3::new(0.0, 8.0, 0.0)),
            AvatarLocomotion::Jump
        );
        assert_eq!(
            locomotion_for(false, Vec3::new(0.0, -8.0, 0.0)),
            AvatarLocomotion::Fall
        );
    }

    #[test]
    fn damp_converges_toward_target() {
        let mut value = 0.0;
        for _ in 0..120 {
            value = damp(value, 1.0, 10.0, 1.0 / 60.0);
        }
        assert!((value - 1.0).abs() < 0.01);
        assert_eq!(damp(2.0, 2.0, 10.0, 1.0 / 60.0), 2.0);
    }
}
