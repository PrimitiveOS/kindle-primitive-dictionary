# Dictionary

**Look up words, read examples, and save useful terms on Kindle.**

Dictionary is an offline PrimitiveOS app. It opens with a small original English starter glossary and supports your own word lists.

- **Search** matches headwords regardless of letter case and shows prefix matches first.
- **Save word** adds a definition to Favorites. **Export for Cards** writes the saved headwords and definitions as a two-column TSV file in Documents.
- **Import** lists UTF-8 TSV files in Documents. Use one word and definition per line, separated by a tab. A third column can hold an example. The import preview shows one entry and the counts of new and duplicate words.

An import can contain up to 768 KiB. Dictionary accepts up to 4,000 imported entries. Readable definitions, favorites, and imported words are available between launches and without Wi-Fi.

## Screenshots

| Search and browse | A saved definition | Import preview |
|---|---|---|
| ![Dictionary showing the first page of words](docs/images/dictionary.png) | ![The definition and example for adapt in dark mode](docs/images/dictionary-definition-dark.png) | ![Preview showing new words in a TSV file](docs/images/dictionary-import.png) |

Screenshots are from the desktop simulator.

## Install and develop

Open **Store** in Launcher, find **Dictionary**, and tap **Get**. See the [installation guide](INSTALL.md) for building an ARMHF package.

With Tools and Launcher beside this checkout, run:

```sh
cargo test --offline --locked --workspace
cargo run --offline --locked -p dictionary-desktop -- build/previews
```

## Licence

Code uses [PolyForm Noncommercial 1.0.0](LICENSE). Project-owned documentation, screenshots, artwork, and the starter glossary use [CC BY-NC-SA 4.0](LICENSE-DOCS).
