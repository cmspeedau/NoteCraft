//! NC-E009 — drag and drop text.
//!
//! `dnd.rs` decides *where* a drop lands; the document performs the move. The
//! reference's modifier rule (drag-copy) is a design decision, so the engine only
//! guarantees the plain internal drag: a move, selected afterwards, undone as a
//! single step.

use nc_core::{Document, DragSource, drop_target};

#[test]
fn a_drop_before_the_span_keeps_the_target_index() {
    assert_eq!(drop_target(4..8, 0), Some(0));
}

#[test]
fn a_drop_between_the_source_and_the_target_shifts_left() {
    assert_eq!(drop_target(2..5, 9), Some(6));
}

#[test]
fn a_drop_at_or_inside_the_span_is_not_a_target() {
    assert_eq!(drop_target(2..5, 2), None);
    assert_eq!(drop_target(2..5, 3), None);
    assert_eq!(drop_target(2..5, 5), None);
}

#[test]
fn an_empty_source_has_no_target() {
    assert_eq!(drop_target(4..4, 0), None);
}

#[test]
fn a_drag_forwards_moves_the_span_to_the_target() {
    // Three identical-width words; "one " travels past "two ".
    let mut document = Document::from_text("one two three");
    document.click(0, nc_core::ClickKind::Single);
    document.extend_to(4, nc_core::ClickKind::Single);
    assert!(document.move_selection_to(8));
    assert_eq!(document.text(), "two one three");
    assert_eq!(document.selected_text(), Some("one "));
    assert_eq!(document.selection().anchor(), 4);
}

#[test]
fn a_drag_backwards_moves_the_span_to_the_target() {
    let mut document = Document::from_text("one two three");
    document.click(8, nc_core::ClickKind::Single);
    document.extend_to(13, nc_core::ClickKind::Single);
    assert!(document.move_selection_to(0));
    assert_eq!(document.text(), "threeone two ");
    assert_eq!(document.selected_text(), Some("three"));
}

#[test]
fn a_drag_to_the_end_appends_the_span() {
    let mut document = Document::from_text("one two three");
    document.click(0, nc_core::ClickKind::Single);
    document.extend_to(4, nc_core::ClickKind::Single);
    assert!(document.move_selection_to(99));
    assert_eq!(document.text(), "two threeone ");
    assert_eq!(document.selected_text(), Some("one "));
}

#[test]
fn a_drag_onto_itself_changes_nothing() {
    let mut document = Document::from_text("one two three");
    document.click(4, nc_core::ClickKind::Single);
    document.extend_to(8, nc_core::ClickKind::Single);
    let before = document.text().to_string();
    assert!(!document.move_selection_to(6));
    assert_eq!(document.text(), before);
    assert!(!document.can_undo());
}

#[test]
fn a_drag_with_no_selection_changes_nothing() {
    let mut document = Document::from_text("one two three");
    document.click(4, nc_core::ClickKind::Single);
    assert!(!document.move_selection_to(0));
    assert_eq!(document.text(), "one two three");
}

#[test]
fn a_drag_moves_whole_lines() {
    let mut document = Document::from_text("one\ntwo\nthree\nfour");
    document.click(0, nc_core::ClickKind::Triple);
    document.extend_to(5, nc_core::ClickKind::Triple);
    assert!(document.move_selection_to(99));
    assert_eq!(document.text(), "three\nfourone\ntwo\n");
    assert_eq!(document.selected_text(), Some("one\ntwo\n"));
}

#[test]
fn a_drag_is_one_undo_step() {
    let mut document = Document::from_text("one two three");
    document.click(0, nc_core::ClickKind::Single);
    document.extend_to(4, nc_core::ClickKind::Single);
    let depth = document.undo_depth();
    assert!(document.move_selection_to(8));
    assert_eq!(document.undo_depth(), depth + 1);

    assert!(document.undo());
    assert_eq!(document.text(), "one two three");
    assert_eq!(document.selected_text(), Some("one "));

    assert!(document.redo());
    assert_eq!(document.text(), "two one three");
}

#[test]
fn a_drop_marks_the_document_dirty() {
    // A move is an edit: a window shell that saves on a timer must see it.
    let mut document = Document::from_text("one two three");
    document.mark_saved();
    assert!(!document.is_dirty());
    document.click(0, nc_core::ClickKind::Single);
    document.extend_to(4, nc_core::ClickKind::Single);
    assert!(document.move_selection_to(8));
    assert!(document.is_dirty());
}

#[test]
fn a_drop_from_outside_the_document_inserts_and_selects_it() {
    let mut document = Document::from_text("one two");
    document.click(4, nc_core::ClickKind::Single);
    assert!(document.drop_text("three ", 4));
    assert_eq!(document.text(), "one three two");
    assert_eq!(document.selected_text(), Some("three "));
}

#[test]
fn a_drop_from_outside_normalizes_the_line_ending() {
    let mut document = Document::from_text("one");
    assert!(document.drop_text("a\r\nb", 3));
    assert_eq!(document.text(), "onea\nb");
    assert_eq!(document.selected_text(), Some("a\nb"));
}

#[test]
fn a_drop_from_outside_that_is_empty_changes_nothing() {
    let mut document = Document::from_text("one");
    assert!(!document.drop_text("", 1));
    assert!(!document.can_undo());
}

#[test]
fn an_empty_internal_drop_does_not_duplicate_the_document() {
    // Dragging an empty selection into the document is the no-op; the guard in
    // `Document::move_selection_to` is what keeps the text intact.
    let mut document = Document::from_text("one");
    document.click(1, nc_core::ClickKind::Single);
    assert!(!document.move_selection_to(1));
    assert_eq!(document.text(), "one");
    assert_eq!(DragSource::Internal, DragSource::Internal);
}
