use bear_rs::{
    SqliteStore,
    model::InsertPosition,
    store::{EditOp, ListInput},
};

fn section(name: &str) {
    println!("\n── {} {}", name, "─".repeat(50 - name.len()));
}

fn main() -> anyhow::Result<()> {
    // ── list / search ─────────────────────────────────────────────────────────

    section("reads");
    let store = SqliteStore::open_ro()?;

    let notes = store.list_notes(&ListInput::default())?;
    println!("  {:<30} {} notes", "list_notes (default)", notes.len());
    assert!(!notes.is_empty(), "expected at least one note");

    let limited = store.list_notes(&ListInput {
        limit: Some(1),
        include_tags: true,
        ..Default::default()
    })?;
    println!("  {:<30} {} note", "list_notes (limit=1)", limited.len());
    assert_eq!(limited.len(), 1);

    let first = &notes[0];
    let note = store.get_note(Some(&first.id), None, true, true)?;
    println!(
        "  {:<30} title={:?} tags={:?} attachments={} pins={:?}",
        "get_note (id)",
        note.title,
        note.tags,
        note.attachments.len(),
        note.pinned_in_tags
    );

    let by_title = store.get_note(None, Some(&note.title), false, false)?;
    assert_eq!(
        by_title.id, note.id,
        "get_note by title returned wrong note"
    );
    println!("  {:<30} ok", "get_note (--title)");

    let cat = store.cat_note(Some(&first.id), None, None, Some(40))?;
    println!("  {:<30} {:?}", "cat_note (limit=40)", cat);

    let cat_offset = store.cat_note(Some(&first.id), None, Some(10), Some(20))?;
    println!("  {:<30} {:?}", "cat_note (offset=10 limit=20)", cat_offset);

    let search_results = store.search_notes("@todo", None)?;
    println!(
        "  {:<30} {} notes",
        "search_notes (@todo)",
        search_results.len()
    );

    let search_limited = store.search_notes("@todo", Some(1))?;
    assert!(search_limited.len() <= 1);
    println!(
        "  {:<30} {} note",
        "search_notes (@todo limit=1)",
        search_limited.len()
    );

    let tags = store.list_tags(None, None)?;
    println!("  {:<30} {} tags", "list_tags (all)", tags.len());

    let pins = store.list_pins(None, None)?;
    println!("  {:<30} {} pins", "list_pins (all)", pins.len());

    let atts = store.list_attachments(Some(&first.id), None)?;
    println!("  {:<30} {} attachments", "list_attachments", atts.len());

    // ── setup: create note ────────────────────────────────────────────────────

    section("create / write");
    let store = SqliteStore::open_rw()?;

    let note = store.create_note(
        "# bear-rs smoke test\n\nOriginal body.",
        &["bear-rs-test"],
        false,
    )?;
    println!("  {:<30} id={}", "create_note", note.id);
    assert_eq!(note.title, "bear-rs smoke test");
    assert_eq!(note.tags, vec!["bear-rs-test"]);

    // if_not_exists returns existing note
    let same = store.create_note("# bear-rs smoke test\n\nDuplicate.", &[], true)?;
    assert_eq!(
        same.id, note.id,
        "if_not_exists should return existing note"
    );
    println!(
        "  {:<30} ok (returned same id)",
        "create_note if_not_exists"
    );

    // ── write_note ────────────────────────────────────────────────────────────

    store.write_note(
        Some(&note.id),
        None,
        "# bear-rs smoke test\n\nRewritten.",
        None,
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(
        after.text.contains("Rewritten"),
        "write_note did not update text"
    );
    println!("  {:<30} {:?}", "write_note", after.text);

    // write_note with correct base hash
    let hash = after.hash();
    store.write_note(
        Some(&note.id),
        None,
        "# bear-rs smoke test\n\nHash-guarded write.",
        Some(&hash),
    )?;
    println!("  {:<30} ok", "write_note (base hash)");

    // write_note with wrong hash should fail
    let bad = store.write_note(Some(&note.id), None, "bad", Some("deadbeef"));
    assert!(bad.is_err(), "write_note with wrong hash should fail");
    println!("  {:<30} correctly rejected", "write_note (bad hash)");

    // ── append ────────────────────────────────────────────────────────────────

    section("append");
    store.append_to_note(
        Some(&note.id),
        None,
        "Appended at end.",
        InsertPosition::End,
        true,
        Default::default(),
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(
        after.text.ends_with("Appended at end."),
        "append End failed: {:?}",
        after.text
    );
    println!("  {:<30} {:?}", "append (end)", after.text);

    store.append_to_note(
        Some(&note.id),
        None,
        "Prepended line.\n\n",
        InsertPosition::Beginning,
        true,
        Default::default(),
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(
        after.text.contains("Prepended line."),
        "append Beginning failed"
    );
    println!("  {:<30} {:?}", "append (beginning)", after.text);

    // ── edit ──────────────────────────────────────────────────────────────────

    section("edit");
    store.edit_note(
        Some(&note.id),
        None,
        &[EditOp {
            at: "Appended at end.".into(),
            replace: Some("Replaced text.".into()),
            insert: None,
            all: false,
            ignore_case: false,
            word: false,
        }],
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(after.text.contains("Replaced text."), "edit replace failed");
    println!("  {:<30} {:?}", "edit (replace)", after.text);

    store.edit_note(
        Some(&note.id),
        None,
        &[EditOp {
            at: "Prepended".into(),
            replace: Some("PREPENDED".into()),
            insert: None,
            all: false,
            ignore_case: true,
            word: false,
        }],
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(after.text.contains("PREPENDED"), "edit ignore_case failed");
    println!("  {:<30} {:?}", "edit (ignore_case)", after.text);

    // edit insert (append to match)
    store.edit_note(
        Some(&note.id),
        None,
        &[EditOp {
            at: "Replaced text.".into(),
            replace: None,
            insert: Some(" (inserted)".into()),
            all: false,
            ignore_case: false,
            word: false,
        }],
    )?;
    let after = store.get_note(Some(&note.id), None, false, false)?;
    assert!(
        after.text.contains("Replaced text. (inserted)"),
        "edit insert failed"
    );
    println!("  {:<30} {:?}", "edit (insert)", after.text);

    // search_in_note
    let matches = store.search_in_note(Some(&note.id), None, "PREPENDED", false)?;
    assert!(!matches.is_empty(), "search_in_note returned no matches");
    println!(
        "  {:<30} {} line(s) matched",
        "search_in_note",
        matches.len()
    );

    // ── tags ──────────────────────────────────────────────────────────────────

    section("tags");
    store.add_tags(Some(&note.id), None, &["bear-rs-extra", "bear-rs-extra2"])?;
    let note_tags = store.list_tags(Some(&note.id), None)?;
    let tag_names: Vec<&str> = note_tags.iter().map(|t| t.name.as_str()).collect();
    assert!(tag_names.contains(&"bear-rs-extra"), "add_tags failed");
    println!("  {:<30} {:?}", "add_tags", tag_names);

    store.remove_tags(Some(&note.id), None, &["bear-rs-extra2"])?;
    let note_tags = store.list_tags(Some(&note.id), None)?;
    let tag_names: Vec<&str> = note_tags.iter().map(|t| t.name.as_str()).collect();
    assert!(!tag_names.contains(&"bear-rs-extra2"), "remove_tags failed");
    println!("  {:<30} {:?}", "remove_tags", tag_names);

    store.rename_tag("bear-rs-extra", "bear-rs-renamed", false)?;
    let note_tags = store.list_tags(Some(&note.id), None)?;
    let tag_names: Vec<&str> = note_tags.iter().map(|t| t.name.as_str()).collect();
    assert!(tag_names.contains(&"bear-rs-renamed"), "rename_tag failed");
    println!("  {:<30} {:?}", "rename_tag", tag_names);

    store.delete_tag("bear-rs-renamed")?;
    let note_tags = store.list_tags(Some(&note.id), None)?;
    let tag_names: Vec<&str> = note_tags.iter().map(|t| t.name.as_str()).collect();
    assert!(!tag_names.contains(&"bear-rs-renamed"), "delete_tag failed");
    println!("  {:<30} {:?}", "delete_tag", tag_names);

    // ── pins ──────────────────────────────────────────────────────────────────

    section("pins");
    store.add_pins(Some(&note.id), None, &["global"])?;
    let note_pins = store.list_pins(Some(&note.id), None)?;
    let pin_names: Vec<&str> = note_pins.iter().map(|p| p.pin.as_str()).collect();
    assert!(pin_names.contains(&"global"), "add_pins global failed");
    println!("  {:<30} {:?}", "add_pins (global)", pin_names);

    store.remove_pins(Some(&note.id), None, &["global"])?;
    let note_pins = store.list_pins(Some(&note.id), None)?;
    let pin_names: Vec<&str> = note_pins.iter().map(|p| p.pin.as_str()).collect();
    assert!(!pin_names.contains(&"global"), "remove_pins global failed");
    println!("  {:<30} {:?}", "remove_pins (global)", pin_names);

    // ── attachments ───────────────────────────────────────────────────────────

    section("attachments");
    let content = b"hello from bear-rs attachment test";
    store.add_attachment(Some(&note.id), None, "bear-rs-test.txt", content)?;
    let atts = store.list_attachments(Some(&note.id), None)?;
    let found = atts.iter().find(|a| a.filename == "bear-rs-test.txt");
    assert!(found.is_some(), "add_attachment: file not in list");
    println!(
        "  {:<30} {:?}",
        "add_attachment",
        atts.iter().map(|a| &a.filename).collect::<Vec<_>>()
    );

    let bytes = store.read_attachment(Some(&note.id), None, "bear-rs-test.txt")?;
    assert_eq!(bytes, content, "read_attachment returned wrong bytes");
    println!("  {:<30} {} bytes", "read_attachment", bytes.len());

    store.delete_attachment(Some(&note.id), None, "bear-rs-test.txt")?;
    let atts = store.list_attachments(Some(&note.id), None)?;
    assert!(
        atts.iter().all(|a| a.filename != "bear-rs-test.txt"),
        "delete_attachment failed"
    );
    println!("  {:<30} ok", "delete_attachment");

    // ── trash / archive / restore ─────────────────────────────────────────────

    section("trash / archive / restore");
    store.archive_note(Some(&note.id), None)?;
    println!("  {:<30} ok", "archive_note");

    store.restore_note(Some(&note.id), None)?;
    println!("  {:<30} ok", "restore_note (from archive)");

    store.trash_note(Some(&note.id), None)?;
    println!("  {:<30} ok", "trash_note");

    store.restore_note(Some(&note.id), None)?;
    println!("  {:<30} ok", "restore_note (from trash)");

    // ── cleanup ───────────────────────────────────────────────────────────────

    section("cleanup");
    store.trash_note(Some(&note.id), None)?;
    println!("  test note trashed (id={})", note.id);

    // clean up the remaining bear-rs-test tag
    let _ = store.delete_tag("bear-rs-test");

    println!("\n✓ all checks passed");
    Ok(())
}
