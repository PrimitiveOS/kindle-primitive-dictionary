use std::path::PathBuf;

use dictionary_render::View;

fn main() -> Result<(), String> {
    let directory = std::env::var_os("PRIMITIVE_APP_DIR")
        .map_or_else(|| PathBuf::from("/mnt/us/dictionary"), PathBuf::from);
    primitive_app::run(
        View::open(directory.clone(), PathBuf::from("/mnt/us/documents")),
        &directory,
    )
}
