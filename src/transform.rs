#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct Transform {
    tx: nalgebra::Matrix4<f32>,

    pub hue: f32,
    pub sat: f32,
    pub brightness: f32,
    pub alpha: f32,
}

impl Transform {
    pub fn translation(x: f32, y: f32, z: f32) -> Transform {
        Self {
            tx: nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(x, y, z)),
            ..Default::default()
        }
    }

    pub fn rotate_x(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0., 0.5, 0.5))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::x_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0., -0.5, -0.5));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn rotate_y(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0., 0.5))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::y_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, 0., -0.5));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn rotate_z(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0.5, 0.))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::z_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, -0.5, 0.));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0.5, 0.5))
            * nalgebra::Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(x, y, z))
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, -0.5, -0.5));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn hsv(hue: f32, sat: f32, brightness: f32) -> Transform {
        Self {
            hue,
            sat,
            brightness,
            ..Default::default()
        }
    }
}

impl std::ops::MulAssign for Transform {
    fn mul_assign(&mut self, rhs: Self) {
        self.tx *= rhs.tx;

        self.hue += rhs.hue;
        self.sat *= rhs.sat;
        self.brightness *= rhs.brightness;
        self.alpha *= rhs.alpha;
    }
}

impl std::ops::Mul for Transform {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self::Output {
        self *= rhs;
        self
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            tx: nalgebra::Matrix4::identity(),
            hue: 0.0,
            sat: 1.0,
            brightness: 1.0,
            alpha: 1.0,
        }
    }
}

impl From<Transform> for mint::ColumnMatrix4<f32> {
    fn from(t: Transform) -> Self {
        t.tx.into()
    }
}

impl From<&Transform> for mint::ColumnMatrix4<f32> {
    fn from(t: &Transform) -> Self {
        t.tx.into()
    }
}

impl approx::AbsDiffEq for Transform {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.tx.abs_diff_eq(&other.tx, epsilon)
            && self.hue.abs_diff_eq(&other.hue, epsilon)
            && self.sat.abs_diff_eq(&other.sat, epsilon)
            && self.brightness.abs_diff_eq(&other.brightness, epsilon)
            && self.alpha.abs_diff_eq(&other.alpha, epsilon)
    }
}

impl approx::RelativeEq for Transform {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.tx.relative_eq(&other.tx, epsilon, max_relative)
            && self.hue.relative_eq(&other.hue, epsilon, max_relative)
            && self.sat.relative_eq(&other.sat, epsilon, max_relative)
            && self
                .brightness
                .relative_eq(&other.brightness, epsilon, max_relative)
            && self.alpha.relative_eq(&other.alpha, epsilon, max_relative)
    }
}

impl approx::UlpsEq for Transform {
    fn default_max_ulps() -> u32 {
        f32::default_max_ulps()
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        self.tx.ulps_eq(&other.tx, epsilon, max_ulps)
            && self.hue.ulps_eq(&other.hue, epsilon, max_ulps)
            && self.sat.ulps_eq(&other.sat, epsilon, max_ulps)
            && self
                .brightness
                .ulps_eq(&other.brightness, epsilon, max_ulps)
            && self.alpha.ulps_eq(&other.alpha, epsilon, max_ulps)
    }
}
