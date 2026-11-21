use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use dictionary_core::{Library, Preview, MAX_BYTES, MAX_SAVE};
use primitive_app::{with_storage, App, Change, WIDTH};
use primitive_ui::{
    back_button, header, keyboard_key, screen_rect, Canvas, Editor, Input, Layout, Press, Rect,
    TextStyle, BODY, CAPTION,
};

enum Screen {
    Search,
    Favorites,
    Files,
    Entry { name: String, from_favorites: bool },
    Preview(Preview),
}

pub struct View {
    directory: PathBuf,
    documents: PathBuf,
    library: Library,
    screen: Screen,
    query: Editor,
    typing: bool,
    page: usize,
    files: Vec<String>,
    notice: String,
}

impl View {
    pub fn open(directory: PathBuf, documents: PathBuf) -> Self {
        let library = load(&directory).unwrap_or_default();
        Self {
            directory,
            documents,
            library,
            screen: Screen::Search,
            query: Editor::new(80),
            typing: false,
            page: 0,
            files: Vec::new(),
            notice: String::new(),
        }
    }

    fn row_count(&self) -> usize {
        if self.typing {
            3
        } else {
            7
        }
    }

    fn select(&mut self, screen: Screen) -> Change {
        self.screen = screen;
        self.page = 0;
        self.typing = false;
        self.notice.clear();
        Change::Full
    }

    fn save_library(&mut self, update: Library) {
        match serde_json::to_vec(&update)
            .map_err(io::Error::other)
            .and_then(|bytes| {
                if bytes.len() > MAX_SAVE {
                    return Err(io::Error::other(
                        "The dictionary storage limit was reached.",
                    ));
                }
                primitive_app::save(&self.directory.join("dictionary.json"), &bytes)
            }) {
            Ok(()) => {
                self.library = update;
                self.notice.clear();
            }
            Err(error) => self.notice = format!("Could not save. {error}"),
        }
    }

    fn refresh_files(&mut self) {
        let result = with_storage(|| {
            let mut names = Vec::new();
            for item in fs::read_dir(&self.documents)? {
                let item = item?;
                if let Ok(name) = item.file_name().into_string() {
                    if name.len() <= 200
                        && item.file_type()?.is_file()
                        && Path::new(&name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("tsv"))
                    {
                        names.push(name);
                    }
                }
                if names.len() == 512 {
                    break;
                }
            }
            names.sort();
            Ok(names)
        });
        match result {
            Ok(files) => self.files = files,
            Err(error) => {
                self.files.clear();
                self.notice = format!("Could not read Documents. {error}");
            }
        }
    }

    fn read_import(&mut self, index: usize) -> Change {
        let Some(name) = self.files.get(index) else {
            return Change::None;
        };
        let result = with_storage(|| bounded_read(&self.documents.join(name), MAX_BYTES));
        let preview = result
            .map_err(|error| format!("Could not open the file. {error}"))
            .and_then(|bytes| self.library.import_preview(&bytes));
        match preview {
            Ok(preview) => self.select(Screen::Preview(preview)),
            Err(error) => {
                self.notice = error;
                Change::Partial
            }
        }
    }

    fn confirm_import(&mut self) -> Change {
        let Screen::Preview(preview) = std::mem::replace(&mut self.screen, Screen::Files) else {
            return Change::None;
        };
        let count = preview.len();
        let mut update = self.library.clone();
        update.apply(preview);
        self.save_library(update);
        if self.notice.is_empty() {
            self.select(Screen::Search);
            self.notice = format!("Imported {count} words.");
        }
        Change::Full
    }

    fn export(&mut self) -> Change {
        let data = self.library.export_favorites();
        if data.is_empty() {
            self.notice = "Save a word before exporting.".into();
        } else {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let name = format!("Dictionary cards {stamp}.tsv");
            self.notice = match primitive_app::save(&self.documents.join(&name), data.as_bytes()) {
                Ok(()) => format!("Saved {name} in Documents."),
                Err(error) => format!("Could not export cards. {error}"),
            };
        }
        Change::Partial
    }

    fn back(&mut self) -> Change {
        match self.screen {
            Screen::Entry {
                from_favorites: true,
                ..
            } => {
                self.screen = Screen::Favorites;
                self.page = self.page.min(self.pages(self.result_count()) - 1);
                self.notice.clear();
                Change::Full
            }
            Screen::Entry { .. } => {
                self.screen = Screen::Search;
                self.notice.clear();
                Change::Full
            }
            Screen::Preview(_) => self.select(Screen::Files),
            _ if self.typing => {
                self.typing = false;
                Change::Full
            }
            _ => Change::Exit,
        }
    }

    fn pages(&self, count: usize) -> usize {
        count.div_ceil(self.row_count()).max(1)
    }

