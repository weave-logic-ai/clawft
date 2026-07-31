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

    /// Cross product.
    pub fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
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

    /// Unit vector, or `ZERO` if length is near zero.
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-12 {
            Self::ZERO
        } else {
            self.scale(1.0 / len)
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, o: Self) -> Self {
        Self {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, o: Self) -> Self {
        Self {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
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
        let e = self.max - self.min;
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

    /// Closest point on (or inside) this AABB to `p`.
    pub fn closest_point(self, p: Vec3) -> Vec3 {
        Vec3::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
            p.z.clamp(self.min.z, self.max.z),
        )
    }

    /// Squared distance from `p` to this AABB (0 if `p` is inside).
    pub fn distance_sq_point(self, p: Vec3) -> f32 {
        let c = self.closest_point(p);
        (p - c).length_sq()
    }

    /// True if this AABB is not completely outside any frustum plane.
    ///
    /// Planes use outward normals: a point is outside when
    /// `normal · p + d > 0`. An AABB is culled when its p-vertex
    /// (corner most opposite the normal) is outside.
    pub fn intersects_frustum(self, frustum: &Frustum) -> bool {
        for plane in &frustum.planes {
            // p-vertex: corner furthest in the *inward* direction
            // (most negative signed distance) for a conservative outside test.
            let px = if plane.normal.x >= 0.0 {
                self.min.x
            } else {
                self.max.x
            };
            let py = if plane.normal.y >= 0.0 {
                self.min.y
            } else {
                self.max.y
            };
            let pz = if plane.normal.z >= 0.0 {
                self.min.z
            } else {
                self.max.z
            };
            if plane.signed_distance(Vec3::new(px, py, pz)) > 0.0 {
                return false;
            }
        }
        true
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

/// Half-space plane: `normal · p + d = 0`. Outward normal convention
/// (positive side is outside the frustum volume).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Plane {
    /// Unit outward normal.
    pub normal: Vec3,
    /// Plane offset so that `normal · p + d = 0` on the plane.
    pub d: f32,
}

impl Plane {
    /// From unit normal and a point on the plane.
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            d: -n.dot(point),
        }
    }

    /// Signed distance: positive ⇒ outside (for outward normals).
    pub fn signed_distance(self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }
}

/// View frustum as six outward half-spaces (near, far, left, right, bottom, top).
///
/// Broad-phase only: leaves whose AABB intersects the frustum volume are
/// returned; narrow-phase may refine further.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frustum {
    /// Six clipping planes (order is not significant for intersection).
    pub planes: [Plane; 6],
}

/// Plane through three points; normal = `(b-a) × (c-a)` (may point either way).
fn plane_from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Plane {
    let n = (b - a).cross(c - a).normalize();
    Plane::from_point_normal(a, n)
}

impl Frustum {
    /// Construct from six planes.
    pub const fn from_planes(planes: [Plane; 6]) -> Self {
        Self { planes }
    }

    /// Perspective frustum from eye pose + vertical FOV.
    ///
    /// `forward` and `up` need not be orthonormal; they are re-orthonormalized.
    /// `vfov_rad` is the full vertical field of view in radians; `aspect` is
    /// width/height; `near` / `far` are positive distances along forward.
    ///
    /// Plane normals are forced outward using a known interior test point.
    pub fn perspective(
        eye: Vec3,
        forward: Vec3,
        up: Vec3,
        vfov_rad: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let f = forward.normalize();
        // right-handed: right = forward × up
        let r = f.cross(up).normalize();
        let u = r.cross(f).normalize();

        let half_v = (vfov_rad * 0.5).tan();
        let half_h = half_v * aspect.max(1e-6);

        let near_center = eye + f.scale(near.max(1e-6));
        let far_center = eye + f.scale(far.max(near + 1e-3));

        let near_h = u.scale(half_v * near.max(1e-6));
        let near_w = r.scale(half_h * near.max(1e-6));
        // far extents used for side-plane points
        let far_h = u.scale(half_v * far.max(near + 1e-3));
        let far_w = r.scale(half_h * far.max(near + 1e-3));

        let n_tl = near_center + near_h - near_w;
        let n_tr = near_center + near_h + near_w;
        let n_bl = near_center - near_h - near_w;
        let n_br = near_center - near_h + near_w;
        let f_tl = far_center + far_h - far_w;
        let f_tr = far_center + far_h + far_w;
        let f_bl = far_center - far_h - far_w;
        let f_br = far_center - far_h + far_w;

        // Side planes from three corners (winding arbitrary; flipped below).
        let mut planes = [
            plane_from_triangle(n_tl, n_tr, n_br), // near
            plane_from_triangle(f_tl, f_bl, f_br), // far
            plane_from_triangle(n_tl, n_bl, f_bl), // left
            plane_from_triangle(n_tr, f_tr, n_br), // right
            plane_from_triangle(n_bl, n_br, f_br), // bottom
            plane_from_triangle(n_tl, f_tl, n_tr), // top
        ];

        // Interior point along the view axis (mid near/far).
        let inside = eye + f.scale(0.5 * (near + far));
        for plane in &mut planes {
            if plane.signed_distance(inside) > 0.0 {
                // Flip so interior is on the negative side (outward normal).
                plane.normal = -plane.normal;
                plane.d = -plane.d;
            }
        }

        Self { planes }
    }

    /// Axis-aligned orthographic "box" frustum (useful for tests / region culls).
    pub fn from_aabb(bb: Aabb) -> Self {
        Self {
            planes: [
                Plane {
                    normal: Vec3::new(-1.0, 0.0, 0.0),
                    d: bb.min.x,
                }, // left  (x >= min.x)  → outward -X, d = min.x
                Plane {
                    normal: Vec3::new(1.0, 0.0, 0.0),
                    d: -bb.max.x,
                }, // right
                Plane {
                    normal: Vec3::new(0.0, -1.0, 0.0),
                    d: bb.min.y,
                }, // bottom
                Plane {
                    normal: Vec3::new(0.0, 1.0, 0.0),
                    d: -bb.max.y,
                }, // top
                Plane {
                    normal: Vec3::new(0.0, 0.0, -1.0),
                    d: bb.min.z,
                }, // near-ish (-Z outward)
                Plane {
                    normal: Vec3::new(0.0, 0.0, 1.0),
                    d: -bb.max.z,
                }, // far
            ],
        }
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

    #[test]
    fn distance_sq_inside_is_zero() {
        let a = Aabb::unit();
        assert_eq!(a.distance_sq_point(Vec3::ZERO), 0.0);
        let far = a.distance_sq_point(Vec3::new(10.0, 0.0, 0.0));
        assert!((far - 9.5_f32 * 9.5).abs() < 1e-4);
    }

    #[test]
    fn frustum_from_aabb_contains_unit() {
        let region = Aabb::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let f = Frustum::from_aabb(region);
        assert!(Aabb::unit().intersects_frustum(&f));
        let outside = Aabb::from_min_max(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));
        assert!(!outside.intersects_frustum(&f));
    }

    #[test]
    fn perspective_frustum_culls_behind() {
        let f = Frustum::perspective(
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            std::f32::consts::FRAC_PI_2,
            1.0,
            0.5,
            10.0,
        );
        let in_front = Aabb::from_min_max(Vec3::new(-0.1, -0.1, 1.0), Vec3::new(0.1, 0.1, 1.2));
        let behind = Aabb::from_min_max(Vec3::new(-0.1, -0.1, -2.0), Vec3::new(0.1, 0.1, -1.0));
        assert!(in_front.intersects_frustum(&f));
        assert!(!behind.intersects_frustum(&f));
    }
}
