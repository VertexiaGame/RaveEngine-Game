use naga::front::wgsl::parse_str;
use naga::valid::{Capabilities, ValidationFlags, Validator};
//Wow.
#[test]
fn clouds_shaders_parse_and_validate() {
    let common = include_str!("../src/client/sky/clouds/shaders/common.wgsl")
        .trim_start_matches('\u{FEFF}')
        .lines()
        .filter(|l| !l.contains("#define_import_path"))
        .collect::<Vec<_>>()
        .join("\n");

    let main = include_str!("../src/client/sky/clouds/shaders/clouds_compute.wgsl")
        .replace("#import bevy_open_world::common", &common)
        .replace("common::", "");

    let module = parse_str(&main).expect("WGSL parse failed");
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("WGSL validation failed");
}
