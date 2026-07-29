//! Region extraction — which closed areas a sketch encloses.
//!
//! This is what makes a sketch clickable. Curves are flattened into segments,
//! split wherever they cross, and stitched into a planar graph; walking that
//! graph's half-edges yields exactly one loop per enclosed area. A loop that
//! sits inside another becomes its hole, so a circle drawn inside a rectangle
//! gives two regions — the disc, and the rectangle with a hole in it — which
//! is what a user coming from Fusion expects.
//!
//! Crossings matter: drawing one line across a rectangle splits it into two
//! regions without the user having to trim anything.

use std::collections::HashMap;

use glam::Vec2;
use uuid::Uuid;

use super::{Sketch, SketchEntity};

/// Segments used to flatten a full circle; arcs take a proportional share
const CIRCLE_SEGMENTS: usize = 64;
/// Segments per spline span
const SPLINE_SEGMENTS: usize = 16;
/// Points nearer than this weld into one vertex (sketch units are metres)
const WELD: f32 = 1e-6;
/// Loops thinner than this are numerical debris, not regions
const MIN_AREA: f32 = 1e-10;
/// |sin| between two segments below which they count as parallel
const PARALLEL_SIN: f32 = 1e-6;
/// Slack on the [0, 1] parameter range so touching endpoints still register
const PARAM_SLACK: f32 = 1e-5;

/// A closed area enclosed by a sketch's curves
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Derived from the curves that bound it, so it survives an edit elsewhere
    /// in the sketch and a feature can keep pointing at the same region
    pub id: Uuid,
    /// Outer boundary, counter-clockwise
    pub outer: Vec<Vec2>,
    /// Islands cut out of `outer`, each clockwise
    pub holes: Vec<Vec<Vec2>>,
    /// Enclosed area with the holes already subtracted
    pub area: f32,
    /// A point that really is inside the region — for labels and hit-testing
    pub centroid: Vec2,
    /// Sketch entities bounding the region (outer boundary only), sorted
    pub edges: Vec<Uuid>,
}

impl Profile {
    /// Every boundary as a point loop, outer first — what a kernel needs to
    /// build a face
    pub fn loops(&self) -> impl Iterator<Item = &Vec<Vec2>> {
        std::iter::once(&self.outer).chain(self.holes.iter())
    }
}

/// Find every closed region in a sketch, largest first.
///
/// Construction geometry is ignored, and so is anything that fails to close.
pub fn extract(sketch: &Sketch) -> Vec<Profile> {
    let flats = flatten(sketch);
    if flats.is_empty() {
        return Vec::new();
    }
    let segments = split_crossings(&flats);
    let graph = Graph::build(&segments);
    let loops = graph.walk_faces();
    assemble(loops)
}

// ---- flattening ---------------------------------------------------------

/// One curve reduced to a polyline
struct Flat {
    entity: Uuid,
    pts: Vec<Vec2>,
    closed: bool,
}

fn flatten(sketch: &Sketch) -> Vec<Flat> {
    let mut out = Vec::new();
    for entity in sketch.entities_iter() {
        if entity.is_point() || sketch.is_construction(entity.id()) {
            continue;
        }
        let Some(flat) = flatten_one(sketch, entity) else {
            continue;
        };
        if flat.pts.len() >= 2 {
            out.push(flat);
        }
    }
    // HashMap iteration order is arbitrary; regions must not depend on it
    out.sort_by_key(|f| f.entity);
    out
}

