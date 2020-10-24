use image::DynamicImage;
use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use imageproc::geometric_transformations::warp_into_with;
use imageproc::geometric_transformations::Interpolation;
use std::cmp::min;

/// Creates a Smash Ultimate Minecraft Steve inspired render from the given Minecraft skin texture.
pub fn create_render(minecraft_skin_texture: &RgbaImage) -> RgbaImage {
    let lighting = image::load_from_memory(include_bytes!("../lighting.png"))
        .unwrap()
        .into_rgba();

    // At least 16 bit precision is required for the texture sampling to look decent.
    let uvs = match image::load_from_memory(include_bytes!("../uvs.png")).unwrap() {
        DynamicImage::ImageRgba16(buffer) => buffer,
        _ => panic!("Expected RGBA 16 bit for UVs"),
    };

    sample_texture_apply_lighting(&minecraft_skin_texture, &uvs, &lighting)
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

    // Align the render with the target chara image.
    // warp_into_with defines the preimage, so invert the transformation.
    warp_into_with(
        &render,
        |x, y| ((x - translate_x) / scale, (y - translate_y) / scale),
        Interpolation::Bilinear,
        Rgba([0u8, 0u8, 0u8, 0u8]),
        &mut output,
    );

    // Use the reference image's alpha for appropriate masking on some portraits.
    copy_alpha(&mut output, &chara_reference);

    output
}

fn copy_alpha(target: &mut RgbaImage, source: &RgbaImage) {
    // TODO: There may be a cleaner/more efficient way to do this.
    for x in 0..target.width() {
        for y in 0..target.height() {
            let current = target.get_pixel_mut(x, y);
            let alpha = source.get_pixel(x, y)[3];
            *current = Rgba([current[0], current[1], current[2], alpha]);
        }
    }
}

fn sample_texture_apply_lighting(
    texture: &RgbaImage,
    uvs: &ImageBuffer<Rgba<u16>, Vec<u16>>,
    lighting: &RgbaImage,
) -> RgbaImage {
    let mut output = ImageBuffer::new(uvs.dimensions().0, uvs.dimensions().1);

    for x in 0..output.width() {
        for y in 0..output.height() {
            *output.get_pixel_mut(x, y) = calculate_render_pixel(x, y, uvs, lighting, texture);
        }
    }

    output
}

fn calculate_render_pixel(
    x: u32,
    y: u32,
    uvs: &ImageBuffer<Rgba<u16>, Vec<u16>>,
    lighting: &RgbaImage,
    texture: &RgbaImage,
) -> Rgba<u8> {
    let uv = uvs.get_pixel(x, y);
    let (u, v, _) = normalize_rgb_u16(uv);
    let alpha = uv[3];

    // Flip v to transform from an origin at the bottom left (OpenGL) to top left (image).
    let (tex_width, tex_height) = texture.dimensions();
    let (texture_x, texture_y) = interpolate_nearest(u, 1f32 - v, tex_width, tex_height);

    // Perform all calculations in floating point to avoid overflow.
    let (tex_r, tex_g, tex_b) = normalize_rgb_u8(texture.get_pixel(texture_x, texture_y));
    let (light_r, light_g, light_b) = normalize_rgb_u8(lighting.get_pixel(x, y));

    // The lighting pass is scaled down by a factor of 0.25 to fit into 8 bits per channel.
    // Multiplying by 4 is a bit too bright, so use 2 instead.
    let apply_lighting = |color: f32, light: f32| color * light * 2f32;
    let get_result = |color, light| to_u8_clamped(apply_lighting(color, light));

    Rgba([
        get_result(tex_r, light_r),
        get_result(tex_g, light_g),
        get_result(tex_b, light_b),
        alpha as u8,
    ])
}

fn interpolate_nearest(x: f32, y: f32, width: u32, height: u32) -> (u32, u32) {
    // Nearest neighbor interpolation often performs some sort of rounding.
    // UVs are snapped to pixel corners in the exported UV map, so just floor the UVs instead.
    // Clamp to the edges for out of bounds indices.
    let nearest = |f: f32, max_val: u32| min((f * max_val as f32).floor() as u32, max_val - 1);
    (nearest(x, width), nearest(y, height))
}

fn normalize_rgb_u8(pixel: &Rgba<u8>) -> (f32, f32, f32) {
    // 0u16 -> 0.0f32, 65535u16 -> 1.0f32
    let normalize = |u| u as f32 / 255f32;
    (
        normalize(pixel[0]),
        normalize(pixel[1]),
        normalize(pixel[2]),
    )
}

fn normalize_rgb_u16(pixel: &Rgba<u16>) -> (f32, f32, f32) {
    // 0u16 -> 0.0f32, 65535u16 -> 1.0f32
    let normalize = |u| u as f32 / 65535f32;
    (
        normalize(pixel[0]),
        normalize(pixel[1]),
        normalize(pixel[2]),
    )
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
        assert_eq!(
            interpolate_nearest(1.5f32, 1.5f32, 8u32, 8u32),
            (7u32, 7u32)
        );
    }

    #[test]
    fn test_normalize_u8() {
        assert_eq!(
            normalize_rgb_u8(&Rgba([0u8, 0u8, 0u8, 0u8])),
            (0f32, 0f32, 0f32)
        );
        assert_eq!(
            normalize_rgb_u8(&Rgba([255u8, 255u8, 255u8, 255u8])),
            (1f32, 1f32, 1f32)
        );
    }

    #[test]
    fn test_normalize_u16() {
        assert_eq!(
            normalize_rgb_u16(&Rgba([0u16, 0u16, 0u16, 0u16])),
            (0f32, 0f32, 0f32)
        );
        assert_eq!(
            normalize_rgb_u16(&Rgba([65535u16, 65535u16, 65535u16, 65535u16])),
            (1f32, 1f32, 1f32)
        );
    }

    #[test]
    fn test_to_u8_clamped() {
        assert_eq!(to_u8_clamped(0f32), 0u8);
        assert_eq!(to_u8_clamped(0.5f32), 127u8);
        assert_eq!(to_u8_clamped(1f32), 255u8);
        assert_eq!(to_u8_clamped(1.01f32), 255u8);
    }
}
