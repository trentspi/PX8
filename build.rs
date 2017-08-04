use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::io;

fn prebuild() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("parameters.rs");

    let map_width = match env::var_os("PX8_MAP_WIDTH") {
        Some(v) => v.into_string().unwrap(),
        None => "128".to_string(),
    };

    let map_height = match env::var_os("PX8_MAP_HEIGHT") {
        Some(v) => v.into_string().unwrap(),
        None => "32".to_string(),
    };

    let mut f = File::create(&dest_path).unwrap();

    f.write_all(format!("pub const MAP_WIDTH: usize = {:?};\n",
                           map_width.parse::<u32>().unwrap())
                           .as_bytes())
        .unwrap();
    f.write_all(format!("pub const MAP_HEIGHT: usize = {:?};\n",
                           map_height.parse::<u32>().unwrap())
                           .as_bytes())
        .unwrap();
    f.write_all(format!("pub const VERSION: u32 = 0;\n").as_bytes())
        .unwrap();
    f.write_all(format!("pub const MAJOR_VERSION: u32 = 0;\n").as_bytes())
        .unwrap();
    f.write_all(format!("pub const MINOR_VERSION: u32 = 5;\n").as_bytes())
        .unwrap();

    Ok(())
}

fn main() {
    match prebuild() {
        Err(e) => panic!("Error: {}", e),
        Ok(()) => (),
    }
}