fn flatten_one(sketch: &Sketch, entity: &SketchEntity) -> Option<Flat> {
    let point = |id: Uuid| sketch.get_entity(id).and_then(|e| e.position());
    let id = entity.id();
    match entity {
        SketchEntity::Point { .. } => None,

        SketchEntity::Line { start, end, .. } => Some(Flat {
            entity: id,
            pts: vec![point(*start)?, point(*end)?],
            closed: false,
        }),

        SketchEntity::Circle { center, radius, .. } => {
            let c = point(*center)?;
            Some(Flat {
                entity: id,
                pts: sample_arc(c, *radius, 0.0, std::f32::consts::TAU),
                closed: true,
            })
        }

        SketchEntity::Arc {
            center,
            start,
            end,
            radius,
            ..
        } => {
            let c = point(*center)?;
            let a = point(*start)?;
            let b = point(*end)?;
            let from = (a - c).to_angle();
            let mut sweep = (b - c).to_angle() - from;
            // The UI draws arcs counter-clockwise from start to end
            while sweep <= 1e-6 {
                sweep += std::f32::consts::TAU;
            }
            Some(Flat {
                entity: id,
                pts: sample_arc(c, *radius, from, sweep),
                closed: false,
            })
        }

        SketchEntity::Ellipse {
            center,
            major_radius,
            minor_radius,
            rotation,
            ..
        } => {
            let c = point(*center)?;
            let (sin, cos) = rotation.sin_cos();
            let pts = (0..CIRCLE_SEGMENTS)
                .map(|i| {
                    let t = (i as f32 / CIRCLE_SEGMENTS as f32) * std::f32::consts::TAU;
                    let (x, y) = (major_radius * t.cos(), minor_radius * t.sin());
                    c + Vec2::new(x * cos - y * sin, x * sin + y * cos)
                })
                .collect();
            Some(Flat {
                entity: id,
                pts,
                closed: true,
            })
        }

        SketchEntity::Spline {
            control_points,
            closed,
            ..
        } => {
            let knots: Vec<Vec2> = control_points.iter().filter_map(|p| point(*p)).collect();
            if knots.len() < 2 {
                return None;
            }
            Some(Flat {
                entity: id,
                pts: catmull_rom(&knots, *closed),
                closed: *closed,
            })
        }
    }
}

/// Points along an arc, `sweep` radians counter-clockwise from `from`
fn sample_arc(center: Vec2, radius: f32, from: f32, sweep: f32) -> Vec<Vec2> {
    let steps = ((CIRCLE_SEGMENTS as f32 * sweep.abs()) / std::f32::consts::TAU).ceil() as usize;
    let steps = steps.max(4);
    // A closed curve must not repeat its first point as its last
    let full = (sweep - std::f32::consts::TAU).abs() < 1e-5;
    let count = if full { steps } else { steps + 1 };
    (0..count)
        .map(|i| {
            let a = from + sweep * (i as f32 / steps as f32);
            center + Vec2::new(radius * a.cos(), radius * a.sin())
        })
        .collect()
}

