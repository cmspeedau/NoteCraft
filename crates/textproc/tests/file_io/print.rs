//! NC-F007 — page setup and the print pipeline.
//!
//! The Page setup dialog (§8, `docs/ui-reference.md`) collects the sheet the printer
//! will mark: a paper size, an orientation, four margins in millimetres, and a
//! header/footer line with `&p`/`&d` placeholders. What follows from those choices is
//! computed here — how the text is wrapped into rows, how many sheets it fills, and
//! the PostScript that carries it to the system print pipeline — and it is byte-for-
//! byte deterministic, so the page can be proven correct with no printer attached.
//! The one clause these cannot exercise is a real sheet leaving a real printer; that
//! hand-off is the shell feeding these bytes to `lpr`, and it is stated as a boundary
//! in `docs/feature-matrix.md` rather than asserted.

use nc_textproc::{
    Margins, Metrics, Orientation, PageSetup, PaperSize, expand_placeholders, mm_to_pt, paginate,
    print_document, to_postscript,
};

/// The date the placeholder tests expand `&d` against; any fixed string works because
/// the layout is a function of its inputs.
const TODAY: &str = "2026-10-17";

/// A4, portrait, two-centimetre margins — the default a fresh Page setup shows.
fn setup() -> PageSetup {
    PageSetup::default()
}

/// Ten-point monospace, the metrics the shell prints a document with.
fn metrics() -> Metrics {
    Metrics::monospace(10.0)
}

