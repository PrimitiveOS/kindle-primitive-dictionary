use dictionary_render::View;
use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&output)?;
    let fixture = std::env::temp_dir().join(format!(
        "primitive-dictionary-preview-{}",
        std::process::id()
    ));
    fs::create_dir(&fixture)?;
    let docs = fixture.join("documents");
    fs::create_dir(&docs)?;
    fs::write(docs.join("Vocabulary.tsv"), "term\tdefinition\texample\nsustainable\tAble to continue over time.\tThe group found a sustainable way to share books.\nkiến thức\tknowledge\n")?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut view = View::open(fixture.clone(), docs);
    let mut capture = |view: &View, filename: &str, dark| {
        write_png(
            &output.join(filename),
            WIDTH,
            HEIGHT,
            &frame(view, &mut canvas, dark),
        )
    };
    capture(&view, "dictionary.png", false)?;
    assert_eq!(view.input(Input::Tap(580.0, 474.0)), Change::Full);
    assert_eq!(view.input(Input::Tap(580.0, 1450.0)), Change::Partial);
    capture(&view, "dictionary-definition-dark.png", true)?;
    assert_eq!(view.input(Input::Tap(1098.0, 90.0)), Change::Full);
    assert_eq!(view.input(Input::Tap(1030.0, 205.0)), Change::Full);
    assert_eq!(view.input(Input::Tap(500.0, 475.0)), Change::Full);
    capture(&view, "dictionary-import.png", false)?;
    assert_eq!(view.input(Input::Tap(580.0, 1450.0)), Change::Full);
    assert!(fs::read_to_string(fixture.join("dictionary.json"))?.contains("kiến thức"));
    fs::remove_dir_all(&fixture)?;
    Ok(())
}
