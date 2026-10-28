use glam::camera::rh::{proj::directx, view};
use glam::{Mat4, Vec3};
use std::collections::HashSet;
use winit::keyboard::KeyCode;

pub struct Camera {
    pub position: Vec3,
    /// Radians; 0 looks towards -Z.
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: -0.3,
            fov_y: 70f32.to_radians(),
            aspect,
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn view_proj(&self) -> Mat4 {
        let view = view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        let proj = directx::perspective(self.fov_y, self.aspect, 0.1, 1000.0);
        proj * view
    }

    pub fn rotate(&mut self, dx: f64, dy: f64) {
        const SENSITIVITY: f32 = 0.0025;
        self.yaw -= dx as f32 * SENSITIVITY;
        self.pitch = (self.pitch - dy as f32 * SENSITIVITY).clamp(-1.55, 1.55);
    }

    /// Creative-style flight: WASD on the horizontal plane, Space/Shift vertical.
    pub fn update(&mut self, keys: &HashSet<KeyCode>, dt: f32) {
        let speed = if keys.contains(&KeyCode::ControlLeft) {
            40.0
        } else {
            12.0
        };
        let flat_forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = flat_forward.cross(Vec3::Y);
        let mut dir = Vec3::ZERO;
        if keys.contains(&KeyCode::KeyW) {
            dir += flat_forward;
        }
        if keys.contains(&KeyCode::KeyS) {
            dir -= flat_forward;
        }
        if keys.contains(&KeyCode::KeyD) {
            dir += right;
        }
        if keys.contains(&KeyCode::KeyA) {
            dir -= right;
        }
        if keys.contains(&KeyCode::Space) {
            dir += Vec3::Y;
        }
        if keys.contains(&KeyCode::ShiftLeft) {
            dir -= Vec3::Y;
        }
        self.position += dir.normalize_or_zero() * speed * dt;
    }
}
