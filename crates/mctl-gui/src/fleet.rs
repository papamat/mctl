//! Lightweight fleet overview: orthographic Earth and equirectangular subsatellite map.
//! Positions come only from the published solution; no propagation or synthetic positions here.
use crate::{coast::Line, theme::Theme};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use mctl_core::{
    config::Regime,
    world::{VehicleView, World},
};

fn selected<'a>(world: &'a World, only: &[String]) -> Vec<&'a VehicleView> {
    world
        .vehicles
        .iter()
        .filter(|v| v.regime == Regime::Earth && (only.is_empty() || only.contains(&v.id)))
        .collect()
}

/// Unit-sphere orthographic projection, with positive depth facing the viewer.
fn project(lon: f64, lat: f64, centre_lon: f64) -> [f32; 3] {
    let (s, c) = lat.to_radians().sin_cos();
    let (sl, cl) = (lon - centre_lon).to_radians().sin_cos();
    [(c * sl) as f32, -s as f32, (c * cl) as f32]
}

const EARTH_RADIUS_KM: f64 = 6378.137;

/// Orbit geometry uses one common Earth orientation at the snapshot time, not
/// the future Earth rotation used by ground-track longitudes.
fn orbit_project(r: [f64; 3], matrix: [[f64; 3]; 3], lon: f64) -> [f32; 3] {
    let e: [f64; 3] =
        matrix.map(|row| row.iter().zip(r).map(|(a, b)| a * b).sum::<f64>() / EARTH_RADIUS_KM);
    let (sin, cos) = lon.to_radians().sin_cos();
    [
        (e[1] * cos - e[0] * sin) as f32,
        -e[2] as f32,
        (e[0] * cos + e[1] * sin) as f32,
    ]
}

fn visible(q: [f32; 3]) -> bool {
    q.iter().all(|v| v.is_finite()) && (q[2] >= 0.0 || q[0] * q[0] + q[1] * q[1] > 1.0)
}

fn map_segment(a: &mctl_core::world::TrackPoint, b: &mctl_core::world::TrackPoint) -> bool {
    [a.lon_deg, a.lat_deg, b.lon_deg, b.lat_deg]
        .iter()
        .all(|v| v.is_finite())
        && (a.lon_deg - b.lon_deg).abs() <= 180.0
}

fn track_segment(
    p: &Painter,
    points: [Pos2; 2],
    stroke: Stroke,
    future: bool,
    dash: f32,
    phase: &mut f32,
) {
    if !future {
        p.line_segment(points, stroke);
        return;
    }
    let delta = points[1] - points[0];
    let length = delta.length();
    if !length.is_finite() || length < 0.001 {
        return;
    }
    let mut offset = 0.0;
    while offset < length {
        let on = *phase < dash;
        let end = if on { dash } else { dash * 2.0 };
        let step = (end - *phase).min(length - offset).max(0.001);
        if on {
            p.line_segment(
                [
                    points[0] + delta * (offset / length),
                    points[0] + delta * ((offset + step).min(length) / length),
                ],
                stroke,
            );
        }
        offset += step;
        *phase = (*phase + step) % (dash * 2.0);
    }
}

fn marker(
    p: &Painter,
    at: Pos2,
    n: usize,
    area: Rect,
    font: &FontId,
    colour: Color32,
    labels: &mut Vec<Rect>,
) {
    p.circle_filled(at, 2.5, colour);
    let text = format!("{n:02}");
    let size = Vec2::new(font.size * 1.3, font.size * 1.2);
    let mut label = Rect::from_min_size(at + Vec2::new(5.0, -size.y), size);
    // Bounded label placement, retaining a leader to the real position in tight clusters.
    for step in 0..32 {
        let angle = (step % 8) as f32 * std::f32::consts::TAU / 8.0;
        let radius = 12.0 + (step / 8) as f32 * size.y;
        let candidate = at + Vec2::angled(angle) * radius;
        let min = Pos2::new(
            candidate
                .x
                .clamp(area.left(), (area.right() - size.x).max(area.left())),
            candidate
                .y
                .clamp(area.top(), (area.bottom() - size.y).max(area.top())),
        );
        label = Rect::from_min_size(min, size);
        if labels.iter().all(|r| !r.expand(2.0).intersects(label)) {
            break;
        }
    }
    labels.push(label);
    p.line_segment(
        [at, label.center()],
        Stroke::new(0.7, colour.gamma_multiply(0.6)),
    );
    p.rect_filled(label, 0.0, Color32::BLACK);
    p.text(label.min, Align2::LEFT_TOP, text, font.clone(), colour);
}

