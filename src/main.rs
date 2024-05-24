use clap::{App, Arg};
use std::time::Instant;

fn main() {
    // TODO: Create better argument names.
    let matches = App::new("minecraft_render")
        .version("0.1")
        .author("SMG")
        .about("Create Smash Ultimate Steve UI from Minecraft skin textures")
        .arg(
            Arg::with_name("skin")
                .short("s")
                .long("skin")
                .value_name("sample.png")
                .help("the Minecraft skin texture")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("is_legacy")
                .short("l")
                .long("legacy")
                .help("convert 2:1 skins (pre Minecraft v1.8) to 1:1 aspect ratio")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("color_correct")
                .short("c")
                .long("colorcorrect")
                .help("levels adjustment to match Smash Ultimate")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("is_slim")
                .long("slim")
                .help("render as slim character")
                .takes_value(false)
        )
        .get_matches();

    let texture_path = matches.value_of("skin").unwrap();
    let mut skin_texture = image::open(texture_path).unwrap().into_rgba();
    if matches.is_present("is_legacy") {
        skin_texture = minecraft_render::modern_skin::convert_to_modern_skin(&skin_texture);
    }

    if matches.is_present("color_correct") {
        for pixel in skin_texture.pixels_mut() {
            *pixel = minecraft_render::color_correct(pixel);
        }
    }

    let start_time = Instant::now();

    let output = 
        if matches.is_present("is_slim") {
            minecraft_render::create_render_slim(&skin_texture)
        } else {
            minecraft_render::create_render(&skin_texture)
        };

    let elapsed = start_time.elapsed();
    eprintln!("Create Render: {:?}", elapsed);

    // Create UI renders from the output render.
    // The transformations are hardcoded based on the output render resolution.
    // The final render is scaled down to match the appropriate sizes.
    let chara_3 = image::load_from_memory(include_bytes!("../images/masks/chara_3_mask.png"))
        .unwrap()
        .into_rgba();

    let chara_3_custom = minecraft_render::create_chara_image(
        &output,
        &chara_3,
        1.28451252f32,
        -456.55612f32,
        11.757321f32,
    );

    let chara_4 = image::load_from_memory(include_bytes!("../images/masks/chara_4_mask.png"))
        .unwrap()
        .into_rgba();
    let chara_4_custom = minecraft_render::create_chara_image(
        &output,
        &chara_4,
        0.232882008f32,
        -90.16959f32,
        9.084564f32,
    );

    let chara_6 = image::load_from_memory(include_bytes!("../images/masks/chara_6_mask.png"))
        .unwrap()
        .into_rgba();
    let chara_6_custom = minecraft_render::create_chara_image(
        &output,
        &chara_6,
        0.938028f32,
        -480.87906f32,
        -96.13269f32,
    );

    chara_3_custom.save("chara_3_custom.png").unwrap();
    chara_4_custom.save("chara_4_custom.png").unwrap();
    chara_6_custom.save("chara_6_custom.png").unwrap();

    output.save("output.png").unwrap();
}
