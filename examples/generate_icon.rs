use std::path::Path;

fn rounded_rect_sdf(dx: f32, dy: f32, half_w: f32, half_h: f32, corner_r: f32) -> f32 {
    let adx = dx.abs();
    let ady = dy.abs();
    let corner_dx = (adx - (half_w - corner_r)).max(0.0);
    let corner_dy = (ady - (half_h - corner_r)).max(0.0);
    (corner_dx * corner_dx + corner_dy * corner_dy).sqrt() - corner_r
}

fn main() {
    const SIZE: u32 = 256;
    const CX: f32 = (SIZE - 1) as f32 / 2.0;
    const CY: f32 = (SIZE - 1) as f32 / 2.0;
    const HALF_W: f32 = 96.0;
    const HALF_H: f32 = 96.0;
    const CORNER_R: f32 = 40.0;

    let mut img = image::RgbaImage::new(SIZE, SIZE);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = x as f32 - CX;
        let dy = y as f32 - CY;
        let dist = rounded_rect_sdf(dx, dy, HALF_W, HALF_H, CORNER_R);

        let alpha = if dist < -1.0 {
            1.0
        } else if dist < 1.0 {
            0.5 - dist / 2.0
        } else {
            0.0
        };

        if alpha > 0.0 {
            let t = (x + y) as f32 / (2.0 * SIZE as f32);
            let bg_r = (66.0 + t * 50.0) as u8;
            let bg_g = (133.0 + t * 80.0) as u8;
            let bg_b = (244.0 - t * 50.0) as u8;

            // Crescent moon (Hypnos motif)
            let moon1_dist =
                ((x as f32 - (CX - 20.0)).powi(2) + (y as f32 - CY).powi(2)).sqrt();
            let moon2_dist =
                ((x as f32 - (CX + 12.0)).powi(2) + (y as f32 - CY).powi(2)).sqrt();

            let (r, g, b) = if moon1_dist < 48.0 && moon2_dist > 36.0 {
                (255, 255, 255)
            } else {
                (bg_r, bg_g, bg_b)
            };

            *pixel = image::Rgba([r, g, b, (alpha * 255.0) as u8]);
        }
    }

    let out = Path::new("assets/icon.png");
    img.save(out).expect("failed to save icon");
    println!("Icon saved to {}", out.display());
}