pub fn paint(
    p: &Painter,
    theme: &Theme,
    px: [u32; 4],
    coast: &[Line],
    world: &World,
    only: &[String],
    elapsed: f64,
) {
    let area = Rect::from_min_size(
        Pos2::new(px[0] as f32, px[1] as f32),
        Vec2::new(px[2] as f32, px[3] as f32),
    );
    let p = p.with_clip_rect(area);
    let vehicles = selected(world, only);
    let scale = (area.width() / 608.0).max(0.5);
    let line_h = 17.0 * scale;
    let font = FontId::monospace(11.0 * scale);
    let bright = theme.palette.bright.to_egui();
    let dim = theme.palette.dim.to_egui();
    // Two-column legend; paginate only when a larger configured fleet exceeds the screen.
    let capacity_rows = ((area.height() * 0.36 / line_h) as usize).max(1);
    let capacity = capacity_rows * 2;
    let pages = vehicles.len().div_ceil(capacity).max(1);
    let page = (elapsed.max(0.0) / 6.0) as usize % pages;
    let legend_rows = vehicles.len().div_ceil(2).min(capacity_rows).max(1);
    let picture_bottom = area.bottom() - (legend_rows + 2) as f32 * line_h;
    let top = area.top() + line_h;
    let picture_h = (picture_bottom - top).max(1.0);
    let left = Rect::from_min_max(
        Pos2::new(area.left(), top),
        Pos2::new(area.center().x - 8.0 * scale, picture_bottom),
    );
    let right = Rect::from_min_max(
        Pos2::new(area.center().x + 8.0 * scale, top),
        Pos2::new(area.right(), picture_bottom),
    );
    p.text(
        Pos2::new(left.center().x, area.top()),
        Align2::CENTER_TOP,
        "EARTH / ROTATING",
        font.clone(),
        bright,
    );
    p.text(
        Pos2::new(right.center().x, area.top()),
        Align2::CENTER_TOP,
        "WORLD MAP / GROUND TRACKS",
        font.clone(),
        bright,
    );
    let extent = vehicles
        .iter()
        .filter_map(|v| v.track.as_ref())
        .flat_map(|t| &t.points)
        .map(|p| p.r_gcrf_km.iter().map(|x| x * x).sum::<f64>().sqrt() / EARTH_RADIUS_KM)
        .filter(|r| r.is_finite())
        .fold(1.15_f64, f64::max);
    let radius = ((left.width().min(picture_h) / 2.0 - 8.0 * scale) / extent as f32).max(1.0);
    let centre = left.center();
    let centre_lon = world.observer.active_site().map_or(0.0, |s| s.lon_deg) + elapsed * 3.0;
    let globe = |lon, lat| {
        let q = project(lon, lat, centre_lon);
        (centre + Vec2::new(q[0], q[1]) * radius, q[2])
    };
    p.circle_filled(centre, radius, theme.palette.background.to_egui());
    p.circle_stroke(centre, radius, Stroke::new(1.0, bright));
    let map_w = right.width().min(picture_h * 2.0);
    let map = Rect::from_center_size(right.center(), Vec2::new(map_w, map_w / 2.0));
    let flat = |lon: f64, lat: f64| {
        Pos2::new(
            map.left() + ((lon + 180.0) / 360.0) as f32 * map.width(),
            map.top() + ((90.0 - lat) / 180.0) as f32 * map.height(),
        )
    };
    let stroke = Stroke::new(0.7, dim);
    for lat in (-60..=60).step_by(30) {
        p.line_segment([flat(-180.0, lat as f64), flat(180.0, lat as f64)], stroke);
        for lon in (-180..180).step_by(5) {
            let (a, za) = globe(lon as f64, lat as f64);
            let (b, zb) = globe((lon + 5) as f64, lat as f64);
            if za >= 0.0 && zb >= 0.0 {
                p.line_segment([a, b], stroke);
            }
        }
    }
    for lon in (-180..=180).step_by(30) {
        p.line_segment([flat(lon as f64, -90.0), flat(lon as f64, 90.0)], stroke);
        for lat in (-90..90).step_by(5) {
            let (a, za) = globe(lon as f64, lat as f64);
            let (b, zb) = globe(lon as f64, (lat + 5) as f64);
            if za >= 0.0 && zb >= 0.0 {
                p.line_segment([a, b], stroke);
            }
        }
    }
    for line in coast {
        for pair in line.windows(2) {
            let [(lo1, la1), (lo2, la2)] = [pair[0], pair[1]];
            if (lo1 - lo2).abs() <= 180.0 {
                p.line_segment(
                    [flat(lo1 as f64, la1 as f64), flat(lo2 as f64, la2 as f64)],
                    Stroke::new(1.0, bright.gamma_multiply(0.65)),
                );
            }
            let (a, za) = globe(lo1 as f64, la1 as f64);
            let (b, zb) = globe(lo2 as f64, la2 as f64);
            if za >= 0.0 && zb >= 0.0 {
                p.line_segment([a, b], Stroke::new(1.0, bright.gamma_multiply(0.65)));
            }
        }
    }
    // Draw all cached tracks once, underneath the brighter numbered positions.
    // Predicted portions are dashed; historical portions remain solid.
    let orbit_painter = p.with_clip_rect(left);
    let map_painter = p.with_clip_rect(map);
    for v in &vehicles {
        let (Some(sol), Some(track)) = (&v.solution, &v.track) else {
            continue;
        };
        let stroke = Stroke::new(0.9 * scale, bright.gamma_multiply(0.40));
        let (mut orbit_phase, mut map_phase) = (0.0, 0.0);
        for (i, pair) in track.points.windows(2).enumerate() {
            let future = i + 1 >= track.now_index;
            if map_segment(&pair[0], &pair[1]) {
                track_segment(
                    &map_painter,
                    [
                        flat(pair[0].lon_deg, pair[0].lat_deg),
                        flat(pair[1].lon_deg, pair[1].lat_deg),
                    ],
                    stroke,
                    future,
                    3.0 * scale,
                    &mut map_phase,
                );
            }
            let a = orbit_project(pair[0].r_gcrf_km, sol.ecef_from_gcrf, centre_lon);
            let b = orbit_project(pair[1].r_gcrf_km, sol.ecef_from_gcrf, centre_lon);
            // The midpoint test prevents a coarse chord passing through the hidden Earth disc.
            let mid = std::array::from_fn(|j| (a[j] + b[j]) * 0.5);
            if visible(a) && visible(b) && visible(mid) {
                track_segment(
                    &orbit_painter,
                    [
                        centre + Vec2::new(a[0], a[1]) * radius,
                        centre + Vec2::new(b[0], b[1]) * radius,
                    ],
                    stroke,
                    future,
                    3.0 * scale,
                    &mut orbit_phase,
                );
            }
        }
    }
    let (mut globe_labels, mut map_labels) = (Vec::new(), Vec::new());
    let mut placed = 0;
    for (i, v) in vehicles.iter().enumerate() {
        if let Some(s) = &v.solution {
            let g = &s.geodetic;
            if !g.lon_deg.is_finite() || !g.lat_deg.is_finite() {
                continue;
            }
            placed += 1;
            let q = orbit_project(
                [s.state_gcrf.r.x, s.state_gcrf.r.y, s.state_gcrf.r.z],
                s.ecef_from_gcrf,
                centre_lon,
            );
            let at = centre + Vec2::new(q[0], q[1]) * radius;
            if visible(q) {
                marker(&p, at, i + 1, left, &font, bright, &mut globe_labels);
            }
            marker(
                &p,
                flat(g.lon_deg, g.lat_deg),
                i + 1,
                right,
                &font,
                bright,
                &mut map_labels,
            );
        }
    }
    for (j, (i, v)) in vehicles
        .iter()
        .enumerate()
        .skip(page * capacity)
        .take(capacity)
        .enumerate()
    {
        let status = v.solution.as_ref().map_or("NO DATA", |s| s.quality.label());
        let label = format!("{:02} {}  [{}]", i + 1, v.name.to_uppercase(), status);
        let x = area.left() + (j % 2) as f32 * area.width() / 2.0;
        let y = picture_bottom + line_h + (j / 2) as f32 * line_h;
        let fit_font = FontId::monospace(
            font.size
                .min((area.width() / 2.0 - 6.0 * scale) / (label.len().max(1) as f32 * 0.61)),
        );
        p.text(Pos2::new(x, y), Align2::LEFT_TOP, label, fit_font, bright);
    }
    p.text(
        Pos2::new(area.center().x, area.bottom()),
        Align2::CENTER_BOTTOM,
        format!(
            "{placed}/{} PLACED | SOLID: PAST / DASH: PRED | KEY {}/{}",
            vehicles.len(),
            page + 1,
            pages
        ),
        font,
        dim,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn globe_front_back_and_wrap() {
        assert_eq!(project(0.0, 0.0, 0.0), [0.0, -0.0, 1.0]);
        assert!(project(180.0, 0.0, 0.0)[2] < 0.0);
        assert!((project(181.0, 25.0, 30.0)[0] - project(-179.0, 25.0, 30.0)[0]).abs() < 1e-6);
        assert!((project(0.0, 90.0, 0.0)[1] + 1.0).abs() < 1e-6);
    }
    #[test]
    fn orbit_projection_keeps_altitude_and_occludes_the_far_side() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let q = orbit_project([0.0, EARTH_RADIUS_KM * 1.1, 0.0], identity, 0.0);
        assert!((q[0] - 1.1).abs() < 1e-6);
        assert!(!visible([0.0, 0.0, -1.1]));
        assert!(visible([1.1, 0.0, -0.1]));
        assert!(!visible([f32::NAN, 0.0, 1.0]));
        let rotated = orbit_project([EARTH_RADIUS_KM, 0.0, 0.0], identity, 90.0);
        assert!((rotated[0] + 1.0).abs() < 1e-6);
    }
    #[test]
    fn ground_tracks_break_at_the_dateline_and_invalid_samples() {
        use mctl_core::{time::Utc, world::TrackPoint};
        let mut a = TrackPoint {
            t: Utc(0.0),
            r_gcrf_km: [0.0; 3],
            lat_deg: 0.0,
            lon_deg: 179.0,
            alt_km: 400.0,
            illumination: 1.0,
            rot_km: [0.0; 3],
        };
        let mut b = a;
        b.lon_deg = -179.0;
        assert!(!map_segment(&a, &b));
        b.lon_deg = 178.0;
        assert!(map_segment(&a, &b));
        a.lat_deg = f64::NAN;
        assert!(!map_segment(&a, &b));
    }
    #[test]
    fn fleet_respects_selection_and_keeps_missing_data() {
        let mut w = crate::kiosk::test_world(3);
        w.vehicles[2].regime = Regime::Deep;
        assert_eq!(selected(&w, &[]).len(), 2);
        let ids = vec!["v1".to_owned()];
        let v = selected(&w, &ids);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "v1");
        assert!(v[0].solution.is_none());
    }
}
