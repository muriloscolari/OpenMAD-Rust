//! Procedural Apple-style dark squircle + Hall-effect keycap icon generator.

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rounded_box_sdf(px: f32, py: f32, bx: f32, by: f32, r: f32) -> f32 {
    let qx = px.abs() - bx + r;
    let qy = py.abs() - by + r;
    let out_x = qx.max(0.0);
    let out_y = qy.max(0.0);
    (out_x * out_x + out_y * out_y).sqrt() + qx.max(qy).min(0.0) - r
}

fn blend(bg: [f32; 4], fg: [f32; 4]) -> [f32; 4] {
    let a = fg[3];
    let inv = 1.0 - a;
    [
        fg[0] * a + bg[0] * inv,
        fg[1] * a + bg[1] * inv,
        fg[2] * a + bg[2] * inv,
        (a + bg[3] * inv).clamp(0.0, 1.0),
    ]
}

/// Generates an anti-aliased `size x size` RGBA8 icon with an Apple-style dark
/// squircle chassis, sculpted 3D keycap, and `#0A84FF -> #64D2FF` Hall-effect glow bar.
pub fn generate_app_icon_rgba(size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    let half = s * 0.5;
    let aa = 1.25 / s;

    for y in 0..size {
        for x in 0..size {
            let nx = ((x as f32 + 0.5) - half) / s;
            let ny = ((y as f32 + 0.5) - half) / s;

            // 1. Outer Apple-style squircle chassis
            let d_outer = rounded_box_sdf(nx, ny, 0.44, 0.44, 0.115);
            let outer_alpha = 1.0 - smoothstep(-aa, aa, d_outer);
            if outer_alpha <= 0.001 {
                continue;
            }

            let t_y = (ny + 0.44) / 0.88;
            let mut col = [
                0.088 - 0.032 * t_y,
                0.092 - 0.034 * t_y,
                0.114 - 0.036 * t_y,
                outer_alpha,
            ];

            // Subtle inner top rim highlight
            let rim = smoothstep(-0.009, 0.0, d_outer) * (1.0 - smoothstep(0.0, 0.006, d_outer));
            let rim_top = (1.0 - smoothstep(-0.2, 0.25, ny)) * 0.14 + 0.05;
            col = blend(col, [1.0, 1.0, 1.0, rim * rim_top]);

            // 2. Soft drop shadow behind keycap
            let d_shadow = rounded_box_sdf(nx, ny - 0.025, 0.275, 0.26, 0.065);
            let sh_a = (1.0 - smoothstep(-0.02, 0.05, d_shadow)) * 0.55;
            col = blend(col, [0.0, 0.0, 0.0, sh_a]);

            // 3. Keycap outer skirt (3D beveled base)
            let d_skirt = rounded_box_sdf(nx, ny + 0.005, 0.265, 0.25, 0.06);
            let skirt_a = 1.0 - smoothstep(-aa, aa, d_skirt);
            if skirt_a > 0.0 {
                let kt = (ny + 0.25) / 0.50;
                let skirt_col = [
                    0.155 - 0.045 * kt,
                    0.160 - 0.045 * kt,
                    0.195 - 0.045 * kt,
                    skirt_a,
                ];
                col = blend(col, skirt_col);
            }

            // 4. Sculpted inner keycap top face
            let d_top = rounded_box_sdf(nx, ny - 0.022, 0.215, 0.195, 0.048);
            let top_a = 1.0 - smoothstep(-aa, aa, d_top);
            if top_a > 0.0 {
                let ft = (ny + 0.20) / 0.40;
                let face_col = [
                    0.220 - 0.035 * ft,
                    0.228 - 0.035 * ft,
                    0.275 - 0.040 * ft,
                    top_a,
                ];
                col = blend(col, face_col);

                let top_rim = smoothstep(-0.008, 0.0, d_top) * (1.0 - smoothstep(0.0, 0.004, d_top));
                col = blend(col, [1.0, 1.0, 1.0, top_rim * 0.14 * top_a]);
            }

            // 5. Hall-effect glow bloom + #0A84FF -> #64D2FF LED bar
            let d_glow = rounded_box_sdf(nx, ny - 0.125, 0.135, 0.024, 0.024);
            let bloom_a = (1.0 - smoothstep(-0.01, 0.09, d_glow)) * 0.42;
            col = blend(col, [0.039, 0.518, 1.0, bloom_a]);

            let bar_a = 1.0 - smoothstep(-aa, aa, d_glow);
            if bar_a > 0.0 {
                let gx = ((nx + 0.135) / 0.27).clamp(0.0, 1.0);
                let bar_col = [
                    0.039 + (0.392 - 0.039) * gx,
                    0.518 + (0.824 - 0.518) * gx,
                    1.0,
                    bar_a,
                ];
                col = blend(col, bar_col);
            }

            let idx = ((y * size + x) * 4) as usize;
            out[idx] = (col[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 1] = (col[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 2] = (col[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            out[idx + 3] = (col[3].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    out
}
