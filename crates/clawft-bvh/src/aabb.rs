//! Axis-aligned bounding boxes and basic geometry.

use serde::{Deserialize, Serialize};

/// 3D point / vector (f32).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    /// X
    pub x: f32,
    /// Y
    pub y: f32,
    /// Z
    pub z: f32,
}

impl Vec3 {
    /// Origin.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Construct.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Component-wise min.
    pub fn min(self, o: Self) -> Self {
        Self {
            x: self.x.min(o.x),
            y: self.y.min(o.y),
            z: self.z.min(o.z),
        }
    }

    /// Component-wise max.
    pub fn max(self, o: Self) -> Self {
        Self {
            x: self.x.max(o.x),
            y: self.y.max(o.y),
            z: self.z.max(o.z),
        }
    }

    /// Squared length.
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Length.
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Dot product.
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// Subtraction.
    pub fn sub(self, o: Self) -> Self {
        Self {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }

    /// Addition.
    pub fn add(self, o: Self) -> Self {
        Self {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }

    /// Scale.
    pub fn scale(self, s: f32) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

/// Axis-aligned bounding box (canonical broad-phase volume, ADR-056 §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb {
    /// Empty (inverted) AABB that expands correctly under `union`.
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    /// From min/max corners (caller ensures min ≤ max per axis).
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Unit cube centered at origin.
    pub fn unit() -> Self {
        Self {
            min: Vec3::new(-0.5, -0.5, -0.5),
            max: Vec3::new(0.5, 0.5, 0.5),
        }
    }

    /// Center.
    pub fn center(self) -> Vec3 {
        Vec3::new(
            0.5 * (self.min.x + self.max.x),
            0.5 * (self.min.y + self.max.y),
            0.5 * (self.min.z + self.max.z),
        )
    }

    /// Half extents.
    pub fn extents(self) -> Vec3 {
        Vec3::new(
            0.5 * (self.max.x - self.min.x),
            0.5 * (self.max.y - self.min.y),
            0.5 * (self.max.z - self.min.z),
        )
    }

    /// Surface area (for SAH later; used lightly in build).
    pub fn surface_area(self) -> f32 {
        let e = self.max.sub(self.min);
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }

    /// Union of two AABBs.
    pub fn union(self, o: Self) -> Self {
        Self {
            min: self.min.min(o.min),
            max: self.max.max(o.max),
        }
    }

    /// Point inside (inclusive).
    pub fn contains_point(self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Overlap with another AABB.
    pub fn intersects_aabb(self, o: Self) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
            && self.min.z <= o.max.z
            && self.max.z >= o.min.z
    }

    /// Sphere overlap (broad-phase).
    pub fn intersects_sphere(self, center: Vec3, radius: f32) -> bool {
        let cx = center.x.clamp(self.min.x, self.max.x);
        let cy = center.y.clamp(self.min.y, self.max.y);
        let cz = center.z.clamp(self.min.z, self.max.z);
        let d = Vec3::new(center.x - cx, center.y - cy, center.z - cz);
        d.length_sq() <= radius * radius
    }

    /// Ray vs AABB (slab method). Returns `t_enter` if hit with `t_enter <= max_t`.
    pub fn intersects_ray(self, ray: Ray, max_t: f32) -> Option<f32> {
        let mut tmin = 0.0_f32;
        let mut tmax = max_t;
        for (origin, dir, bmin, bmax) in [
            (ray.origin.x, ray.dir.x, self.min.x, self.max.x),
            (ray.origin.y, ray.dir.y, self.min.y, self.max.y),
            (ray.origin.z, ray.dir.z, self.min.z, self.max.z),
        ] {
            if dir.abs() < 1e-12 {
                if origin < bmin || origin > bmax {
                    return None;
                }
            } else {
                let inv = 1.0 / dir;
                let mut t0 = (bmin - origin) * inv;
                let mut t1 = (bmax - origin) * inv;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                tmin = tmin.max(t0);
                tmax = tmax.min(t1);
                if tmin > tmax {
                    return None;
                }
            }
        }
        Some(tmin)
    }
}

/// Ray for broad-phase casts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ray {
    /// Origin.
    pub origin: Vec3,
    /// Direction (need not be unit; slab handles non-unit).
    pub dir: Vec3,
}

impl Ray {
    /// Construct.
    pub const fn new(origin: Vec3, dir: Vec3) -> Self {
        Self { origin, dir }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_contains() {
        let a = Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::from_min_max(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 2.0, 2.0));
        let u = a.union(b);
        assert!(u.contains_point(Vec3::new(1.5, 1.5, 1.5)));
        assert!(a.intersects_aabb(b));
    }

    #[test]
    fn sphere_and_ray() {
        let a = Aabb::unit();
        assert!(a.intersects_sphere(Vec3::ZERO, 0.1));
        assert!(!a.intersects_sphere(Vec3::new(10.0, 0.0, 0.0), 0.1));
        let r = Ray::new(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(a.intersects_ray(r, 100.0).is_some());
    }
}
