//! Perspective camera and first-hit rendering of the approved 3D sea.

use crate::options::{Detail, Sea, Viewpoint};
use std::f64::consts::TAU;

const FAR: f64 = 160.0;
const CELL_ASPECT: f64 = 7.8 / 17.0;
const GLYPHS: &[u8] = b"  .,:;~-=+*#";
const WAVES: [[f64; 5]; 4] = [
    [0.40, 0.58, 1.48, 0.64, 0.10],
    [0.23, -0.90, 0.85, 0.40, 2.0],
    [0.12, 1.70, 2.60, 1.02, 4.2],
    [0.045, -3.4, 4.5, 1.50, 0.90],
];

pub struct Camera {
    width: f64,
    horizon: f64,
    focal_x: f64,
    focal_y: f64,
    eye_height: f64,
}

#[derive(Clone, Copy)]
pub struct Ray {
    across: f64,
    down: f64,
    eye_height: f64,
}

impl Camera {
    pub fn new(width: u16, height: u16, viewpoint: Viewpoint, wave_bound: f64) -> Self {
        let focal_y = f64::from(height.max(1)) * 0.9;
        Self {
            width: f64::from(width),
            horizon: f64::from(height) * (5.5 / 32.0),
            focal_x: focal_y / CELL_ASPECT,
            focal_y,
            // Keep the camera above the crests even at maximum storm intensity.
            eye_height: viewpoint.eye_height()
                + (wave_bound + 0.5 - Viewpoint::Low.eye_height()).max(0.0),
        }
    }

    pub fn ray(&self, x: u16, y: u16) -> Option<Ray> {
        let down = (f64::from(y) + 0.5 - self.horizon) / self.focal_y;
        (down > 0.0).then_some(Ray {
            across: (f64::from(x) + 0.5 - self.width / 2.0) / self.focal_x,
            down,
            eye_height: self.eye_height,
        })
    }
}

#[derive(Clone, Copy, Default)]
struct Wave {
    amplitude: f64,
    kx: f64,
    kz: f64,
    phase: f64,
    filter: f64,
}

struct Surface {
    height: f64,
    slope_x: f64,
    slope_z: f64,
    curvature: f64,
}

pub struct Pixel {
    pub glyph: char,
    /// 0..7: shaded water; 8: distant, unresolved water.
    pub tone: usize,
}

pub struct Ocean {
    waves: [Wave; 4],
    count: usize,
    bound: f64,
    time: f64,
    detail: Detail,
    intensity: f64,
}

impl Ocean {
    pub fn new(
        seed: u64,
        time: f64,
        sea: Sea,
        intensity: f64,
        detail: Detail,
        camera: &Camera,
    ) -> Self {
        let (amplitude, frequency, pace) = match sea {
            Sea::Calm => (1.0, 1.0, 1.0),
            Sea::Swell => (1.35, 0.8, 0.85),
            Sea::Storm => (1.9, 1.2, 1.5),
        };
        let time = time * pace;
        let waves = std::array::from_fn(|i| {
            let [a, kx, kz, speed, phase] = WAVES[i];
            let kx = kx * frequency;
            let kz = kz * frequency;
            // Seed 42 preserves the accepted preview; others vary the crossing swells.
            let offset = if seed == 42 {
                0.0
            } else {
                unit(seed.wrapping_add(i as u64)) * TAU
            };
            Wave {
                amplitude: a * amplitude * intensity,
                kx,
                kz,
                phase: speed * time + phase + offset,
                filter: (kx * kx + kz * kz) * 1.2 / (camera.focal_x * camera.focal_x),
            }
        });
        Self {
            waves,
            count: if detail == Detail::Balanced { 3 } else { 4 },
            bound: Self::wave_bound(sea, intensity),
            time,
            detail,
            intensity,
        }
    }

    pub fn wave_bound(sea: Sea, intensity: f64) -> f64 {
        0.8 * intensity
            * match sea {
                Sea::Calm => 1.0,
                Sea::Swell => 1.35,
                Sea::Storm => 1.9,
            }
    }

    fn height(&self, x: f64, z: f64) -> f64 {
        self.waves[..self.count]
            .iter()
            .map(|wave| {
                wave.amplitude / (1.0 + wave.filter * z * z)
                    * (wave.kx * x + wave.kz * z + wave.phase).sin()
            })
            .sum()
    }

