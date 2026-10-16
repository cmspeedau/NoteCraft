//! Reading the wall clock for the F5 time/date command.
//!
//! `nc-textproc::TimeDate` decides the *text* of the F5 insertion; this module
//! is the small piece of platform work that fills its fields from the system
//! clock. The clock is read and expressed as **UTC**, because the Rust standard
//! library exposes no local time zone and NoteCraft carries no time-zone
//! database. A machine outside UTC therefore sees UTC written into the document
//! — a recorded limitation (`docs/feature-matrix.md`, NC-S007) rather than a
//! silent approximation, and one to revisit alongside the localisation work in
//! M8.

use std::time::{SystemTime, UNIX_EPOCH};

use nc_textproc::TimeDate;

/// Seconds in a day.
const SECONDS_PER_DAY: u64 = 86_400;

/// The current instant, read from the system clock as UTC civil time.
pub(crate) fn now_utc() -> TimeDate {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since_epoch| since_epoch.as_secs())
        .unwrap_or(0);
    from_unix_seconds(seconds)
}

/// The UTC date and time `seconds` past the Unix epoch.
fn from_unix_seconds(seconds: u64) -> TimeDate {
    let days = (seconds / SECONDS_PER_DAY) as i64;
    let time = seconds % SECONDS_PER_DAY;
    let (year, month, day) = civil_from_days(days);
    TimeDate::new(
        year,
        month,
        day,
        (time / 3600) as u32,
        ((time % 3600) / 60) as u32,
        (time % 60) as u32,
    )
}

/// The civil (year, month, day) for a count of days since the Unix epoch.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole `i64`
/// range and needs no table or loop.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    // Shift the epoch to 0000-03-01, which starts the 400-year leap cycle.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let month_position = (5 * day_of_year + 2) / 153; // [0, 11]
    let day = (day_of_year - (153 * month_position + 2) / 5 + 1) as u32; // [1, 31]
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    } as u32; // [1, 12]
    let year = year + i64::from(month <= 2);
    (year as i32, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_the_first_of_january_1970() {
        let moment = from_unix_seconds(0);
        assert_eq!(moment, TimeDate::new(1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn a_known_instant_reads_correctly() {
        // 2001-09-09 01:46:40 UTC.
        let moment = from_unix_seconds(1_000_000_000);
        assert_eq!(moment, TimeDate::new(2001, 9, 9, 1, 46, 40));

        // 2023-11-14 22:13:20 UTC.
        let moment = from_unix_seconds(1_700_000_000);
        assert_eq!(moment, TimeDate::new(2023, 11, 14, 22, 13, 20));
    }

    #[test]
    fn a_leap_day_is_not_missed() {
        // 2024-02-29 is 19_782 days after 1970-01-01.
        let days = 19_782;
        let (year, month, day) = civil_from_days(days);
        assert_eq!((year, month, day), (2024, 2, 29));
    }

    #[test]
    fn the_clock_reads_a_plausible_date() {
        let moment = now_utc();
        assert!(
            moment.year >= 2024,
            "the system clock reads an implausible year: {}",
            moment.year
        );
        assert!((1..=12).contains(&moment.month));
        assert!((1..=31).contains(&moment.day));
    }
}
