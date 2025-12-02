use std::sync::Arc;

use crate::hittable::{Aabb, HitRecord, Hittable};
use crate::ray::Ray;

#[derive(Clone)]
pub enum BvhNode {
    Leaf {
        objects: Vec<Arc<dyn Hittable>>,
        bbox: Aabb,
    },
    Branch {
        left: Arc<BvhNode>,
        right: Arc<BvhNode>,
        bbox: Aabb,
    },
}

impl BvhNode {
    fn bbox(&self) -> Aabb {
        match self {
            BvhNode::Leaf { bbox, .. } | BvhNode::Branch { bbox, .. } => *bbox,
        }
    }

    pub fn build(mut objects: Vec<Arc<dyn Hittable>>) -> Self {
        let bbox = Aabb::surrounding_box(&objects.iter().map(|o| o.bounding_box()).collect::<Vec<_>>());

        if objects.len() <= 2 {
            return Self::Leaf { objects, bbox };
        }

        let axis = bbox.max.x - bbox.min.x > bbox.max.y - bbox.min.y
            && bbox.max.x - bbox.min.x > bbox.max.z - bbox.min.z;
        let axis = if axis {
            0
        } else if bbox.max.y - bbox.min.y > bbox.max.z - bbox.min.z {
            1
        } else {
            2
        };

        objects.sort_by(|a, b| {
            let ca = a.bounding_box().min;
            let cb = b.bounding_box().min;
            let va = match axis {
                0 => ca.x,
                1 => ca.y,
                _ => ca.z,
            };
            let vb = match axis {
                0 => cb.x,
                1 => cb.y,
                _ => cb.z,
            };
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mid = objects.len() / 2;
        let right = objects.split_off(mid);
        let left = BvhNode::build(objects);
        let right = BvhNode::build(right);
        let bbox = Aabb::surrounding_box(&[left.bbox(), right.bbox()]);

        Self::Branch {
            left: Arc::new(left),
            right: Arc::new(right),
            bbox,
        }
    }

}

impl Hittable for BvhNode {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        if !self.bbox().hit(ray, t_min, t_max) {
            return None;
        }

        match self {
            BvhNode::Leaf { objects, .. } => {
                let mut closest: Option<HitRecord> = None;
                let mut closest_t = t_max;
                for object in objects {
                    if let Some(hit) = object.hit(ray, t_min, closest_t) {
                        closest_t = hit.t;
                        closest = Some(hit);
                    }
                }
                closest
            }
            BvhNode::Branch { left, right, .. } => {
                let hit_left = left.hit(ray, t_min, t_max);
                let t_far = hit_left.as_ref().map(|h| h.t).unwrap_or(t_max);
                let hit_right = right.hit(ray, t_min, t_far);
                match (hit_left, hit_right) {
                    (Some(l), Some(r)) => {
                        if l.t <= r.t {
                            Some(l)
                        } else {
                            Some(r)
                        }
                    }
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
        }
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            BvhNode::Leaf { bbox, .. } | BvhNode::Branch { bbox, .. } => *bbox,
        }
    }
}