/// A document of `n` lines numbered from one.
fn lines(n: usize) -> String {
    (1..=n)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn each_paper_size_is_a_sheet_and_landscape_trades_the_sides() {
    // §8 offers exactly Letter, A4 and Legal, and orientation turns the sheet.
    assert_eq!(
        PaperSize::ALL,
        [PaperSize::Letter, PaperSize::A4, PaperSize::Legal]
    );

    let a4 = PaperSize::A4.points(Orientation::Portrait);
    assert!(a4.0 < a4.1, "A4 portrait stands taller than wide: {a4:?}");

    let turned = PaperSize::A4.points(Orientation::Landscape);
    assert_eq!(
        (turned.0, turned.1),
        (a4.1, a4.0),
        "landscape is the same sheet turned through a quarter turn"
    );
}

#[test]
fn millimetres_become_points_on_the_printer_unit() {
    // A printed inch is 72 points; the margins the user sets are in millimetres, so
    // one inch (25.4 mm) has to land on 72.
    assert!(
        (mm_to_pt(254) - 720.0).abs() < 0.001,
        "10 inches = 720 points"
    );
    assert!((mm_to_pt(0) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn a_short_document_prints_on_one_sheet() {
    let pages = paginate("one\ntwo\nthree", &setup(), metrics(), TODAY);
    assert_eq!(pages.len(), 1, "three lines cannot overflow a page");
    assert_eq!(pages[0].lines, vec!["one", "two", "three"]);
    assert!(pages[0].header.is_empty(), "no header typed, none drawn");
}

#[test]
fn an_empty_document_still_fills_one_sheet() {
    // Printing an empty document has to send one blank page, not zero pages and a
    // confused print queue.
    let pages = paginate("", &setup(), metrics(), TODAY);
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].lines, vec![String::new()]);
}

#[test]
fn a_document_taller_than_the_sheet_spills_onto_the_next() {
    // Fifty-two rows of ten-point text fit between two-centimetre margins on A4; the
    // line past them begins a new sheet, and the sheets reassemble the document.
    let text = lines(53);
    let pages = paginate(&text, &setup(), metrics(), TODAY);

    assert_eq!(
        pages.len(),
        2,
        "fifty-three rows do not fit on one A4 sheet"
    );
    assert_eq!(pages[0].lines.len(), 52, "the first sheet is full");
    assert_eq!(
        pages[1].lines,
        vec!["line 53"],
        "the spill is the one line over"
    );

    let reassembled: Vec<&str> = pages
        .iter()
        .flat_map(|p| p.lines.iter().map(String::as_str))
        .collect();
    assert_eq!(
        reassembled,
        (1..=53).map(|i| format!("line {i}")).collect::<Vec<_>>()[..]
    );
}

#[test]
fn the_header_and_footer_each_occupy_a_row() {
    // A typed header steals a body row: the text that filled one plain sheet now
    // needs a second, because the header has to sit above it.
    let mut setup = setup();
    setup.header = "Draft".to_string();

    let plain = paginate(&lines(52), &PageSetup::default(), metrics(), TODAY);
    let headed = paginate(&lines(52), &setup, metrics(), TODAY);

    assert_eq!(plain.len(), 1);
    assert_eq!(headed.len(), 2, "the header pushed the last line over");
    assert_eq!(headed[0].header, "Draft");
    assert_eq!(headed[0].lines.len(), 51);
}

#[test]
fn a_line_wider_than_the_frame_wraps_by_character() {
    // Eighty monospace characters span the text width at ten points on A4; the
    // eighty-first starts a second printed row under the first.
    let long = "x".repeat(81);
    let pages = paginate(&long, &setup(), metrics(), TODAY);
    assert_eq!(
        pages[0].lines.len(),
        2,
        "one long row became two short ones"
    );
    assert_eq!(pages[0].lines[0], "x".repeat(80));
    assert_eq!(pages[0].lines[1], "x");
}

#[test]
fn placeholders_become_the_page_number_and_date() {
    // §8 names only `&p` and `&d`; a `&&` is how the user types a literal ampersand,
    // and an `&` that leads nowhere is left alone rather than swallowed.
    assert_eq!(
        expand_placeholders("Page &p of the run", 3, TODAY),
        "Page 3 of the run"
    );
    assert_eq!(
        expand_placeholders("Printed &d", 1, TODAY),
        "Printed 2026-10-17"
    );
    assert_eq!(expand_placeholders("A && B", 1, TODAY), "A && B");
    assert_eq!(
        expand_placeholders("100% & ready", 1, TODAY),
        "100% & ready"
    );

    // The number advances with the sheet, so a page-numbered footer reads in order.
    let mut setup = setup();
    setup.footer = "&p".to_string();
    let pages = paginate(&lines(53), &setup, metrics(), TODAY);
    assert_eq!(pages[0].footer, "1");
    assert_eq!(pages[1].footer, "2");
}

#[test]
fn every_line_ending_style_starts_a_printed_row() {
    // A document may carry CRLF or CR endings; both are a new printed row, neither
    // leaves its terminator inside the text.
    let pages = paginate("a\r\nb\rc\n d", &setup(), metrics(), TODAY);
    assert_eq!(pages[0].lines, vec!["a", "b", "c", " d"]);
}

#[test]
fn the_program_begins_and_ends_like_postscript_and_pages_itself() {
    let text = lines(53);
    let ps = print_document(&text, &setup(), 10.0, TODAY);

    assert!(
        ps.starts_with("%!PS-Adobe-3.0\n"),
        "a printer needs the magic first line"
    );
    assert!(ps.trim_end().ends_with("%%EOF"), "and a clean last line");

    let declared: usize = ps
        .lines()
        .find_map(|l| l.strip_prefix("%%Pages: "))
        .and_then(|n| n.trim().parse().ok())
        .expect("a page count");
    let sheets = ps.matches("showpage").count();
    assert_eq!(
        declared, sheets,
        "the declared page count is the sheets actually emitted"
    );
    assert_eq!(
        sheets, 2,
        "the same document that paginated to two sheets prints two"
    );
}

#[test]
fn the_sheet_size_carried_to_the_printer_follows_the_setup() {
    // The /PageSize directive is what makes A4 print on A4; a mismatch is the printer
    // marking the wrong sheet. It is points, and it trades sides in landscape.
    let a4 = print_document("hi", &setup(), 10.0, TODAY);
    assert!(
        a4.contains("/PageSize [595 842]"),
        "A4 portrait is 595 by 842 points:\n{a4}"
    );

    let mut landscape = setup();
    landscape.orientation = Orientation::Landscape;
    let wide = print_document("hi", &landscape, 10.0, TODAY);
    assert!(
        wide.contains("/PageSize [842 595]"),
        "the same sheet turned is 842 by 595"
    );

    let letter = print_document("hi", &letter_setup(), 10.0, TODAY);
    assert!(
        letter.contains("/PageSize [612 791]"),
        "Letter is its own box, not A4's"
    );
}

fn letter_setup() -> PageSetup {
    PageSetup {
        paper: PaperSize::Letter,
        ..PageSetup::default()
    }
}

#[test]
fn characters_a_literal_cannot_hold_are_escaped_or_replaced() {
    // Parentheses and the backslash are the PostScript string's own syntax; they have
    // to be escaped or the document breaks the program. A Latin-1 letter becomes its
    // octal byte; anything past what Courier can show becomes a question mark rather
    // than corrupting the run.
    let ps = print_document("a (b) c\\d", &setup(), 10.0, TODAY);
    assert!(
        ps.contains("a \\(b\\) c\\\\d"),
        "the literal stays intact:\n{ps}"
    );

    let accented = print_document("café", &setup(), 10.0, TODAY);
    assert!(accented.contains("\\351"), "é is octal 351");

    let beyond = print_document("a\u{2019}b", &setup(), 10.0, TODAY);
    assert!(
        beyond.contains("a?b"),
        "a curly quote past Latin-1 is not drawn"
    );
}

#[test]
fn the_same_document_prints_the_same_bytes_twice() {
    // No clock, no randomness: the same document and setup produce an identical job,
    // which is what lets all of the above be asserted rather than eyeballed.
    let setup = PageSetup {
        header: "&d, page &p".to_string(),
        ..setup()
    };
    let first = print_document("a\nb\nc", &setup, 10.0, TODAY);
    let again = print_document("a\nb\nc", &setup, 10.0, TODAY);
    assert_eq!(first, again);

    // And the header lands resolved, on the sheet it belongs to.
    assert!(
        first.contains("(2026-10-17, page 1) Tj"),
        "the header carries its page"
    );
}

#[test]
fn a_zero_margin_frame_still_prints_something() {
    // A pathological setup must not divide to a zero-sized box; the page always
    // keeps at least one row and one column.
    let setup = PageSetup {
        margins: Margins {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
        ..PageSetup::default()
    };
    let pages = paginate("hello", &setup, metrics(), TODAY);
    assert_eq!(pages[0].lines, vec!["hello"]);
    // Guard the PostScript against an empty page list from a nonsense box.
    let ps = to_postscript(&pages, &setup, metrics());
    assert_eq!(ps.matches("showpage").count(), 1);
}
