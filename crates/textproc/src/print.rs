//! Turning a document into a printable page (`NC-F007`, `docs/ui-reference.md` §8).
//!
//! The reference's Page setup dialog describes one thing: the sheet the printer
//! will mark. This module is that description made computable — the paper, its
//! orientation, the margins, and the header/footer line the capture lets the user
//! type — and the deterministic work that follows from it: where the text runs
//! out to, how many sheets it fills, and the PostScript that carries it to the
//! system print pipeline.
//!
//! Everything here is pure and byte-deterministic so the page can be proven to be
//! right without a printer attached. The one thing that cannot be checked in a
//! headless run is a real sheet coming out of a real printer; that hand-off lives
//! in the shell, which feeds these bytes to `lpr` and is as far as the tests reach.

/// One PostScript point is 1/72 of an inch, the unit the page geometry is drawn in.
const POINTS_PER_INCH: f32 = 72.0;

/// A page-setup paper size, named as `docs/ui-reference.md` §8 names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    /// US Letter, 8.5 × 11 in.
    Letter,
    /// ISO A4, 210 × 297 mm — the default, a Notepad-like editor's safe sheet.
    #[default]
    A4,
    /// US Legal, 8.5 × 14 in.
    Legal,
}

impl PaperSize {
    /// The three sizes §8 offers, in the order the dropdown shows them.
    pub const ALL: [PaperSize; 3] = [PaperSize::Letter, PaperSize::A4, PaperSize::Legal];

    /// The sheet as it is sold, in whole millimetres, as `(width, height)` upright.
    ///
    /// Letter and Legal are inch sizes; rounded to the millimetre they are named by,
    /// because the margins the user sets are in millimetres and one unit keeps the
    /// arithmetic honest.
    pub const fn mm(self) -> (u32, u32) {
        match self {
            PaperSize::Letter => (216, 279),
            PaperSize::A4 => (210, 297),
            PaperSize::Legal => (216, 356),
        }
    }

    /// The sheet in points for one orientation — the sides simply trade in landscape.
    pub fn points(self, orientation: Orientation) -> (f32, f32) {
        let (w, h) = self.mm();
        let (w, h) = (mm_to_pt(w), mm_to_pt(h));
        match orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        }
    }

    /// The word shown in the dialog and stored in settings.
    pub const fn label(self) -> &'static str {
        match self {
            PaperSize::Letter => "Letter",
            PaperSize::A4 => "A4",
            PaperSize::Legal => "Legal",
        }
    }

    /// The size a settings token names, if any.
    pub fn from_label(label: &str) -> Option<PaperSize> {
        Self::ALL.into_iter().find(|p| p.label() == label)
    }
}

/// Which way the sheet is fed, as §8 names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Tall edge up — the way a page is read.
    #[default]
    Portrait,
    /// Wide edge up.
    Landscape,
}

impl Orientation {
    /// The word shown in the dialog and stored in settings.
    pub const fn label(self) -> &'static str {
        match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
        }
    }

    /// The orientation a settings token names, if any.
    pub fn from_label(label: &str) -> Option<Orientation> {
        match label {
            "Portrait" => Some(Orientation::Portrait),
            "Landscape" => Some(Orientation::Landscape),
            _ => None,
        }
    }
}

/// The four page margins, in millimetres, as §8 collects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margins {
    /// Distance from the left sheet edge to the text.
    pub left: u32,
    /// Distance from the right sheet edge to the text.
    pub right: u32,
    /// Distance from the top sheet edge to the header's line.
    pub top: u32,
    /// Distance from the bottom sheet edge to the footer's line.
    pub bottom: u32,
}

impl Margins {
    /// Two centimetres on every side — the plain, printable-everywhere frame.
    pub const fn default_mm() -> Margins {
        Margins {
            left: 20,
            right: 20,
            top: 20,
            bottom: 20,
        }
    }
}

impl Default for Margins {
    fn default() -> Self {
        Margins::default_mm()
    }
}

/// What §8's Page setup dialog collects: the shape of the printed page and the
/// header/footer line drawn on it.
///
/// The header and footer hold `&p` (the page number) and `&d` (the date) where the
/// user wants them; [`expand_placeholders`] is what turns one typed line into the
/// text that lands on a given sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageSetup {
    /// The sheet.
    pub paper: PaperSize,
    /// How it is fed.
    pub orientation: Orientation,
    /// The frame the text lives inside.
    pub margins: Margins,
    /// The line drawn above the text; may be empty.
    pub header: String,
    /// The line drawn below the text; may be empty.
    pub footer: String,
}

