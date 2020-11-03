use image::DynamicImage;
use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use imageproc::geometric_transformations::warp_into_with;
use imageproc::geometric_transformations::Interpolation;
use std::cmp::{max, min};

pub mod modern_skin;

/// Creates a Smash Ultimate Minecraft Steve inspired render from the given Minecraft skin texture.
pub fn create_render(skin_texture: &RgbaImage) -> RgbaImage {
    // At least 16 bit precision is required for the texture sampling to look decent.
    let load_rgba_u16 = |buffer| match image::load_from_memory(buffer).unwrap() {
        DynamicImage::ImageRgba16(image_buffer) => image_buffer,
        _ => panic!("Expected RGBA 16 bit for UVs"),
    };

    let head_uvs = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/head.png"));
    let chest_uvs = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/chest.png"));
    let arm_l_uvs = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/arm_l.png"));
    let arm_r_uvs = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/arm_r.png"));
    let leg_rl_uvs = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/leg_rl.png"));

    let mut output = ImageBuffer::new(head_uvs.dimensions().0, head_uvs.dimensions().1);

    // Alpha blending relies on having the correct color already present in the render buffer.
    // Steve has simple geometry, so blend layers from back to front rather than using a depth map.
    blend_layer_with_base(&mut output, &leg_rl_uvs, skin_texture);
    blend_layer_with_base(&mut output, &arm_l_uvs, skin_texture);
    blend_layer_with_base(&mut output, &head_uvs, skin_texture);
    blend_layer_with_base(&mut output, &chest_uvs, skin_texture);

    // Skip costly image loading and blending for regions with fully transparent pixels.
    // Assume the base layers are always used.
    if has_pixel_in_region(&skin_texture, 0.75f32, 1.0f32, 0.75f32, 1.0f32) {
        let arm_l_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/arm_l2.png"));
        blend_layer_with_base(&mut output, &arm_l_uvs2, skin_texture);
    }

    if has_pixel_in_region(&skin_texture, 0.25f32, 0.625f32, 0.5f32, 0.75f32) {
        let chest_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/chest2.png"));
        blend_layer_with_base(&mut output, &chest_uvs2, skin_texture);
    }

    if has_pixel_in_region(&skin_texture, 0.5f32, 1.0f32, 0.0f32, 0.25f32) {
        let head_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/head2.png"));
        blend_layer_with_base(&mut output, &head_uvs2, skin_texture);
    }

    if has_pixel_in_region(&skin_texture, 0.0f32, 0.25f32, 0.75f32, 1.0f32) {
        let leg_l_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/leg_l2.png"));
        blend_layer_with_base(&mut output, &leg_l_uvs2, skin_texture);
    }

    if has_pixel_in_region(&skin_texture, 0.0f32, 0.25f32, 0.5f32, 0.75f32) {
        let leg_r_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/leg_r2.png"));
        blend_layer_with_base(&mut output, &leg_r_uvs2, skin_texture);
    }

    blend_layer_with_base(&mut output, &arm_r_uvs, skin_texture);

    if has_pixel_in_region(&skin_texture, 0.625f32, 0.875f32, 0.5f32, 0.75f32) {
        let arm_r_uvs2 = load_rgba_u16(include_bytes!("../images/uv_lighting_alpha/arm_r2.png"));
        blend_layer_with_base(&mut output, &arm_r_uvs2, skin_texture);
    }

    output
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
    blend_alpha(&mut output, &chara_reference);

    output
}

/// Converts a color from Minecraft to match Smash ultimate using the following formula:
/// `ultimate = (minecraft ^ (1.0 / 0.72)) * 0.72`
pub fn color_correct(color: &Rgba<u8>) -> Rgba<u8> {
    let reduce_contrast = |c: f32| c.powf(0.72f32) * 0.72f32;
    let (r, g, b, _) = normalize_rgba_u8(color);
    Rgba([
        to_u8_clamped(reduce_contrast(r)),
        to_u8_clamped(reduce_contrast(g)),
        to_u8_clamped(reduce_contrast(b)),
        color[3],
    ])
}

fn blend_layer_with_base(
    base: &mut RgbaImage,
    layer_uvs_lighting: &ImageBuffer<Rgba<u16>, Vec<u16>>,
    texture: &RgbaImage,
) {
    for x in 0..base.width() {
        for y in 0..base.height() {
            // Skip pixels outside the masked region to improve performance.
            let uv_rgba = layer_uvs_lighting.get_pixel(x, y);
            if uv_rgba[3] == 0u16 {
                continue;
            }

            // Skip fully transparent sampled texels to improve performance.
            let (u, v, lighting, uv_alpha) = normalize_rgba_u16(uv_rgba);
            let layer_color = sample_texture(texture, u, v);
            if layer_color[3] == 0u8 {
                continue;
            }

            let (layer_r, layer_g, layer_b, layer_alpha) = normalize_rgba_u8(layer_color);

            // The lighting pass is scaled down by a factor of 0.25 to fit into 8 bits per channel.
            // Multiplying by 4 is a bit too bright, so use 2 instead.
            let apply_lighting = |color: f32, light: f32| color * light * 2f32;

            let get_result = |base: f32, layer: f32| {
                let lighting_result = apply_lighting(layer, lighting);

                // Skip the costly floating point gamma correction and blending if possible.
                if layer_color[3] < 255u8 {
                    alpha_blend(base, lighting_result, layer_alpha * uv_alpha)
                } else {
                    lighting_result
                }
            };

            let (base_r, base_g, base_b, base_a) = normalize_rgba_u8(base.get_pixel(x, y));

            // Use the uv map alpha as well to prevent blending outside the masked region.
            let r = get_result(base_r, layer_r);
            let g = get_result(base_g, layer_g);
            let b = get_result(base_b, layer_b);
            let alpha_final = base_a + layer_alpha * uv_alpha;

            *base.get_pixel_mut(x, y) = Rgba([
                to_u8_clamped(r),
                to_u8_clamped(g),
                to_u8_clamped(b),
                to_u8_clamped(alpha_final),
            ]);
        }
    }
}