    fn surface(&self, x: f64, z: f64) -> Surface {
        let mut sample = Surface {
            height: 0.0,
            slope_x: 0.0,
            slope_z: 0.0,
            curvature: 0.0,
        };
        for wave in &self.waves[..self.count] {
            let denominator = 1.0 + wave.filter * z * z;
            let amplitude = wave.amplitude / denominator;
            let derivative = -2.0 * wave.amplitude * wave.filter * z / denominator.powi(2);
            let (sine, cosine) = (wave.kx * x + wave.kz * z + wave.phase).sin_cos();
            sample.height += amplitude * sine;
            sample.slope_x += amplitude * wave.kx * cosine;
            sample.slope_z += amplitude * wave.kz * cosine + derivative * sine;
            sample.curvature += amplitude * (wave.kx * wave.kx + wave.kz * wave.kz) * sine;
        }
        sample
    }

    fn trace(&self, ray: Ray) -> Option<f64> {
        if self.bound == 0.0 {
            let z = ray.eye_height / ray.down;
            return (z <= FAR).then_some(z);
        }
        let mut near = ((ray.eye_height - self.bound) / ray.down).max(0.04);
        let end = ((ray.eye_height + self.bound) / ray.down).min(FAR);
        while near < end {
            let next = (near + (0.075 + near * 0.014).min(0.5)).min(end);
            if ray.eye_height - ray.down * next <= self.height(ray.across * next, next) {
                let (mut low, mut high) = (near, next);
                for _ in 0..7 {
                    let middle = (low + high) * 0.5;
                    if ray.eye_height - ray.down * middle > self.height(ray.across * middle, middle)
                    {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                return Some((low + high) * 0.5);
            }
            near = next;
        }
        None
    }

    pub fn pixel(&self, ray: Ray, column: u16) -> Pixel {
        let Some(z) = self.trace(ray) else {
            return Pixel {
                glyph: if column % 3 == 0 { '-' } else { '.' },
                tone: 8,
            };
        };
        let x = ray.across * z;
        let sample = self.surface(x, z);
        let (mut sx, mut sz) = (sample.slope_x, sample.slope_z);
        if self.detail != Detail::Balanced {
            let strength = if self.detail == Detail::Rich {
                0.19
            } else {
                0.11
            } * (-z / 30.0).exp()
                * self.intensity;
            sx += strength * (x * 7.1 + z * 4.7 + self.time * 1.1).sin();
            sz += strength * (x * 3.4 - z * 8.3 - self.time * 1.3).cos();
        }
        let normal_length = sx.hypot(1.0).hypot(sz);
        let (nx, ny, nz) = (
            -sx / normal_length,
            1.0 / normal_length,
            -sz / normal_length,
        );
        let view_length = ray.across.hypot(ray.down).hypot(1.0);
        let (vx, vy, vz) = (
            -ray.across / view_length,
            ray.down / view_length,
            -1.0 / view_length,
        );
        let (lx, ly, lz) = (-0.71, 0.68, 0.18);
        let diffuse = (nx * lx + ny * ly + nz * lz).max(0.0);
        let half_length = (lx + vx).hypot(ly + vy).hypot(lz + vz);
        let glint = ((nx * (lx + vx) + ny * (ly + vy) + nz * (lz + vz)) / half_length)
            .max(0.0)
            .powi(36);
        let facing = (nx * vx + ny * vy + nz * vz).max(0.0);
        let fog = (-z / 55.0).exp();
        let light = 0.06 + diffuse * 0.65 + glint * 0.12 + (1.0 - facing).powi(3) * 0.06;
        let shade = (((light - 0.22) / 0.55) * fog + 0.16 * (1.0 - fog)).clamp(0.0, 1.0);
        let mut glyph =
            GLYPHS[((shade * GLYPHS.len() as f64) as usize).min(GLYPHS.len() - 1)] as char;
        let mut tone = ((shade * 8.0) as usize).min(7);
        if self.intensity > 0.0 {
            let rill =
                0.5 + 0.5 * (x * 4.6 + z * 9.0 + self.time * 1.8 + 0.6 * (x * 2.3 - z).sin()).sin();
            if z < 30.0 && shade < 0.75 && rill < 0.18 {
                glyph = ' ';
            } else if z < 30.0 && shade < 0.75 && rill < 0.43 {
                glyph = '.';
                tone = tone.saturating_sub(1);
            }
            let lace = (x * 4.9 + z * 3.1 + self.time * 0.65).sin()
                + 0.5 * (x * 8.2 - z * 1.7 - self.time * 0.3).sin();
            if z < 30.0 && sample.height > 0.26 && sample.curvature > 0.45 && lace > 0.45 {
                glyph = if lace > 1.1 { '=' } else { '~' };
                tone = if z < 15.0 { 6 } else { 4 };
            }
        }
        Pixel { glyph, tone }
    }
}

fn unit(mut value: u64) -> f64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((value ^ (value >> 31)) >> 11) as f64 / ((1_u64 << 53) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ocean(intensity: f64) -> (Camera, Ocean) {
        let camera = Camera::new(
            91,
            32,
            Viewpoint::Low,
            Ocean::wave_bound(Sea::Calm, intensity),
        );
        let ocean = Ocean::new(42, 5.0, Sea::Calm, intensity, Detail::Rich, &camera);
        (camera, ocean)
    }

    #[test]
    fn flat_water_has_an_analytic_intersection() {
        let (_, ocean) = ocean(0.0);
        let ray = Ray {
            across: 0.25,
            down: 0.5,
            eye_height: 2.3,
        };
        assert!((ocean.trace(ray).unwrap() - 4.6).abs() < 1e-10);
        let sample = ocean.surface(1.15, 4.6);
        assert_eq!(sample.height, 0.0);
        assert_eq!(sample.slope_x, 0.0);
        assert_eq!(sample.slope_z, 0.0);
    }

    #[test]
    fn intersections_are_on_the_nearest_visible_surface() {
        let (camera, ocean) = ocean(1.0);
        for (column, row) in [(0, 10), (12, 18), (45, 6), (45, 31), (78, 18), (90, 10)] {
            let ray = camera.ray(column, row).unwrap();
            let hit = ocean.trace(ray).unwrap();
            let clearance = |z| ray.eye_height - ray.down * z - ocean.height(ray.across * z, z);
            assert!(
                clearance(hit).abs() < 0.01,
                "intersection is off the surface"
            );
            // A much finer independent scan must not find water in front of the hit.
            for sample in 1..((hit - 0.01) / 0.01) as usize {
                assert!(
                    clearance(sample as f64 * 0.01) > 0.0,
                    "missed a nearer crest at ({column}, {row})"
                );
            }
        }
    }

    #[test]
    fn lighting_slopes_agree_with_the_rendered_height_field() {
        let (_, ocean) = ocean(1.0);
        let epsilon = 1e-5;
        for (x, z) in [(-2.0, 1.5), (0.0, 10.0), (1.25, 70.0)] {
            let sample = ocean.surface(x, z);
            let dx =
                (ocean.height(x + epsilon, z) - ocean.height(x - epsilon, z)) / (2.0 * epsilon);
            let dz =
                (ocean.height(x, z + epsilon) - ocean.height(x, z - epsilon)) / (2.0 * epsilon);
            assert!((sample.slope_x - dx).abs() < 1e-6);
            assert!((sample.slope_z - dz).abs() < 1e-6);
        }
    }

    #[test]
    fn maximum_intensity_preserves_distinct_viewpoints_above_the_water() {
        for sea in [Sea::Calm, Sea::Swell, Sea::Storm] {
            let bound = Ocean::wave_bound(sea, 3.0);
            let camera = Camera::new(91, 32, Viewpoint::Low, bound);
            let raised = Camera::new(91, 32, Viewpoint::Raised, bound);
            assert!(
                raised.eye_height > camera.eye_height,
                "raising the viewpoint should still work in a storm"
            );
            let ocean = Ocean::new(u64::MAX, 86400.0, sea, 3.0, Detail::Rich, &camera);
            assert!(camera.eye_height > bound);
            for column in [0, 45, 90] {
                let ray = camera.ray(column, 31).unwrap();
                let hit = ocean.trace(ray).unwrap();
                assert!(hit.is_finite() && hit > 0.0);
                let pixel = ocean.pixel(ray, column);
                assert!(pixel.glyph.is_ascii() && pixel.tone <= 8);
            }
        }
    }
}
