use std::f32::consts::PI;

use nalgebra::{Matrix4, SimdComplexField, vector, Vector3, Vector4};
use winit::{dpi::PhysicalPosition, event::*};

const UP: Vector3<f32> = Vector3::<f32>::new(0.0, 0.0, 1.0);

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub target: Vector3<f32>,
    pub eye: nalgebra::Point3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub z_near: f32,
    pub z_far: f32,
}

#[allow(unused)]
impl Camera {
    pub fn calc_target(&self, yaw: f32, pitch: f32) -> Vector3<f32> {
        let (sin, cos) = yaw.to_radians().simd_sin_cos();
        let target = Vector3::new(cos, sin * (1.0 - UP.y), sin * (1.0 - UP.z));
        let (sin, cos) = pitch.to_radians().simd_sin_cos();
        let target = (target * cos) + (UP * sin);
        target
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let proj = Matrix4::new_perspective(self.aspect, self.fovy, self.z_near, self.z_far);
        let view = Matrix4::<f32>::look_at_rh(&self.eye, &(self.eye + self.target), &UP);
        // v′=P⋅V⋅M⋅v
        proj * view
    }
    pub fn new(eye: nalgebra::Point3<f32>) -> Self {
        Self {
            target: vector![1.0, 0.0, 0.0],
            eye,
            aspect: 16.0 / 9.0,
            fovy: 80.0_f32.to_radians(),
            z_near: 0.0001,
            z_far: 1000.0,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: Vector4<f32>,
    pub view_proj: Matrix4<f32>,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: Vector4::zeros(),
            view_proj: Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.eye.to_homogeneous();
        self.view_proj = camera.build_view_projection_matrix();
    }
}
#[allow(unused)]
pub struct CameraController {
    // Keyboard input
    is_up_pressed: bool,
    is_modifier_shift_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_rotate_left_pressed: bool,
    is_rotate_right_pressed: bool,

    // Mouse input
    pub is_mouse_right_pressed: bool,
    pub is_mouse_right_tracked: bool,

    // Mouse position
    // The initial or previous position, used for calculating direction/speed of movement
    pub mouse_initial_position: PhysicalPosition<f32>,
    // The difference between initial + current position
    mouse_diff_position: PhysicalPosition<f32>,

    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

#[allow(unused)]
impl CameraController {
    pub fn new() -> Self {
        Self {
            is_up_pressed: false,
            is_modifier_shift_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_rotate_left_pressed: false,
            is_rotate_right_pressed: false,
            is_mouse_right_pressed: false,
            is_mouse_right_tracked: false,
            mouse_initial_position: PhysicalPosition { x: 0.0, y: 0.0 },
            mouse_diff_position: PhysicalPosition { x: 0.0, y: 0.0 },
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    /// Handle keyboard input for camera (like moving camera with WASD keys)
    pub fn process_events(
        &mut self,
        state: &ElementState,
        &virtual_keycode: &VirtualKeyCode,
    ) -> bool {
        let is_pressed = *state == ElementState::Pressed;
        match virtual_keycode {
            VirtualKeyCode::Space => {
                self.is_up_pressed = is_pressed;
                true
            }
            VirtualKeyCode::LShift => {
                self.is_modifier_shift_pressed = is_pressed;
                true
            }
            VirtualKeyCode::Q => {
                self.is_rotate_left_pressed = is_pressed;
                true
            }
            VirtualKeyCode::E => {
                self.is_rotate_right_pressed = is_pressed;
                true
            }
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.is_forward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.is_left_pressed = is_pressed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.is_backward_pressed = is_pressed;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    /// Handle mouse input for camera (like moving camera based on mouse position)
    pub fn process_mouse_moved(
        &mut self,
        position: &PhysicalPosition<f64>,
        screen_size: &winit::dpi::PhysicalSize<u32>,
    ) {
        // println!(
        //     "Mouse position X: {} - Y : {}",
        //     &position.x / screen_size.width as f64,
        //     &position.y / screen_size.height as f64
        // );

        let current_x = &position.x / screen_size.width as f64;
        let current_y = &position.y / screen_size.height as f64;

        // Not tracking? Set initial position
        if self.is_mouse_right_pressed && !self.is_mouse_right_tracked {
            self.mouse_initial_position = PhysicalPosition {
                x: current_x as f32,
                y: current_y as f32,
            };
            self.is_mouse_right_tracked = true;
        }

        // Tracking? Set current position
        if self.is_mouse_right_pressed && self.is_mouse_right_tracked {
            self.mouse_diff_position = PhysicalPosition {
                x: current_x as f32 - self.mouse_initial_position.x,
                y: current_y as f32 - self.mouse_initial_position.y,
            };
        }

        // Not pressing anymore? Stop tracking.
        if !self.is_mouse_right_pressed && self.is_mouse_right_tracked {
            self.is_mouse_right_tracked = false;
        }
    }

    pub fn process_mouse_input(
        &mut self,
        device_id: &DeviceId,
        state: &ElementState,
        button: &MouseButton,
    ) {
        match button {
            MouseButton::Right => {
                self.is_mouse_right_pressed = *state == ElementState::Pressed;
            }
            _ => {}
        }
    }

    /// Update camera angles and return the pos delta unit
    pub fn update_direction(&mut self, camera: &mut Camera) -> Vector3<f32> {
        let plane_view = camera.target.xy().normalize();
        self.yaw = plane_view.x.acos() * 180.0 / PI;
        if plane_view.y < 0.0 {
            self.yaw = 360.0 - self.yaw;
        }

        let (sin, cos) = self.yaw.to_radians().simd_sin_cos();
        let forward = Vector3::<f32>::new(cos, sin * (1.0 - UP.y), sin * (1.0 - UP.z));

        let mut eye_delta = Vector3::zeros();
        if self.is_forward_pressed {
            eye_delta += forward;
        }
        if self.is_backward_pressed {
            eye_delta -= forward;
        }

        let right = UP.cross(&forward);


        if self.is_right_pressed {
            // go right
            eye_delta -= right;
        }

        if self.is_left_pressed {
            eye_delta += right;
        }

        if self.is_modifier_shift_pressed {
            eye_delta -= UP;
        }
        if self.is_up_pressed {
            // go up
            eye_delta += UP;
        }


        // Mouse input
        if self.is_mouse_right_tracked {
            if self.mouse_diff_position.x.is_finite() && self.mouse_diff_position.y.is_finite() {
                self.yaw -= self.mouse_diff_position.x * 180.0;
                self.yaw %= 360.0;
                self.pitch -= self.mouse_diff_position.y * 180.0;
                self.pitch = self.pitch.clamp(-90.0 + 1.0, 90.0 - 1.0);
            }
            self.mouse_diff_position = Default::default();
        }
        camera.target = camera.calc_target(self.yaw, self.pitch);
        eye_delta
    }
}

#[cfg(test)]
mod test {
    use nalgebra::{point, vector};

    use crate::engine::render::camera::{Camera, UP};

    #[test]
    fn test_coord() {
        assert_eq!(UP, vector![0.0, 0.0, 1.0]);
        let camera = Camera::new(point![0.0, 0.0, 0.0]);
        assert_eq!(camera.calc_target(0.0, 0.0), vector![1.0, 0.0, 0.0]);
    }
}