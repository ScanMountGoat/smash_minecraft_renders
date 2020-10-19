use image::png::PngDecoder;
use image::DynamicImage;
use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use std::io::Cursor;

/// Creates a Smash Ultimate Minecraft Steve inspired render from the given Minecraft skin texture.
pub fn create_render(minecraft_skin_texture: &RgbaImage) -> RgbaImage {
    let lighting = image::load_from_memory(include_bytes!("../lighting.png"))
        .unwrap()
        .into_rgba();

    // At least 16 bit precision is required for the texture sampling to look decent.
    let uvs_data = Cursor::new(include_bytes!("../uvs.png").to_vec());
    let uvs_decoder = PngDecoder::new(uvs_data).unwrap();
    let uvs = match DynamicImage::from_decoder(uvs_decoder).unwrap() {
        DynamicImage::ImageRgba16(buffer) => buffer,
        _ => panic!("Expected RGBA 16 bit for UVs"),
    };

    sample_texture_apply_lighting(&uvs, &lighting, &minecraft_skin_texture)
}

/// Creates a render with the dimensions and alpha of the reference chara file
/// by transforming the render using the given transformations.
pub fn create_chara_image(
    render: &RgbaImage,
    chara_reference: &RgbaImage,
    scale: f32,
    translate_x: f32,
    translate_y: f32,
) -> RgbaImage {
    let mut output = ImageBuffer::new(
        chara_reference.dimensions().0,
        chara_reference.dimensions().1,
    );

    // warp_into_with defines the preimage, so invert the transformation.
    imageproc::geometric_transformations::warp_into_with(
        &render,
        |x, y| ((x - translate_x) / scale, (y - translate_y) / scale),
        imageproc::geometric_transformations::Interpolation::Bilinear,
        Rgba([0u8, 0u8, 0u8, 0u8]),
        &mut output,
    );

    // Use the reference image's alpha for appropriate masking on some portraits.
    // TODO: There may be a cleaner/more efficient way to do this.
    for x in 0..output.width() {
        for y in 0..output.height() {
            let current = output.get_pixel_mut(x, y);
            let alpha = chara_reference.get_pixel(x, y)[3];
            *current = Rgba([current[0], current[1], current[2], alpha]);
        }
    }

    output
}

fn sample_texture_apply_lighting(
    uvs: &ImageBuffer<Rgba<u16>, Vec<u16>>,
    lighting: &RgbaImage,
    texture_to_sample: &RgbaImage,
) -> RgbaImage {
    // Create the rendered result.
    let mut output = ImageBuffer::new(uvs.dimensions().0, uvs.dimensions().1);

    for x in 0..output.width() {
        for y in 0..output.height() {
            let uv = *uvs.get_pixel(x, y);
            let u = normalize_u16_to_f32(uv[0]);
            let v = normalize_u16_to_f32(uv[1]);
            let alpha = uv[3];

            let (texture_x, texture_y) = interpolate_nearest(
                u,
                1f32 - v,
                texture_to_sample.dimensions().0,
                texture_to_sample.dimensions().0,
            );
            let texture_color = texture_to_sample.get_pixel(texture_x, texture_y);
            let lighting_color = lighting.get_pixel(x, y);

            // The lighting pass is scaled down by a factor of 0.25 to fit into 8 bits per channel.
            // Multiplying by 4 to undo the compression is a bit too bright, so use 2 instead.
            // Perform all calculations in floating point to avoid overflow.
            let r = (texture_color[0] as f32 / 255f32) * (lighting_color[0] as f32 / 255f32) * 2f32;
            let g = (texture_color[1] as f32 / 255f32) * (lighting_color[1] as f32 / 255f32) * 2f32;
            let b = (texture_color[2] as f32 / 255f32) * (lighting_color[2] as f32 / 255f32) * 2f32;

            // Convert back to the proper format for the image.
            let r = to_u8_clamped(r);
            let g = to_u8_clamped(g);
            let b = to_u8_clamped(b);

            *output.get_pixel_mut(x, y) = Rgba([r, g, b, alpha as u8]);
        }
    }

    output
}

fn interpolate_nearest(x: f32, y: f32, width: u32, height: u32) -> (u32, u32) {
    // This isn't really "nearest" neighbor because the lower index is always chosen.
    // TODO: figure out why round() instead of floor() produces artifacts on some regions.
    let x = x * width as f32;
    let y = y * height as f32;

    // Clamp to the edges for out of bounds indices.
    let x = std::cmp::min(x.floor() as u32, width - 1);
    let y = std::cmp::min(y.floor() as u32, height - 1);
    (x, y)
}

fn normalize_u16_to_f32(u: u16) -> f32 {
    // Unsigned normalization.
    // 0u16 -> 0.0f32, 65535u16 -> 1.0f32
    u as f32 / 65535f32
}

fn to_u8_clamped(x: f32) -> u8 {
    let result = x * 255f32;
    if result > 255f32 {
        255u8
    } else {
        result as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_nearest_8x8_edges() {
        assert_eq!(interpolate_nearest(0f32, 0f32, 8u32, 8u32), (0u32, 0u32));
        assert_eq!(interpolate_nearest(0f32, 1f32, 8u32, 8u32), (0u32, 7u32));
        assert_eq!(interpolate_nearest(1f32, 0f32, 8u32, 8u32), (7u32, 0u32));
        assert_eq!(interpolate_nearest(1f32, 1f32, 8u32, 8u32), (7u32, 7u32));
    }

    #[test]
    fn test_interpolate_nearest_out_of_bounds() {
        assert_eq!(interpolate_nearest(-1f32, -1f32, 8u32, 8u32), (0u32, 0u32));
        assert_eq!(interpolate_nearest(0f32, 1.5f32, 8u32, 8u32), (0u32, 7u32));
        assert_eq!(interpolate_nearest(1.5f32, 0f32, 8u32, 8u32), (7u32, 0u32));
        assert_eq!(interpolate_nearest(1.5f32, 1.5f32, 8u32, 8u32), (7u32, 7u32));
    }

    #[test]
    fn test_normalize_u16_to_f32() {
        assert_eq!(normalize_u16_to_f32(0u16), 0f32);
        assert_eq!(normalize_u16_to_f32(65535u16), 1f32);
    }

    #[test]
    fn test_to_u8_clamped() {
        assert_eq!(to_u8_clamped(0f32), 0u8);
        assert_eq!(to_u8_clamped(0.5f32), 127u8);
        assert_eq!(to_u8_clamped(1f32), 255u8);
        assert_eq!(to_u8_clamped(1.01f32), 255u8);
    }
}
