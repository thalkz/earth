use crate::assets::ModelHandle;
use crate::scene::Transform;
use glam::{Quat, Vec3};

pub struct Instance {
    pub transform: Transform,
    pub model: ModelHandle,
}

impl Instance {
    pub fn new(position: Vec3, rotation: Quat, model: ModelHandle) -> Self {
        Self {
            transform: Transform::from_rotation_translation(rotation, position),
            model,
        }
    }

    #[allow(dead_code)]
    pub fn from_transform(transform: Transform, model: ModelHandle) -> Self {
        Self { transform, model }
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: Vec3 = Vec3::new(
    NUM_INSTANCES_PER_ROW as f32 * 1.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 1.5,
);
const SPACE_BETWEEN: f32 = 3.0;

pub fn build_instances(model: ModelHandle) -> Vec<Instance> {
    (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = Vec3::new(
                    SPACE_BETWEEN * x as f32,
                    0.0,
                    SPACE_BETWEEN * z as f32,
                ) - INSTANCE_DISPLACEMENT;

                let rotation = if position == Vec3::ZERO {
                    Quat::IDENTITY
                } else {
                    Quat::from_axis_angle(position.normalize(), 45.0_f32.to_radians())
                };

                Instance::new(position, rotation, model)
            })
        })
        .collect()
}
