//! Programmatically drawn Vulture icon — no binary assets needed.
//! Amber head + hooked beak inside swept wing strokes, on ink.

/// Generate a 256x256 RGBA icon: amber vulture mark on a dark rounded square.
pub fn app_icon() -> egui::IconData {
    const S: u32 = 256;
    const INK: (u8, u8, u8) = (20, 23, 30);
    const AMBER: (u8, u8, u8) = (255, 176, 0);

    let mut rgba = vec![0u8; (S * S * 4) as usize];

    // Wing strokes (left, right) — thick, swept upward at the tips.
    let left_wing = [(28.0, 92.0), (128.0, 232.0)];
    let right_wing = [(228.0, 92.0), (128.0, 232.0)];
    let wing_thickness = 17.0;
    // Secondary inner feather strokes (thinner, echo the wings).
    let inner_left = [(56.0, 108.0), (128.0, 212.0)];
    let inner_right = [(200.0, 108.0), (128.0, 212.0)];
    let inner_thickness = 6.5;

    // Head and beak geometry.
    let head = (128.0, 88.0, 40.0); // center_x, center_y, radius
    let beak = [(108.0, 108.0), (148.0, 108.0), (128.0, 160.0)]; // hooked beak triangle

    for y in 0..S {
        for x in 0..S {
            let fx = x as f32;
            let fy = y as f32;
            let idx = ((S * y + x) * 4) as usize;

            // Rounded-square mask (radius 56).
            let qx = (fx - 128.0).abs() - (128.0 - 56.0);
            let qy = (fy - 128.0).abs() - (128.0 - 56.0);
            let outside = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - 56.0;
            if outside > 0.0 {
                continue;
            }

            let d_wing = point_seg_distance(
                fx,
                fy,
                left_wing[0].0,
                left_wing[0].1,
                left_wing[1].0,
                left_wing[1].1,
            )
            .min(point_seg_distance(
                fx,
                fy,
                right_wing[0].0,
                right_wing[0].1,
                right_wing[1].0,
                right_wing[1].1,
            ));
            let d_inner = point_seg_distance(
                fx,
                fy,
                inner_left[0].0,
                inner_left[0].1,
                inner_left[1].0,
                inner_left[1].1,
            )
            .min(point_seg_distance(
                fx,
                fy,
                inner_right[0].0,
                inner_right[0].1,
                inner_right[1].0,
                inner_right[1].1,
            ));
            let head_dist = (fx - head.0).hypot(fy - head.1);
            let in_beak = point_in_triangle(fx, fy, beak[0], beak[1], beak[2]);

            let (r, g, b) = if head_dist <= head.2
                || d_wing <= wing_thickness
                || d_inner <= inner_thickness
                || in_beak
            {
                AMBER
            } else {
                INK
            };
            // Beak hook carved back out in ink so the head reads as a vulture, not a blob.
            let in_beak_inner = point_in_triangle(
                fx,
                fy,
                (beak[0].0 + 14.0, beak[0].1 + 2.0),
                (beak[1].0 - 14.0, beak[1].1 + 2.0),
                (128.0, beak[2].1 - 34.0),
            );
            let (r, g, b) = if in_beak_inner { INK } else { (r, g, b) };

            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = 255;
        }
    }

    egui::IconData {
        rgba,
        width: S,
        height: S,
    }
}

fn point_seg_distance(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let len_sq = abx * abx + aby * aby;
    let t = if len_sq == 0.0 {
        // Degenerate segment — a single point; measure distance to it.
        0.0
    } else {
        ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0)
    };
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    (px - cx).hypot(py - cy)
}

fn point_in_triangle(px: f32, py: f32, a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let sign = |p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| {
        (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1)
    };
    let p = (px, py);
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_seg_distance_handles_degenerate_segment() {
        let d = point_seg_distance(0.0, 0.0, 3.0, 4.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 1e-6);
    }

    #[test]
    fn point_seg_distance_is_zero_on_the_segment() {
        let d = point_seg_distance(5.0, 0.0, 0.0, 0.0, 10.0, 0.0);
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn point_in_triangle_classifies_correctly() {
        let a = (0.0, 0.0);
        let b = (10.0, 0.0);
        let c = (5.0, 10.0);
        assert!(point_in_triangle(5.0, 4.0, a, b, c));
        assert!(!point_in_triangle(9.0, 9.0, a, b, c));
        assert!(point_in_triangle(0.0, 0.0, a, b, c)); // vertex counts as inside
    }

    #[test]
    fn app_icon_is_256_and_opaque_inside_mask() {
        let icon = app_icon();
        assert_eq!(icon.width, 256);
        assert_eq!(icon.height, 256);
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // Center pixel must be painted opaque.
        let center = ((256 * 128 + 128) * 4) as usize;
        assert_eq!(icon.rgba[center + 3], 255);
    }
}