/// Millimetres to PostScript points. Whole millimetres in, a point value out.
pub const fn mm_to_pt(mm: u32) -> f32 {
    (mm as f32) * POINTS_PER_INCH / 25.4
}

/// Expands the §8 header/footer placeholders against one sheet.
///
/// `&p` becomes `page_number` (counted from one) and `&d` becomes `date`, which the
/// caller passes in so the layout is a function of its inputs rather than of the
/// clock. A `&&` puts a literal ampersand-and pair; a lone `&` followed by anything
/// else is left exactly as typed, so text that happens to contain an ampersand is
/// never silently eaten.
pub fn expand_placeholders(line: &str, page_number: usize, date: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('p') => out.push_str(&page_number.to_string()),
            Some('d') => out.push_str(date),
            Some('&') => out.push_str("&&"),
            Some(other) => {
                out.push('&');
                out.push(other);
            }
            None => out.push('&'),
        }
    }
    out
}

/// The monospaced metrics the paginator lays text out with, in points.
///
/// The editor's faces are all monospaced, so a line's width is its scalar count
/// times one advance; one pair of numbers fully describes a page of text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// The width of one character.
    pub char_width_pt: f32,
    /// The baseline-to-baseline distance of one line.
    pub line_height_pt: f32,
}

impl Metrics {
    /// Courier-advance metrics at a point size: six-tenths of an em wide, fourteen-
    /// tenths leading — the proportions the classic fixed font, and thus a printer's
    /// built-in Courier, actually has.
    pub const fn monospace(point_size: f32) -> Metrics {
        Metrics {
            char_width_pt: point_size * 0.6,
            line_height_pt: point_size * 1.4,
        }
    }
}

/// One printed sheet: the resolved header, the body lines that fit, the resolved
/// footer. Held as text rather than drawing commands so the split across pages can
/// be asserted directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The header line with its placeholders resolved for this sheet.
    pub header: String,
    /// The body lines, each already short enough for one text row.
    pub lines: Vec<String>,
    /// The footer line with its placeholders resolved for this sheet.
    pub footer: String,
}

/// How much body text fits on one sheet, given the setup and the metrics.
///
/// The header and footer, when present, each hold a line of their own; the rest of
/// the frame is body. At least one column and one row are always granted, so a page
/// never becomes a zero-sized box that a document could never fit inside.
fn body_grid(setup: &PageSetup, metrics: Metrics) -> (usize, usize) {
    let (w_pt, h_pt) = setup.paper.points(setup.orientation);
    let left = mm_to_pt(setup.margins.left);
    let right = mm_to_pt(setup.margins.right);
    let top = mm_to_pt(setup.margins.top);
    let bottom = mm_to_pt(setup.margins.bottom);

    let text_width = (w_pt - left - right).max(0.0);
    let text_height = (h_pt - top - bottom).max(0.0);

    let header_lines = usize::from(!setup.header.is_empty());
    let footer_lines = usize::from(!setup.footer.is_empty());

    let body_height =
        (text_height - (header_lines + footer_lines) as f32 * metrics.line_height_pt).max(0.0);

    let columns = (text_width / metrics.char_width_pt).floor() as usize;
    let rows = (body_height / metrics.line_height_pt).floor() as usize;

    (columns.max(1), rows.max(1))
}

/// Breaks text into the physical rows a page can hold, and groups those rows into
/// sheets.
///
/// Line endings of any style start a new row; a logical line longer than the frame
/// wraps by character, the way the editor's unwrapped view would run off the edge
/// but a printer cannot. Empty input still fills one sheet, so the caller always has
/// something to print.
pub fn paginate(text: &str, setup: &PageSetup, metrics: Metrics, date: &str) -> Vec<Page> {
    let (columns, rows) = body_grid(setup, metrics);

    let mut body: Vec<String> = Vec::new();
    for logical in split_lines(text) {
        if logical.chars().count() <= columns {
            body.push(logical);
            continue;
        }
        let mut run = String::new();
        let mut len = 0;
        for ch in logical.chars() {
            if len == columns {
                body.push(std::mem::take(&mut run));
                len = 0;
            }
            run.push(ch);
            len += 1;
        }
        if !run.is_empty() {
            body.push(run);
        }
    }

    let page_count = body.len().div_ceil(rows).max(1);
    let mut pages = Vec::with_capacity(page_count);
    for number in 0..page_count {
        let start = number * rows;
        let slice = &body[start..(start + rows).min(body.len())];
        pages.push(Page {
            header: expand_placeholders(&setup.header, number + 1, date),
            lines: slice.to_vec(),
            footer: expand_placeholders(&setup.footer, number + 1, date),
        });
    }
    pages
}