fn has_pixel_in_region(
    image: &RgbaImage,
    x_start: f32,
    x_end: f32,
    y_start: f32,
    y_end: f32,
) -> bool {
    let x_start = (x_start * image.width() as f32) as u32;
    let x_end = (x_end * image.width() as f32) as u32;

    let y_start = (y_start * image.height() as f32) as u32;
    let y_end = (y_end * image.height() as f32) as u32;

    for x in x_start..x_end {
        for y in y_start..y_end {
            if image.get_pixel(x, y)[3] > 0u8 {
                return true;
            }
        }
    }

    false
}

fn blend_alpha(current: &mut RgbaImage, reference: &RgbaImage) {
    // Use the alpha of current with reference as a mask.
    for x in 0..current.width() {
        for y in 0..current.height() {
            let current = current.get_pixel_mut(x, y);
            let target = reference.get_pixel(x, y);
            *current = Rgba([
                current[0],
                current[1],
                current[2],
                min(current[3], target[3]),
            ]);
        }
    }
}

fn alpha_blend(val1: f32, val2: f32, alpha: f32) -> f32 {
    // Gamma correct to ensure the blending result is more accurate.
    let val1_gamma_corrected = val1.powf(2.2f32);
    let val2_gamma_corrected = val2.powf(2.2f32);
    let result = val1_gamma_corrected * (1f32 - alpha) + val2_gamma_corrected * alpha;
    result.powf(1.0f32 / 2.2f32)
}

fn sample_texture(image: &RgbaImage, u: f32, v: f32) -> &Rgba<u8> {
    // Flip v to transform from an origin at the bottom left (OpenGL) to top left (image).
    let (x, y) = interpolate_nearest(u, 1f32 - v, image.dimensions().0, image.dimensions().1);
    image.get_pixel(x, y)
}

fn interpolate_nearest(x: f32, y: f32, width: u32, height: u32) -> (u32, u32) {
    // Clamp to the edges for out of bounds indices.
    // Clamp to 0.0f32 before casting to avoid underflow.
    let nearest = |f: f32, max_val: u32| {
        let val = (f * max_val as f32 - 0.5f32).round();
        if val < 0f32 {
            0u32
        } else {
            min(val as u32, max_val - 1)
        }
    };
    
    (nearest(x, width), nearest(y, height))
}

fn normalize_rgba_u8(pixel: &Rgba<u8>) -> (f32, f32, f32, f32) {
    // 0u16 -> 0.0f32, 65535u16 -> 1.0f32
    let normalize = |u| u as f32 / 255f32;
    (
        normalize(pixel[0]),
        normalize(pixel[1]),
        normalize(pixel[2]),
        normalize(pixel[3]),
    )
}

fn normalize_rgba_u16(pixel: &Rgba<u16>) -> (f32, f32, f32, f32) {
    // 0u16 -> 0.0f32, 65535u16 -> 1.0f32
    let normalize = |u| u as f32 / 65535f32;
    (
        normalize(pixel[0]),
        normalize(pixel[1]),
        normalize(pixel[2]),
        normalize(pixel[3]),
    )
}

fn to_u8_clamped(x: f32) -> u8 {
    // Pick the nearest integer so values close to 1.0 are still converted to 255u8.
    let result = (x * 255f32).round();
    if result < 0.0f32 {
        return 0u8;
    } else if result > 255f32 {
        return 255u8;
    } else {
        return result as u8;
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
            normalize_rgba_u8(&Rgba([0u8, 0u8, 0u8, 0u8])),
            (0f32, 0f32, 0f32, 0f32)
        );
        assert_eq!(
            normalize_rgba_u8(&Rgba([255u8, 255u8, 255u8, 255u8])),
            (1f32, 1f32, 1f32, 1f32)
        );
    }

    #[test]
    fn test_color_correct() {
        assert_eq!(
            color_correct(&Rgba([0u8, 0u8, 0u8, 0u8])),
            Rgba([0u8, 0u8, 0u8, 0u8])
        );
        assert_eq!(
            color_correct(&Rgba([128u8, 128u8, 128u8, 13u8])),
            Rgba([112u8, 112u8, 112u8, 13u8])
        );
        assert_eq!(
            color_correct(&Rgba([255u8, 255u8, 255u8, 255u8])),
            Rgba([184u8, 184u8, 184u8, 255u8])
        );
    }

    #[test]
    fn test_normalize_u16() {
        assert_eq!(
            normalize_rgba_u16(&Rgba([0u16, 0u16, 0u16, 0u16])),
            (0f32, 0f32, 0f32, 0f32)
        );
        assert_eq!(
            normalize_rgba_u16(&Rgba([65535u16, 65535u16, 65535u16, 65535u16])),
            (1f32, 1f32, 1f32, 1f32)
        );
    }

    #[test]
    fn test_to_u8_clamped() {
        assert_eq!(to_u8_clamped(0.999f32), 255u8);
        assert_eq!(to_u8_clamped(-1.5f32), 0u8);
        assert_eq!(to_u8_clamped(0f32), 0u8);
        assert_eq!(to_u8_clamped(0.5f32), 128u8);
        assert_eq!(to_u8_clamped(1f32), 255u8);
        assert_eq!(to_u8_clamped(1.01f32), 255u8);
    }
}
