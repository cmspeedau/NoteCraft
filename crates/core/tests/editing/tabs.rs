//! NC-E007 — Insert and delete tabs.
//!
//! `docs/feature-matrix.md` NC-E007. The tab-stop width and Tab-key behaviour
//! are design decisions (`docs/ui-reference.md`), so the engine keeps the width
//! as a parameter and these tests pin the arithmetic and the storage rather than
//! a guess at the default.

use nc_core::{TabStop, display_width};

#[test]
fn the_default_tab_stop_is_eight_columns() {
    assert_eq!(TabStop::DEFAULT.width(), 8);
    assert_eq!(TabStop::default().width(), 8);
}

#[test]
fn a_zero_width_is_treated_as_one_column() {
    assert_eq!(TabStop::new(0).width(), 1);
}

#[test]
fn advance_moves_to_the_next_stop() {
    let tab = TabStop::new(8);
    assert_eq!(tab.advance(0), 8);
    assert_eq!(tab.advance(1), 8);
    assert_eq!(tab.advance(7), 8);
    assert_eq!(tab.advance(8), 16);
    assert_eq!(tab.advance(9), 16);
}

#[test]
fn a_tab_advances_the_display_column() {
    let tab = TabStop::new(8);
    assert_eq!(display_width("\t", tab, 0), 8);
    assert_eq!(display_width("\tabc", tab, 0), 11);
    // A tab at column 3 reaches column 8, so it occupies 5 columns.
    assert_eq!(display_width("\t", tab, 3), 5);
}

#[test]
fn a_tab_is_stored_as_a_single_character() {
    let mut document = nc_core::Document::new();
    assert!(document.insert_str("\t"));
    assert_eq!(document.text(), "\t");
    assert_eq!(document.line_count(), 1);
    assert_eq!(document.caret(), 1);
}

#[test]
fn backspace_removes_a_tab() {
    let mut document = nc_core::Document::new();
    document.insert_str("\t");
    assert!(document.backspace());
    assert_eq!(document.text(), "");
}

#[test]
fn a_tab_survives_an_undo_and_redo_round_trip() {
    let mut document = nc_core::Document::new();
    document.insert_str("\t");
    document.undo();
    assert_eq!(document.text(), "");
    document.redo();
    assert_eq!(document.text(), "\t");
}

#[test]
fn the_tab_stop_width_is_settable_per_document() {
    let mut document = nc_core::Document::new();
    assert_eq!(document.tab_stop().width(), 8);
    document.set_tab_stop(TabStop::new(4));
    assert_eq!(document.tab_stop().width(), 4);
    assert_eq!(document.tab_stop().advance(4), 8);
}
