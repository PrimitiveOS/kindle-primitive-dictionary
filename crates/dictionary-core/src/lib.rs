use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

pub const MAX_BYTES: usize = 768 * 1024;
pub const MAX_ENTRIES: usize = 4000;
pub const MAX_SAVE: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub term: String,
    pub definition: String,
    #[serde(default)]
    pub example: String,
}

fn key(term: &str) -> String {
    term.trim().to_lowercase()
}

fn search_key(term: &str) -> String {
    key(term)
        .chars()
        .filter_map(|ch| {
            Some(match ch {
                'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' | 'ă' | 'ằ'
                | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'ä' | 'å' => 'a',
                'ç' => 'c',
                'đ' => 'd',
                'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' | 'ë' => {
                    'e'
                }
                'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' | 'î' | 'ï' => 'i',
                'ñ' => 'n',
                'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ'
                | 'ớ' | 'ở' | 'ỡ' | 'ợ' | 'ö' => 'o',
                'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' | 'û' | 'ü' => {
                    'u'
                }
                'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' | 'ÿ' => 'y',
                '\u{0300}'..='\u{036f}' => return None,
                _ => ch,
            })
        })
        .collect()
}

fn entry(line: &str) -> Option<Entry> {
    let mut fields = line.split('\t').map(str::trim);
    let term = fields.next()?;
    let definition = fields.next()?;
    let example = fields.next().unwrap_or_default();
    let fits =
        |value: &str, limit| value.chars().count() <= limit && !value.chars().any(char::is_control);
    if term.is_empty()
        || definition.is_empty()
        || fields.next().is_some()
        || !fits(term, 80)
        || !fits(definition, 320)
        || !fits(example, 200)
    {
        return None;
    }
    Some(Entry {
        term: term.into(),
        definition: definition.into(),
        example: example.into(),
    })
}

static STARTER: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();
fn starter() -> &'static BTreeMap<String, Entry> {
    STARTER.get_or_init(|| {
        include_str!("../assets/starter.tsv")
            .lines()
            .map(|line| {
                let entry = entry(line).expect("valid starter entry");
                (key(&entry.term), entry)
            })
            .collect()
    })
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    imported: BTreeMap<String, Entry>,
    favorites: BTreeSet<String>,
}

#[derive(Debug)]
pub struct Preview {
    entries: BTreeMap<String, Entry>,
    pub duplicates: usize,
}

impl Preview {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn first(&self) -> Option<&Entry> {
        self.entries.values().next()
    }
}

impl Library {
    pub fn restored(mut self) -> Self {
        self.imported.retain(|name, value| {
            *name == key(&value.term)
                && !starter().contains_key(name)
                && entry(&format!(
                    "{}\t{}\t{}",
                    value.term, value.definition, value.example
                ))
                .as_ref()
                    == Some(value)
        });
        if self.imported.len() > MAX_ENTRIES {
            return Self::default();
        }
        let imported = &self.imported;
        self.favorites
            .retain(|name| imported.contains_key(name) || starter().contains_key(name));
        self
    }

    pub fn get(&self, term: &str) -> Option<&Entry> {
        let name = key(term);
        self.imported.get(&name).or_else(|| starter().get(&name))
    }

    pub fn search(&self, term: &str, favorites_only: bool) -> Vec<&Entry> {
        let query = key(term);
        let folded_query = search_key(&query);
        let mut results = Vec::new();
        for (name, item) in starter().iter().chain(&self.imported) {
            if favorites_only && !self.favorites.contains(name) {
                continue;
            }
            let folded = search_key(name);
            if !name.contains(&query) && !folded.contains(&folded_query) {
                continue;
            }
            results.push((
                !name.starts_with(&query),
                !folded.starts_with(&folded_query),
                name,
                item,
            ));
        }
        results.sort_unstable_by_key(|(exact, prefix, name, _)| (*exact, *prefix, *name));
        results.into_iter().map(|(_, _, _, item)| item).collect()
    }

    pub fn favorited(&self, term: &str) -> bool {
        self.favorites.contains(&key(term))
    }

    pub fn toggle(&mut self, term: &str) {
        let name = key(term);
        if self.get(&name).is_some() && !self.favorites.remove(&name) {
            self.favorites.insert(name);
        }
    }

    pub fn import_preview(&self, bytes: &[u8]) -> Result<Preview, String> {
        if bytes.len() > MAX_BYTES {
            return Err("That file is larger than 768 KiB.".into());
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| "Use a UTF-8 text file.")?
            .trim_start_matches('\u{feff}');
        let mut preview = Preview {
            entries: BTreeMap::new(),
            duplicates: 0,
        };
        for (row, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if row == 0
                && (line.eq_ignore_ascii_case("term\tdefinition")
                    || line.eq_ignore_ascii_case("term\tdefinition\texample"))
            {
                continue;
            }
            let item = entry(line).ok_or_else(|| {
                format!(
                    "Line {} needs a term and definition separated by a tab.",
                    row + 1
                )
            })?;
            let name = key(&item.term);
            if self.get(&name).is_some() || preview.entries.contains_key(&name) {
                preview.duplicates += 1;
                continue;
            }
            if self.imported.len() + preview.entries.len() >= MAX_ENTRIES {
                return Err("The dictionary can hold up to 4,000 imported words.".into());
            }
            preview.entries.insert(name, item);
        }
        Ok(preview)
    }

    pub fn apply(&mut self, preview: Preview) {
        self.imported.extend(preview.entries);
    }

    pub fn export_favorites(&self) -> String {
        self.favorites
            .iter()
            .filter_map(|name| self.get(name))
            .map(|item| format!("{}\t{}\n", item.term, item.definition))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_previews_unicode_and_duplicates_without_changing_the_library() {
        let mut lib = Library::default();
        let draft = lib
            .import_preview(
                "term\tdefinition\nAdapt\tDuplicate\nkiến thức\tknowledge\nKIẾN THỨC\tduplicate\n"
                    .as_bytes(),
            )
            .unwrap();
        assert_eq!(draft.len(), 1);
        assert_eq!(draft.duplicates, 2);
        assert!(lib.get("kiến thức").is_none());
        lib.apply(draft);
        assert_eq!(lib.search("KIẾN", false)[0].definition, "knowledge");
        assert_eq!(lib.search("kien thuc", false)[0].term, "kiến thức");
        lib.toggle("Kiến thức");
        assert_eq!(lib.export_favorites(), "kiến thức\tknowledge\n");
    }

    #[test]
    fn broken_input_never_partially_imports() {
        let lib = Library::default();
        assert!(lib
            .import_preview(b"first\tok\nbroken")
            .unwrap_err()
            .contains("Line 2"));
        assert!(lib.import_preview(&[255]).is_err());
        assert!(lib.import_preview(&vec![b' '; MAX_BYTES + 1]).is_err());
        assert!(lib.get("first").is_none());
    }
}
