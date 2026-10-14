use cgmath::Vector3;
use cgmath::prelude::*;

use crate::instance;

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: Vector3<f32> = cgmath::Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);
const SPACE_BETWEEN: f32 = 3.0;

pub fn build_instances() -> Vec<instance::Instance> {
    (0..NUM_INSTANCES_PER_ROW)
        .flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = cgmath::Vector3 {
                    x: SPACE_BETWEEN * x as f32,
                    y: 0.0,
                    z: SPACE_BETWEEN * z as f32,
                } - INSTANCE_DISPLACEMENT;

                let rotation = if position.is_zero() {
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                instance::Instance::new(position, rotation)
            })
        })
        .collect::<Vec<_>>()
}