/// Splits on `\n`, `\r\n`, or a lone `\r`, without leaving the terminator behind.
fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                lines.push(std::mem::take(&mut current));
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }
    lines.push(current);
    lines
}

/// Renders paginated sheets as a self-contained PostScript program.
///
/// PostScript is what the system print pipeline speaks, so the editor's job ends
/// with these bytes. The output is fixed for fixed input: no timestamps, a page size
/// chosen from the setup, and text drawn in the interpreter's built-in Courier.
pub fn to_postscript(pages: &[Page], setup: &PageSetup, metrics: Metrics) -> String {
    let (w_pt, h_pt) = setup.paper.points(setup.orientation);
    let left = mm_to_pt(setup.margins.left);
    let top = mm_to_pt(setup.margins.top);
    let bottom = mm_to_pt(setup.margins.bottom);
    let size = metrics.line_height_pt / 1.4;

    let mut out = String::new();
    out.push_str("%!PS-Adobe-3.0\n");
    out.push_str("%%Creator: NoteCraft\n");
    out.push_str("%%Title: (NoteCraft)\n");
    out.push_str("%%Pages: ");
    out.push_str(&pages.len().to_string());
    out.push('\n');
    out.push_str("%%EndComments\n");

    for (index, page) in pages.iter().enumerate() {
        out.push_str(&format!("%%Page: {} {}\n", index + 1, pages.len()));
        out.push_str(&format!(
            "<< /PageSize [{} {}] /ImagingBBox null >> setpagedevice\n",
            round_pt(w_pt),
            round_pt(h_pt)
        ));
        // The page geometry is re-set each sheet, which resets the graphics state,
        // so the font is re-selected with it.
        out.push_str(&format!("/Courier {} scalefont setfont\n", round_pt(size)));

        let mut y = h_pt - top;
        if !page.header.is_empty() {
            draw_text(&mut out, &page.header, left, y);
            y -= metrics.line_height_pt;
        }
        for line in &page.lines {
            draw_text(&mut out, line, left, y);
            y -= metrics.line_height_pt;
        }
        if !page.footer.is_empty() {
            draw_text(&mut out, &page.footer, left, bottom);
        }
        out.push_str("showpage\n");
    }

    out.push_str("%%Trailer\n");
    out.push_str("%%EOF\n");
    out
}

/// Draws one line at an absolute baseline origin, the font already selected.
fn draw_text(out: &mut String, text: &str, x: f32, y: f32) {
    out.push_str(&format!(
        "BT {} {} Td ({}) Tj ET\n",
        round_pt(x),
        round_pt(y),
        escape_postscript(text)
    ));
}

/// Escapes the bytes a PostScript string literal cannot carry directly.
///
/// A backslash and the two parentheses are the literal's own syntax and take a
/// backslash. Control characters become spaces. Any other character is written as a
/// PostScript octal byte escape when it sits in the Latin-1 range the built-in
/// Courier can actually show, and as `?` beyond it, so the program stays valid for
/// whatever the document contains.
fn escape_postscript(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            ' ' => out.push(' '),
            c if c.is_ascii_graphic() => out.push(c),
            c if c.is_ascii_control() => out.push(' '),
            c if (c as u32) <= 0xff => out.push_str(&format!("\\{:03o}", c as u32)),
            _ => out.push('?'),
        }
    }
    out
}

/// Rounds a point value for the PostScript program, so the emitted text is stable.
fn round_pt(value: f32) -> i32 {
    value.round() as i32
}

/// Builds the complete PostScript job for a document, from its text and setup.
///
/// This is the seam the shell feeds to the system print pipeline; everything above
/// it is pure.
pub fn print_document(text: &str, setup: &PageSetup, point_size: f32, date: &str) -> String {
    let metrics = Metrics::monospace(point_size);
    let pages = paginate(text, setup, metrics, date);
    to_postscript(&pages, setup, metrics)
}
