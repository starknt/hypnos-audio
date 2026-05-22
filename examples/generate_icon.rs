use std::io::{self, Write};
use std::path::Path;

use image::codecs::png::PngEncoder;
use image::ImageEncoder;

fn rounded_rect_sdf(dx: f32, dy: f32, half_w: f32, half_h: f32, corner_r: f32) -> f32 {
    let adx = dx.abs();
    let ady = dy.abs();
    let corner_dx = (adx - (half_w - corner_r)).max(0.0);
    let corner_dy = (ady - (half_h - corner_r)).max(0.0);
    (corner_dx * corner_dx + corner_dy * corner_dy).sqrt() - corner_r
}

fn generate_icon(size: u32) -> image::RgbaImage {
    let scale = size as f32 / 256.0;
    let cx = (size - 1) as f32 / 2.0;
    let cy = (size - 1) as f32 / 2.0;
    let half_w = 96.0 * scale;
    let half_h = 96.0 * scale;
    let corner_r = 40.0 * scale;
    let moon1_offset = 20.0 * scale;
    let moon2_offset = 12.0 * scale;
    let moon_radius1 = 48.0 * scale;
    let moon_radius2 = 36.0 * scale;

    let mut img = image::RgbaImage::new(size, size);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let dist = rounded_rect_sdf(dx, dy, half_w, half_h, corner_r);

        let alpha = if dist < -1.0 {
            1.0
        } else if dist < 1.0 {
            0.5 - dist / 2.0
        } else {
            0.0
        };

        if alpha > 0.0 {
            let t = (x + y) as f32 / (2.0 * size as f32);
            let bg_r = (66.0 + t * 50.0) as u8;
            let bg_g = (133.0 + t * 80.0) as u8;
            let bg_b = (244.0 - t * 50.0) as u8;

            let moon1_dist =
                ((x as f32 - (cx - moon1_offset)).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let moon2_dist =
                ((x as f32 - (cx + moon2_offset)).powi(2) + (y as f32 - cy).powi(2)).sqrt();

            let (r, g, b) = if moon1_dist < moon_radius1 && moon2_dist > moon_radius2 {
                (255, 255, 255)
            } else {
                (bg_r, bg_g, bg_b)
            };

            *pixel = image::Rgba([r, g, b, (alpha * 255.0) as u8]);
        }
    }

    img
}

fn encode_png(img: &image::RgbaImage) -> Vec<u8> {
    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .expect("PNG encode failed");
    buf
}

/// Write a multi-size ICO file.
/// Each image is expected to be a fully-formed PNG bitstream.
fn write_ico<W: Write>(w: &mut W, images: &[(u32, Vec<u8>)]) -> io::Result<()> {
    let count = images.len() as u16;

    // ICO header
    w.write_all(&0u16.to_le_bytes())?; // Reserved
    w.write_all(&1u16.to_le_bytes())?; // Type: 1 = Icon
    w.write_all(&count.to_le_bytes())?; // Count

    let header_size = 6u32 + count as u32 * 16;
    let mut offset = header_size;

    // ICO directory entries
    for &(size, ref data) in images {
        let w_field = if size >= 256 { 0 } else { size as u8 };
        w.write_all(&[w_field])?; // Width
        w.write_all(&[w_field])?; // Height
        w.write_all(&[0])?; // Colors (0 = >256 colors)
        w.write_all(&[0])?; // Reserved
        w.write_all(&1u16.to_le_bytes())?; // Color planes
        w.write_all(&32u16.to_le_bytes())?; // Bits per pixel
        w.write_all(&(data.len() as u32).to_le_bytes())?; // Size in bytes
        w.write_all(&offset.to_le_bytes())?; // Offset to data
        offset += data.len() as u32;
    }

    // Image data (PNG)
    for &(_, ref data) in images {
        w.write_all(data)?;
    }

    Ok(())
}

fn main() {
    let sizes = [256, 128, 64, 48, 32, 16];
    let mut images = Vec::new();

    for &size in &sizes {
        let img = generate_icon(size);
        let png_data = encode_png(&img);
        images.push((size, png_data));
        println!("Generated {}x{} icon", size, size);
    }

    // Save PNG (256x256 for reference)
    let png_path = Path::new("assets/icon.png");
    std::fs::write(png_path, &images[0].1).expect("failed to save PNG");
    println!("PNG saved to {}", png_path.display());

    // Save ICO
    let ico_path = Path::new("assets/icon.ico");
    let mut ico_file = std::fs::File::create(ico_path).expect("failed to create ICO");
    write_ico(&mut ico_file, &images).expect("failed to write ICO");
    println!("ICO saved to {}", ico_path.display());
}