    fn change_page(&mut self, offset: isize, count: usize) -> Change {
        let next = self
            .page
            .saturating_add_signed(offset)
            .min(self.pages(count) - 1);
        if next == self.page {
            return Change::None;
        }
        self.page = next;
        Change::Partial
    }

    fn result_count(&self) -> usize {
        match self.screen {
            Screen::Files => self.files.len(),
            Screen::Favorites => self.library.search("", true).len(),
            Screen::Search => self.library.search(&self.query.string(), false).len(),
            _ => 0,
        }
    }

    fn tap(&mut self, x: f32, y: f32) -> Change {
        let width = WIDTH as i32;
        if back_button(width).contains(x, y) {
            return self.back();
        }
        match &self.screen {
            Screen::Entry { name, .. } => {
                if action(width).contains(x, y) {
                    let mut update = self.library.clone();
                    update.toggle(name);
                    self.save_library(update);
                    return Change::Partial;
                }
                return Change::None;
            }
            Screen::Preview(preview) => {
                return if !preview.is_empty() && action(width).contains(x, y) {
                    self.confirm_import()
                } else {
                    Change::None
                };
            }
            _ => {}
        }

        if tab(width, 0).contains(x, y) {
            return self.select(Screen::Search);
        }
        if tab(width, 1).contains(x, y) {
            return self.select(Screen::Favorites);
        }
        if tab(width, 2).contains(x, y) {
            self.select(Screen::Files);
            self.refresh_files();
            return Change::Full;
        }

        if matches!(self.screen, Screen::Search) && field(width).contains(x, y) {
            self.typing = !self.typing;
            self.page = 0;
            self.notice.clear();
            return Change::Full;
        }
        if matches!(self.screen, Screen::Favorites) && field(width).contains(x, y) {
            return self.export();
        }
        if self.typing {
            let layout = Layout::new(1264, 1680, 4, 5);
            if let Some(key) = keyboard_key(&layout, &self.query, "Done", x, y) {
                self.page = 0;
                return match self.query.press(key) {
                    Press::Go => {
                        self.typing = false;
                        Change::Full
                    }
                    Press::Nothing => Change::None,
                    _ => Change::Partial,
                };
            }
        } else if page_button(width, false).contains(x, y) {
            return self.change_page(-1, self.result_count());
        } else if page_button(width, true).contains(x, y) {
            return self.change_page(1, self.result_count());
        }

        let count = self.row_count();
        let Some(index) = (0..count).find(|i| row(width, *i).contains(x, y)) else {
            return Change::None;
        };
        let index = self.page * count + index;
        if matches!(self.screen, Screen::Files) {
            return self.read_import(index);
        }
        let from_favorites = matches!(self.screen, Screen::Favorites);
        let query = if from_favorites {
            String::new()
        } else {
            self.query.string()
        };
        if let Some(item) = self.library.search(&query, from_favorites).get(index) {
            self.screen = Screen::Entry {
                name: item.term.clone(),
                from_favorites,
            };
            self.typing = false;
            self.notice.clear();
            return Change::Full;
        }
        Change::None
    }

