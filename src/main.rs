fn main() {
    let args: Vec<String> = std::env::args().collect();

    let texture_path = std::path::Path::new(&args[1]);
    let minecraft_skin_texture = image::open(texture_path).unwrap().into_rgba();

    let output = minecraft_render::create_render(&minecraft_skin_texture);

    // Create UI renders from the output render.
    // The transformations are hardcoded based on the output render resolution.
    // The final render is scaled down to match the appropriate sizes.
    let chara_3 = image::load_from_memory(include_bytes!("../chara_3_pickel_00.png"))
        .unwrap()
        .into_rgba();

    let chara_3_custom = minecraft_render::create_chara_image(
        &output,
        &chara_3,
        0.64225626f32,
        -456.55612f32,
        11.757321f32,
    );

    let chara_4 = image::load_from_memory(include_bytes!("../chara_4_pickel_00.png"))
        .unwrap()
        .into_rgba();
    let chara_4_custom = minecraft_render::create_chara_image(
        &output,
        &chara_4,
        0.116441004f32,
        -90.16959f32,
        9.084564f32,
    );

    let chara_6 = image::load_from_memory(include_bytes!("../chara_6_pickel_00.png"))
        .unwrap()
        .into_rgba();
    let chara_6_custom = minecraft_render::create_chara_image(
        &output,
        &chara_6,
        0.469014f32,
        -480.87906f32,
        -96.13269f32,
    );

    chara_3_custom.save("chara_3_custom.png").unwrap();
    chara_4_custom.save("chara_4_custom.png").unwrap();
    chara_6_custom.save("chara_6_custom.png").unwrap();

    output.save("output.png").unwrap();
}
