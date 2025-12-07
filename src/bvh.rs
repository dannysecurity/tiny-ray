use std::sync::Arc;

use crate::hittable::{Aabb, HitRecord, Hittable};
use crate::ray::Ray;
use crate::vec3::Point3;

fn axis_component(point: Point3, axis: usize) -> f64 {
    match axis {
        0 => point.x,
        1 => point.y,
        _ => point.z,
    }
}

fn closest_hit(a: Option<HitRecord>, b: Option<HitRecord>) -> Option<HitRecord> {
    match (a, b) {
        (Some(left), Some(right)) => {
            if left.t <= right.t {
                Some(left)
            } else {
                Some(right)
            }
        }
        (Some(hit), None) | (None, Some(hit)) => Some(hit),
        (None, None) => None,
    }
}

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

        let axis = bbox.longest_axis();

        objects.sort_by(|a, b| {
            let va = axis_component(a.bounding_box().min, axis);
            let vb = axis_component(b.bounding_box().min, axis);
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
                closest_hit(hit_left, hit_right)
            }
        }
    }

    fn bounding_box(&self) -> Aabb {
        match self {
            BvhNode::Leaf { bbox, .. } | BvhNode::Branch { bbox, .. } => *bbox,
        }
    }
}