    fn draw_list(&self, canvas: &mut Canvas, width: i32) {
        header(canvas, "Dictionary", "Exit");
        for (i, (label, active)) in [
            ("Search", matches!(self.screen, Screen::Search)),
            ("Favorites", matches!(self.screen, Screen::Favorites)),
            ("Import", matches!(self.screen, Screen::Files)),
        ]
        .into_iter()
        .enumerate()
        {
            canvas.button(tab(width, i as i32), label, active, false);
        }

        let query = self.query.string();
        let empty;
        let rows: Vec<String>;
        match self.screen {
            Screen::Search => {
                let field = field(width);
                canvas.panel(field, 16, 3, 255);
                let title = if query.is_empty() {
                    "Search words"
                } else {
                    query.as_str()
                };
                canvas.text(field.inset(20), title, BODY);
                empty = "No matching words.";
                rows = self
                    .library
                    .search(&query, false)
                    .into_iter()
                    .map(|item| item.term.clone())
                    .collect();
            }
            Screen::Favorites => {
                canvas.button(field(width), "Export for Cards", false, false);
                empty = "Saved words appear here.";
                rows = self
                    .library
                    .search("", true)
                    .into_iter()
                    .map(|item| item.term.clone())
                    .collect();
            }
            Screen::Files => {
                canvas.text(field(width), "Choose a TSV file in Documents.", BODY);
                empty = "No TSV files were found in Documents.";
                rows = self.files.clone();
            }
            _ => return,
        }
        if rows.is_empty() {
            canvas.text(box_at(width, 40, 478, 1184, 80), empty, BODY.center());
        }
        let count = self.row_count();
        for (i, title) in rows.iter().skip(self.page * count).take(count).enumerate() {
            // A tap reverses the row, as the kit's rows do.
            let bounds = row(width, i);
            let pressed = canvas.control_pressed(bounds, false);
            canvas.panel(bounds, 22, 2, if pressed { 0 } else { 255 });
            canvas.text(bounds.inset(22), title, BODY.ink(if pressed { 255 } else { 0 }));
        }
        if self.typing {
            canvas.keyboard(self.query.layer, "Done", None);
        } else if self.pages(rows.len()) > 1 {
            canvas.button(page_button(width, false), "Previous", false, false);
            canvas.button(page_button(width, true), "Next", false, false);
            canvas.text(
                box_at(width, 378, 1444, 508, 75),
                &format!("{} / {}", self.page + 1, self.pages(rows.len())),
                CAPTION.center(),
            );
        }
    }
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        let width = canvas.bounds().w;
        match &self.screen {
            Screen::Entry { name, .. } => {
                header(canvas, "Definition", "Back");
                if let Some(item) = self.library.get(name) {
                    canvas.text(
                        box_at(width, 64, 177, 1136, 156),
                        &item.term,
                        TextStyle::new(58).bold(),
                    );
                    canvas.card(box_at(width, 40, 350, 1184, 560));
                    canvas.text(
                        box_at(width, 82, 395, 1100, 465),
                        &item.definition,
                        TextStyle::new(42),
                    );
                    if !item.example.is_empty() {
                        canvas.text(box_at(width, 64, 945, 1136, 76), "Example", BODY.bold());
                        canvas.text(box_at(width, 64, 1030, 1136, 265), &item.example, BODY);
                    }
                    let label = if self.library.favorited(name) {
                        "Remove from favorites"
                    } else {
                        "Save word"
                    };
                    canvas.button(action(width), label, true, false);
                }
            }
            Screen::Preview(preview) => {
                header(canvas, "Import preview", "Back");
                canvas.text(
                    box_at(width, 40, 230, 1184, 128),
                    &format!(
                        "{} new words · {} duplicates",
                        preview.len(),
                        preview.duplicates
                    ),
                    BODY.bold(),
                );
                if let Some(first) = preview.first() {
                    canvas.card(box_at(width, 40, 420, 1184, 643));
                    canvas.text(
                        box_at(width, 80, 465, 1104, 133),
                        &first.term,
                        TextStyle::new(52).bold(),
                    );
                    canvas.text(box_at(width, 80, 627, 1104, 375), &first.definition, BODY);
                    canvas.button(action(width), "Import words", true, false);
                } else {
                    canvas.text(
                        box_at(width, 40, 423, 1184, 150),
                        "There are no new words to import.",
                        BODY,
                    );
                }
            }
            _ => self.draw_list(canvas, width),
        }
        if !self.typing && !self.notice.is_empty() {
            canvas.text(box_at(width, 40, 1548, 1184, 114), &self.notice, CAPTION);
        }
    }

    fn input(&mut self, input: Input) -> Change {
        match input {
            Input::Tap(x, y) => self.tap(x, y),
            Input::PageBack
                if self.typing
                    || matches!(self.screen, Screen::Entry { .. } | Screen::Preview(_)) =>
            {
                self.back()
            }
            Input::PageBack => self.change_page(-1, self.result_count()),
            Input::PageForward => self.change_page(1, self.result_count()),
            _ => Change::None,
        }
    }

    fn storage_returned(&mut self) -> io::Result<()> {
        if let Some(library) = load(&self.directory) {
            self.library = library;
        }
        if matches!(self.screen, Screen::Files | Screen::Preview(_)) {
            self.select(Screen::Files);
            self.refresh_files();
        }
        self.page = self.page.min(self.pages(self.result_count()) - 1);
        Ok(())
    }
}

fn bounded_read(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("Choose a regular file."));
    }
    let mut buffer = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut buffer)?;
    if buffer.len() > limit {
        return Err(io::Error::other("The file is too large."));
    }
    Ok(buffer)
}

fn load(directory: &Path) -> Option<Library> {
    let bytes = with_storage(|| bounded_read(&directory.join("dictionary.json"), MAX_SAVE)).ok()?;
    let lib: Library = serde_json::from_slice(&bytes).ok()?;
    Some(lib.restored())
}

fn box_at(w: i32, x: i32, y: i32, width: i32, height: i32) -> Rect {
    screen_rect(w, x, y, width, height)
}

fn tab(w: i32, i: i32) -> Rect {
    box_at(w, 40 + 400 * i, 155, 384, 104)
}
fn field(w: i32) -> Rect {
    box_at(w, 40, 280, 1184, 110)
}
fn row(w: i32, i: usize) -> Rect {
    box_at(w, 40, 412 + 142 * i as i32, 1184, 124)
}
fn action(w: i32) -> Rect {
    box_at(w, 40, 1390, 1184, 116)
}
fn page_button(w: i32, next: bool) -> Rect {
    box_at(w, if next { 926 } else { 40 }, 1420, 298, 104)
}