/// Centripetal-ish Catmull-Rom through the control points
fn catmull_rom(knots: &[Vec2], closed: bool) -> Vec<Vec2> {
    let n = knots.len();
    if n == 2 && !closed {
        return knots.to_vec();
    }
    let at = |i: isize| -> Vec2 {
        if closed {
            knots[i.rem_euclid(n as isize) as usize]
        } else {
            knots[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(spans * SPLINE_SEGMENTS + 1);
    for span in 0..spans {
        let (p0, p1, p2, p3) = (
            at(span as isize - 1),
            at(span as isize),
            at(span as isize + 1),
            at(span as isize + 2),
        );
        for step in 0..SPLINE_SEGMENTS {
            let t = step as f32 / SPLINE_SEGMENTS as f32;
            let (t2, t3) = (t * t, t * t * t);
            out.push(
                0.5 * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3),
            );
        }
    }
    if !closed {
        out.push(knots[n - 1]);
    }
    out
}

// ---- splitting at crossings --------------------------------------------

#[derive(Clone, Copy)]
struct Segment {
    a: Vec2,
    b: Vec2,
    entity: Uuid,
}

/// Cut every segment at every point another segment touches it, so the graph
/// that follows has a vertex wherever two curves meet
fn split_crossings(flats: &[Flat]) -> Vec<Segment> {
    let mut raw: Vec<Segment> = Vec::new();
    for flat in flats {
        let n = flat.pts.len();
        let last = if flat.closed { n } else { n - 1 };
        for i in 0..last {
            let a = flat.pts[i];
            let b = flat.pts[(i + 1) % n];
            if a.distance_squared(b) > WELD * WELD {
                raw.push(Segment {
                    a,
                    b,
                    entity: flat.entity,
                });
            }
        }
    }

    let mut out = Vec::with_capacity(raw.len());
    let mut cuts: Vec<f32> = Vec::new();
    for (i, seg) in raw.iter().enumerate() {
        cuts.clear();
        cuts.push(0.0);
        cuts.push(1.0);
        for (j, other) in raw.iter().enumerate() {
            if i != j {
                collect_cuts(seg, other, &mut cuts);
            }
        }
        cuts.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let span = seg.a.distance(seg.b);
        let mut prev = 0.0f32;
        for &t in cuts.iter().skip(1) {
            // Skip cuts that land on the previous one
            if (t - prev) * span <= WELD {
                continue;
            }
            out.push(Segment {
                a: seg.a.lerp(seg.b, prev),
                b: seg.a.lerp(seg.b, t),
                entity: seg.entity,
            });
            prev = t;
        }
    }
    out
}

/// Parameters along `seg` where `other` touches it
fn collect_cuts(seg: &Segment, other: &Segment, out: &mut Vec<f32>) {
    let da = seg.b - seg.a;
    let db = other.b - other.a;
    let denom = da.perp_dot(db);
    let scale = da.length() * db.length();
    if scale <= f32::EPSILON {
        return;
    }
    let e = other.a - seg.a;

    if (denom / scale).abs() > PARALLEL_SIN {
        let t = e.perp_dot(db) / denom;
        let u = e.perp_dot(da) / denom;
        let range = -PARAM_SLACK..=1.0 + PARAM_SLACK;
        if range.contains(&t) && range.contains(&u) {
            out.push(t.clamp(0.0, 1.0));
        }
        return;
    }

    // Parallel: only an overlap matters, and then it is the other segment's
    // endpoints that have to become vertices here
    let len_sq = da.length_squared();
    if e.perp_dot(da).abs() / da.length() > WELD {
        return; // parallel but off to the side
    }
    for end in [other.a, other.b] {
        let t = (end - seg.a).dot(da) / len_sq;
        if (PARAM_SLACK..=1.0 - PARAM_SLACK).contains(&t) {
            out.push(t);
        }
    }
}

// ---- planar graph -------------------------------------------------------

/// Directed edge. Its twin is always `index ^ 1`, so the head of an edge is
/// the tail of its twin and needs no field of its own.
struct HalfEdge {
    from: usize,
    angle: f32,
    entity: Uuid,
}

struct Graph {
    verts: Vec<Vec2>,
    edges: Vec<HalfEdge>,
    /// Outgoing edges per vertex, sorted counter-clockwise by angle
    ring: Vec<Vec<usize>>,
    /// Where each edge sits in its origin's ring
    slot: Vec<usize>,
}

impl Graph {
    fn build(segments: &[Segment]) -> Self {
        let mut welder = Welder::default();
        let mut edges: Vec<HalfEdge> = Vec::new();
        let mut seen: HashMap<(usize, usize), ()> = HashMap::new();

        for seg in segments {
            let u = welder.weld(seg.a);
            let v = welder.weld(seg.b);
            if u == v {
                continue;
            }
            let key = (u.min(v), u.max(v));
            if seen.insert(key, ()).is_some() {
                continue; // duplicate edge: one pass through is enough
            }
            let (pu, pv) = (welder.pts[u], welder.pts[v]);
            edges.push(HalfEdge {
                from: u,
                angle: (pv - pu).to_angle(),
                entity: seg.entity,
            });
            edges.push(HalfEdge {
                from: v,
                angle: (pu - pv).to_angle(),
                entity: seg.entity,
            });
        }

        let verts = welder.pts;
        let mut ring: Vec<Vec<usize>> = vec![Vec::new(); verts.len()];
        for (i, e) in edges.iter().enumerate() {
            ring[e.from].push(i);
        }
        for r in &mut ring {
            r.sort_by(|&x, &y| {
                edges[x]
                    .angle
                    .partial_cmp(&edges[y].angle)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let mut slot = vec![0usize; edges.len()];
        for r in &ring {
            for (k, &e) in r.iter().enumerate() {
                slot[e] = k;
            }
        }

        Graph {
            verts,
            edges,
            ring,
            slot,
        }
    }

    /// The next half-edge when tracing a face counter-clockwise: from the
    /// twin, step *clockwise* around the shared vertex
    fn next(&self, edge: usize) -> usize {
        let twin = edge ^ 1;
        let ring = &self.ring[self.edges[twin].from];
        ring[(self.slot[twin] + ring.len() - 1) % ring.len()]
    }

    /// One loop per enclosed area; the unbounded outer face is dropped
    fn walk_faces(&self) -> Vec<Face> {
        let mut visited = vec![false; self.edges.len()];
        let mut faces = Vec::new();

        for start in 0..self.edges.len() {
            if visited[start] {
                continue;
            }
            let mut pts = Vec::new();
            let mut entities = Vec::new();
            let mut edge = start;
            loop {
                visited[edge] = true;
                pts.push(self.verts[self.edges[edge].from]);
                entities.push(self.edges[edge].entity);
                edge = self.next(edge);
                if edge == start {
                    break;
                }
                if visited[edge] {
                    // Malformed walk; bail rather than spin
                    break;
                }
            }
            let area = signed_area(&pts);
            if area > MIN_AREA {
                entities.sort_unstable();
                entities.dedup();
                faces.push(Face {
                    pts,
                    area,
                    entities,
                });
            }
        }
        faces
    }
}

/// Merges points that are the same to within `WELD`
#[derive(Default)]
struct Welder {
    pts: Vec<Vec2>,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl Welder {
    fn weld(&mut self, p: Vec2) -> usize {
        let cell = ((p.x / WELD) as i32, (p.y / WELD) as i32);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(bucket) = self.cells.get(&(cell.0 + dx, cell.1 + dy)) else {
                    continue;
                };
                for &i in bucket {
                    if self.pts[i].distance_squared(p) <= WELD * WELD {
                        return i;
                    }
                }
            }
        }
        let index = self.pts.len();
        self.pts.push(p);
        self.cells.entry(cell).or_default().push(index);
        index
    }
}

struct Face {
    pts: Vec<Vec2>,
    area: f32,
    entities: Vec<Uuid>,
}

// ---- nesting ------------------------------------------------------------

/// Turn raw faces into profiles, giving each one the faces nested directly
/// inside it as holes
fn assemble(mut faces: Vec<Face>) -> Vec<Profile> {
    faces.sort_by(|a, b| {
        b.area
            .partial_cmp(&a.area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Faces from one connected component never overlap, so containment only
    // ever runs between components and a single inside-test settles it
    let mut parent: Vec<Option<usize>> = vec![None; faces.len()];
    for child in 0..faces.len() {
        // Sorted by area, so the last container found is the tightest one
        for candidate in 0..child {
            if contains(&faces[candidate].pts, &faces[child].pts) {
                parent[child] = Some(candidate);
            }
        }
    }

    let mut profiles: Vec<Profile> = Vec::with_capacity(faces.len());
    for (i, face) in faces.iter().enumerate() {
        let mut holes: Vec<Vec<Vec2>> = Vec::new();
        let mut net = face.area;
        for (j, other) in faces.iter().enumerate() {
            if parent[j] == Some(i) {
                let mut hole = other.pts.clone();
                hole.reverse(); // holes run clockwise
                net -= other.area;
                holes.push(hole);
            }
        }
        profiles.push(Profile {
            id: profile_id(&face.entities, i),
            centroid: interior_point(&face.pts, &holes),
            outer: face.pts.clone(),
            holes,
            area: net.max(0.0),
            edges: face.entities.clone(),
        });
    }
    profiles
}

/// Is `inner` inside `outer`? Both come from different components, so they
/// share no boundary and a majority vote over the vertices is decisive
fn contains(outer: &[Vec2], inner: &[Vec2]) -> bool {
    let inside = inner.iter().filter(|p| point_in_loop(outer, **p)).count();
    inside * 2 > inner.len()
}

fn point_in_loop(loop_pts: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    let n = loop_pts.len();
    for i in 0..n {
        let a = loop_pts[i];
        let b = loop_pts[(i + 1) % n];
        if (a.y > p.y) != (b.y > p.y) {
            let t = (p.y - a.y) / (b.y - a.y);
            if p.x < a.x + t * (b.x - a.x) {
                inside = !inside;
            }
        }
    }
    inside
}

/// A point guaranteed to be inside the region: the area centroid when it
/// lands inside, otherwise the middle of the widest span across it
fn interior_point(outer: &[Vec2], holes: &[Vec<Vec2>]) -> Vec2 {
    let c = area_centroid(outer);
    if point_in_loop(outer, c) && !holes.iter().any(|h| point_in_loop(h, c)) {
        return c;
    }
    // Scan a few heights and keep the widest gap that is really inside
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for p in outer {
        lo = lo.min(p.y);
        hi = hi.max(p.y);
    }
    let mut best = c;
    let mut best_width = -1.0f32;
    for step in 1..8 {
        let y = lo + (hi - lo) * (step as f32 / 8.0);
        let mut xs: Vec<f32> = crossings(outer, y);
        for hole in holes {
            xs.extend(crossings(hole, y));
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks(2) {
            let [x0, x1] = pair else { continue };
            let mid = Vec2::new((x0 + x1) / 2.0, y);
            let width = x1 - x0;
            if width > best_width
                && point_in_loop(outer, mid)
                && !holes.iter().any(|h| point_in_loop(h, mid))
            {
                best_width = width;
                best = mid;
            }
        }
    }
    best
}

fn crossings(loop_pts: &[Vec2], y: f32) -> Vec<f32> {
    let n = loop_pts.len();
    let mut xs = Vec::new();
    for i in 0..n {
        let a = loop_pts[i];
        let b = loop_pts[(i + 1) % n];
        if (a.y > y) != (b.y > y) {
            let t = (y - a.y) / (b.y - a.y);
            xs.push(a.x + t * (b.x - a.x));
        }
    }
    xs
}

fn area_centroid(pts: &[Vec2]) -> Vec2 {
    let n = pts.len();
    let mut area = 0.0f32;
    let mut acc = Vec2::ZERO;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let cross = a.perp_dot(b);
        area += cross;
        acc += (a + b) * cross;
    }
    if area.abs() < f32::EPSILON {
        return pts.iter().copied().sum::<Vec2>() / n.max(1) as f32;
    }
    acc / (3.0 * area)
}

pub(crate) fn signed_area(pts: &[Vec2]) -> f32 {
    let n = pts.len();
    let mut sum = 0.0;
    for i in 0..n {
        sum += pts[i].perp_dot(pts[(i + 1) % n]);
    }
    sum / 2.0
}

/// A region is named by the curves that bound it, so it keeps the same ID
/// when the sketch is edited elsewhere — a feature can hold on to it.
///
/// `rank` separates regions that happen to share a boundary set, such as the
/// three areas two overlapping circles cut out of each other.
fn profile_id(entities: &[Uuid], rank: usize) -> Uuid {
    // 128-bit FNV-1a: no extra dependency, and stable across runs unlike the
    // std hasher
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013b;
    let mut hash = OFFSET;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= *byte as u128;
            hash = hash.wrapping_mul(PRIME);
        }
    };
    for id in entities {
        feed(id.as_bytes());
    }
    feed(&(rank as u64).to_le_bytes());

    let mut bytes = hash.to_be_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x80; // version 8: custom
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sketch::SketchPlane;

    /// Square with `size` sides, lower-left at the origin
    fn square(sketch: &mut Sketch, origin: Vec2, size: f32) -> Vec<Uuid> {
        let corners = [
            origin,
            origin + Vec2::new(size, 0.0),
            origin + Vec2::new(size, size),
            origin + Vec2::new(0.0, size),
        ];
        let ids: Vec<Uuid> = corners.iter().map(|c| sketch.add_point(*c)).collect();
        (0..4)
            .map(|i| sketch.add_line(ids[i], ids[(i + 1) % 4]))
            .collect()
    }

    fn empty() -> Sketch {
        Sketch::new("test", SketchPlane::xy())
    }

    #[test]
    fn square_is_one_region() {
        let mut sketch = empty();
        square(&mut sketch, Vec2::ZERO, 2.0);
        let profiles = extract(&sketch);
        assert_eq!(profiles.len(), 1);
        assert!(
            (profiles[0].area - 4.0).abs() < 1e-4,
            "{}",
            profiles[0].area
        );
        assert!(profiles[0].holes.is_empty());
        assert!(signed_area(&profiles[0].outer) > 0.0, "outer must be CCW");
    }

    #[test]
    fn circle_is_one_region() {
        let mut sketch = empty();
        let c = sketch.add_point(Vec2::ZERO);
        sketch.add_circle(c, 1.0);
        let profiles = extract(&sketch);
        assert_eq!(profiles.len(), 1);
        // A 64-gon inscribed in the unit circle is a hair under π
        assert!((profiles[0].area - std::f32::consts::PI).abs() < 0.01);
    }

    #[test]
    fn a_line_across_a_square_splits_it() {
        let mut sketch = empty();
        square(&mut sketch, Vec2::ZERO, 2.0);
        // Diagonal, drawn with its own endpoints outside the square so it
        // only meets the sides by crossing them
        let a = sketch.add_point(Vec2::new(-1.0, -1.0));
        let b = sketch.add_point(Vec2::new(3.0, 3.0));
        sketch.add_line(a, b);

        let profiles = extract(&sketch);
        assert_eq!(profiles.len(), 2, "the diagonal cuts the square in two");
        for p in &profiles {
            assert!((p.area - 2.0).abs() < 1e-4, "{}", p.area);
        }
    }

    #[test]
    fn a_circle_inside_a_square_becomes_a_hole() {
        let mut sketch = empty();
        square(&mut sketch, Vec2::ZERO, 4.0);
        let c = sketch.add_point(Vec2::new(2.0, 2.0));
        sketch.add_circle(c, 1.0);

        let profiles = extract(&sketch);
        assert_eq!(profiles.len(), 2);
        let ring = &profiles[0];
        let disc = &profiles[1];
        assert_eq!(ring.holes.len(), 1, "the square keeps the circle as a hole");
        assert!(disc.holes.is_empty());
        assert!((ring.area - (16.0 - std::f32::consts::PI)).abs() < 0.02);
        assert!(signed_area(&ring.holes[0]) < 0.0, "holes run clockwise");
    }

    #[test]
    fn construction_geometry_is_ignored() {
        let mut sketch = empty();
        let lines = square(&mut sketch, Vec2::ZERO, 2.0);
        sketch.set_construction(lines[0], true);
        assert!(extract(&sketch).is_empty(), "an open outline has no region");
    }

    #[test]
    fn open_geometry_yields_nothing() {
        let mut sketch = empty();
        let a = sketch.add_point(Vec2::ZERO);
        let b = sketch.add_point(Vec2::new(1.0, 0.0));
        let c = sketch.add_point(Vec2::new(1.0, 1.0));
        sketch.add_line(a, b);
        sketch.add_line(b, c);
        assert!(extract(&sketch).is_empty());
    }

    #[test]
    fn ids_survive_an_edit_elsewhere() {
        let mut sketch = empty();
        square(&mut sketch, Vec2::ZERO, 2.0);
        let before = extract(&sketch);

        // A second, unrelated square must not rename the first one's region
        square(&mut sketch, Vec2::new(10.0, 10.0), 1.0);
        let after = extract(&sketch);

        assert_eq!(after.len(), 2);
        assert!(
            after.iter().any(|p| p.id == before[0].id),
            "the original region kept its id",
        );
    }

    #[test]
    fn centroid_lands_inside_an_l_shape() {
        let mut sketch = empty();
        let corners = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 3.0),
            Vec2::new(0.0, 3.0),
        ];
        let ids: Vec<Uuid> = corners.iter().map(|c| sketch.add_point(*c)).collect();
        for i in 0..6 {
            sketch.add_line(ids[i], ids[(i + 1) % 6]);
        }
        let profiles = extract(&sketch);
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert!(
            point_in_loop(&p.outer, p.centroid),
            "centroid {:?} escaped the L",
            p.centroid,
        );
    }
}
