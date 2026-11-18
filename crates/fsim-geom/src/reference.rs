//! Exact reference oracle for `fsim-geom` polygon containment, overlap, and clearance checks.

use crate::{AabbMm, PointMm, PolygonWithHolesMm};

/// Reference point-in-polygon check using exact 64-bit integer winding/ray-casting predicates.
pub fn reference_contains_point(poly: &PolygonWithHolesMm, pt: PointMm) -> bool {
    if !point_in_ring(&poly.outer, pt) {
        return false;
    }
    for hole in &poly.holes {
        if point_in_ring_strict_interior(hole, pt) {
            return false;
        }
    }
    true
}

/// Reference AABB containment inside a polygon with holes by checking all 4 corners and all polygon edges.
pub fn reference_contains_aabb(poly: &PolygonWithHolesMm, rect: AabbMm) -> bool {
    let corners = rect.corners();
    for c in corners {
        if !reference_contains_point(poly, c) {
            return false;
        }
    }
    let rect_edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    for ring in std::iter::once(&poly.outer).chain(poly.holes.iter()) {
        for i in 0..ring.len() {
            let p1 = ring[i];
            let p2 = ring[(i + 1) % ring.len()];
            for &(r1, r2) in &rect_edges {
                if segments_properly_intersect(p1, p2, r1, r2) {
                    return false;
                }
            }
        }
        for &v in ring {
            if rect.contains_strict_interior(v) {
                return false;
            }
        }
    }
    true
}

pub fn point_on_segment(a: PointMm, b: PointMm, p: PointMm) -> bool {
    let cross = (b.x as i64 - a.x as i64) * (p.y as i64 - a.y as i64)
        - (b.y as i64 - a.y as i64) * (p.x as i64 - a.x as i64);
    if cross != 0 {
        return false;
    }
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y
}

pub fn point_in_ring(ring: &[PointMm], pt: PointMm) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if point_on_segment(a, b, pt) {
            return true;
        }
        let cond = (a.y > pt.y) != (b.y > pt.y);
        if cond {
            let dy = (b.y as i128) - (a.y as i128);
            let dx = (b.x as i128) - (a.x as i128);
            let lhs = ((pt.x as i128) - (a.x as i128)) * dy;
            let rhs = ((pt.y as i128) - (a.y as i128)) * dx;
            let intersects = if dy > 0 { lhs < rhs } else { lhs > rhs };
            if intersects {
                inside = !inside;
            }
        }
    }
    inside
}

pub fn point_in_ring_strict_interior(ring: &[PointMm], pt: PointMm) -> bool {
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if point_on_segment(a, b, pt) {
            return false;
        }
    }
    point_in_ring(ring, pt)
}

fn orient2d(a: PointMm, b: PointMm, c: PointMm) -> i128 {
    ((b.x as i128) - (a.x as i128)) * ((c.y as i128) - (a.y as i128))
        - ((b.y as i128) - (a.y as i128)) * ((c.x as i128) - (a.x as i128))
}

pub fn segments_properly_intersect(a: PointMm, b: PointMm, c: PointMm, d: PointMm) -> bool {
    let o1 = orient2d(a, b, c).signum();
    let o2 = orient2d(a, b, d).signum();
    let o3 = orient2d(c, d, a).signum();
    let o4 = orient2d(c, d, b).signum();
    o1 != 0 && o2 != 0 && o3 != 0 && o4 != 0 && (o1 != o2) && (o3 != o4)
}
