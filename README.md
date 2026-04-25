# bear-rs

Rust library for reading and writing [Bear](https://bear.app) notes directly via the local SQLite database.

No network, no CloudKit, no Bear.app process required. Works while Bear is running or closed.

## Requirements

- macOS
- Bear installed (the database lives in Bear's app group container)

## Installation

```toml
[dependencies]
bear-rs = "0.1"
```

## Usage

```rust
use bear_rs::{SqliteStore, Note};

// Read
let store = SqliteStore::open_ro()?;
let notes = store.list_notes(&Default::default())?;
let note = store.get_note(Some("7E635AD3-..."), None, true, true)?;

// Write
let store = SqliteStore::open_rw()?;
let note = store.create_note("# My Note\n\nHello world", &["work", "rust"], false)?;
store.append_to_note(Some(&note.id), None, "more text", Default::default(), true, Default::default())?;
store.trash_note(Some(&note.id), None)?;
```

After any write, `bear-rs` posts the Darwin notification `net.shinyfrog.bear.cli.didRequestRefresh` so Bear's UI refreshes automatically.

## API

### `SqliteStore`

Open with `SqliteStore::open_ro()` for reads or `SqliteStore::open_rw()` for writes.

**Reading**

| Method | Description |
|---|---|
| `list_notes(input)` | List notes with optional tag filter, sort, and limit |
| `get_note(id, title, attachments, pins)` | Fetch a single note by ID or title |
| `cat_note(id, title, offset, limit)` | Raw note text with optional pagination |
| `search_notes(query, limit)` | Bear search syntax (`@todo`, `#tag`, `-negation`, etc.) |
| `search_in_note(id, title, string, ignore_case)` | Line matches within a single note |
| `list_tags(id, title)` | All tags, or tags for one note |
| `list_pins(id, title)` | Pin contexts for one or all notes |
| `list_attachments(id, title)` | Attachments for a note |
| `read_attachment(id, title, filename)` | Raw attachment bytes |

**Writing**

| Method | Description |
|---|---|
| `create_note(text, tags, if_not_exists)` | Create a note; title extracted from first line |
| `append_to_note(id, title, content, position, update_modified, tag_position)` | Append or prepend text |
| `write_note(id, title, content, base_hash)` | Overwrite note content; optional hash guard |
| `edit_note(id, title, ops)` | Find/replace operations |
| `trash_note(id, title)` | Move to trash |
| `archive_note(id, title)` | Archive |
| `restore_note(id, title)` | Restore from trash or archive |
| `add_tags(id, title, tags)` | Add tags |
| `remove_tags(id, title, tags)` | Remove tags |
| `rename_tag(old, new, force)` | Rename tag across all notes |
| `delete_tag(name)` | Delete tag and remove from all notes |
| `add_pins(id, title, contexts)` | Pin in contexts (`"global"` or tag name) |
| `remove_pins(id, title, contexts)` | Unpin |
| `add_attachment(id, title, filename, data)` | Attach a file |
| `delete_attachment(id, title, filename)` | Mark attachment unused |

### Search syntax

`search_notes` accepts Bear's query syntax:

```
@today        modified today
@todo         has incomplete todos
@pinned       pinned notes
#tag          has tag
-word         does not contain word
"exact phrase"
@lastNdays    modified in last N days
@date(YYYY-MM-DD)
```

### Export

```rust
use bear_rs::export::{ExportNote, export_notes};

let notes: Vec<ExportNote> = store.list_notes(&Default::default())?
    .into_iter()
    .map(Into::into)
    .collect();

export_notes("./output".as_ref(), &notes, true, true)?;
```

## Development

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
