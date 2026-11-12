//! Programmatically drawn Vulture icon — no binary assets needed.

/// Generate a 256x256 RGBA icon: amber "V" beak on a dark rounded square.
pub fn app_icon() -> egui::IconData {
    const S: u32 = 256;
    let mut rgba = vec![0u8; (S * S * 4) as usize];

    let (ax, ay) = (62.0f32, 74.0f32); // top-left of the V
    let (bx, by) = (194.0f32, 74.0f32); // top-right of the V
    let (tx, ty) = (128.0f32, 200.0f32); // bottom tip of the V
    let thickness = 21.0f32;

    for y in 0..S {
        for x in 0..S {
            let fx = x as f32;
            let fy = y as f32;
            let rel_x = fx - 128.0;
            let rel_y = fy - 128.0;

            // Rounded-square mask (radius 60).
            let qx = rel_x.abs() - (128.0 - 60.0);
            let qy = rel_y.abs() - (128.0 - 60.0);
            let outside = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - 60.0;

            let idx = ((S * y + x) * 4) as usize;
            if outside > 0.0 {
                rgba[idx + 3] = 0;
                continue;
            }

            let d = point_seg_distance(fx, fy, ax, ay, tx, ty)
                .min(point_seg_distance(fx, fy, bx, by, tx, ty));

            let (r, g, b) = if d <= thickness {
                (244, 180, 54) // amber beak
            } else {
                (19, 22, 29) // dark slate background
            };
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
    let t = ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0);
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    (px - cx).hypot(py - cy)
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
}
