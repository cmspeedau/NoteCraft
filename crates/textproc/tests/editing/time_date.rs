//! NC-S007 — the time and date F5 writes into the document.
//!
//! The engine owns the *text*; the shell owns *when*. These tests fill in a
//! `TimeDate` by hand rather than reading a clock, so they pin the format F5
//! writes without waiting for a particular minute to come round.
//!
//! What is settled (`docs/ui-reference.md` S-rows for F5): the inserted
//! text is time, then date, separated by a space — `3:47 PM 10/12/2026`; the
//! date runs month, then day, then year, and month and day carry no leading
//! zeros; the clock is 12-hour, so midnight and noon are both `12`. Seconds are
//! on the type because a clock reports them, but they are not written, so
//! `time_and_date` does not either.

use nc_textproc::{Meridiem, TimeDate};

#[test]
fn the_inserted_text_is_the_time_then_the_date() {
    let now = TimeDate::new(2026, 10, 12, 15, 47, 9);
    assert_eq!(now.time_and_date(), "3:47 PM 10/12/2026");
}

#[test]
fn the_date_is_month_first_without_leading_zeros() {
    let now = TimeDate::new(2026, 1, 5, 9, 3, 0);
    assert_eq!(now.date(), "1/5/2026");
}

#[test]
fn the_minute_is_always_two_digits() {
    let now = TimeDate::new(2026, 10, 12, 15, 7, 0);
    assert_eq!(now.time(), "3:07 PM");
}

#[test]
fn midnight_is_twelve_am() {
    let midnight = TimeDate::new(2026, 10, 12, 0, 0, 0);
    assert_eq!(midnight.hour12(), 12);
    assert_eq!(midnight.meridiem(), Meridiem::Am);
    assert_eq!(midnight.time(), "12:00 AM");
}

#[test]
fn noon_is_twelve_pm() {
    let noon = TimeDate::new(2026, 10, 12, 12, 0, 0);
    assert_eq!(noon.hour12(), 12);
    assert_eq!(noon.meridiem(), Meridiem::Pm);
    assert_eq!(noon.time(), "12:00 PM");
}

#[test]
fn the_morning_runs_to_eleven_fifty_nine() {
    let now = TimeDate::new(2026, 10, 12, 11, 59, 59);
    assert_eq!(now.time(), "11:59 AM");
}

#[test]
fn the_afternoon_starts_at_twelve_and_runs_to_eleven_fifty_nine() {
    let now = TimeDate::new(2026, 8, 1, 13, 0, 0);
    assert_eq!(now.time(), "1:00 PM");
    let now = TimeDate::new(2026, 8, 1, 23, 59, 59);
    assert_eq!(now.time(), "11:59 PM");
}

#[test]
fn the_hour_never_reads_as_zero() {
    for hour in 0..24 {
        let now = TimeDate::new(2026, 10, 12, hour, 0, 0);
        // 0 and 12 both read as 12; every other hour reads as itself.
        let expected = match hour % 12 {
            0 => 12,
            other => other,
        };
        assert_eq!(now.hour12(), expected, "12-hour clock: {hour} o'clock");
    }
}

#[test]
fn meridiem_names_are_spelled_the_way_the_reference_writes_them() {
    assert_eq!(TimeDate::new(2026, 1, 1, 0, 0, 0).meridiem_name(), "AM");
    assert_eq!(TimeDate::new(2026, 1, 1, 12, 0, 0).meridiem_name(), "PM");
}

#[test]
fn seconds_are_kept_but_not_written() {
    let now = TimeDate::new(2026, 10, 12, 15, 47, 33);
    assert_eq!(now.second, 33, "a clock reports them");
    assert!(
        !now.time_and_date().contains("33"),
        "the reference omits them"
    );
}

#[test]
fn the_whole_text_is_one_space_apart() {
    let now = TimeDate::new(2026, 10, 12, 15, 47, 9);
    let whole = now.time_and_date();
    let parts: Vec<&str> = whole.split(' ').collect();
    assert_eq!(parts.len(), 3, "hour, meridiem, date");
    assert_eq!(parts, ["3:47", "PM", "10/12/2026"]);
}

#[test]
fn a_winter_date_at_the_end_of_the_year_is_written_whole() {
    let now = TimeDate::new(2026, 12, 31, 23, 58, 0);
    assert_eq!(now.time_and_date(), "11:58 PM 12/31/2026");
}

#[test]
fn a_clock_reading_can_be_filled_in_and_read_back() {
    // The shell reads these six fields off the platform clock; nothing here
    // assumes a timezone, a locale or a calendar.
    let now = TimeDate::new(2026, 6, 30, 6, 5, 1);
    assert_eq!(now.year, 2026);
    assert_eq!(now.month, 6);
    assert_eq!(now.day, 30);
    assert_eq!(now.hour, 6);
    assert_eq!(now.minute, 5);
    assert_eq!(now.second, 1);
    assert_eq!(now.time_and_date(), "6:05 AM 6/30/2026");
}
