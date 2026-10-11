//! The date and time the editor inserts at the caret
//! (`docs/feature-matrix.md` NC-S007).
//!
//! F5 in the reference writes the current wall-clock time and date into the
//! document. This module is the engine half of that: it turns the fields a wall
//! clock shows into the two strings the reference writes, so the whole feature
//! can be tested without waiting for a particular minute to come around.
//!
//! Reading the clock is the shell's job — [`TimeDate`] is filled in from
//! whatever the platform layer can read — which is the same split the rest of
//! the engine uses: engine decides the text, shell decides when.
//!
//! # Evidence discipline
//!
//! The reference's `TO_CAPTURE` note covers the exact punctuation here. What is
//! settled and encoded below (`docs/reference-matrix.md` S-rows for F5):
//!
//! * the inserted text is *time, then date*, separated by a space —
//!   `3:47 PM 12/10/2026`;
//! * the time is a 12-hour clock with `AM`/`PM`, so midnight and noon are both
//!   `12`, never `0`;
//! * month and day are written without leading zeros.
//!
//! Seconds are kept on the type because a clock reports them; the reference
//! does not write them, so neither does [`TimeDate::time_and_date`].

/// Whether a time is before or after noon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meridiem {
    /// Before noon, written `AM`.
    Am,
    /// Noon or after, written `PM`.
    Pm,
}

/// A wall-clock date and time, as the reference's F5 inserts it.
///
/// The fields are civil, not a timestamp: `hour` is 0–23 on a 24-hour clock,
/// and the 12-hour reading is derived by [`TimeDate::hour12`]. Nothing here
/// reads a clock or consults a time zone, so a value can be built and checked
/// in a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeDate {
    /// The calendar year.
    pub year: i32,
    /// The month, 1–12.
    pub month: u32,
    /// The day of the month, 1–31.
    pub day: u32,
    /// The hour on a 24-hour clock, 0–23.
    pub hour: u32,
    /// The minute, 0–59.
    pub minute: u32,
    /// The second, 0–59. Reported by the clock, not written by the reference.
    pub second: u32,
}

impl TimeDate {
    /// A date and time, given as a wall clock shows them.
    pub const fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Whether this time is in the morning or the afternoon.
    pub const fn meridiem(&self) -> Meridiem {
        if self.hour < 12 {
            Meridiem::Am
        } else {
            Meridiem::Pm
        }
    }

    /// The hour on a 12-hour clock: midnight and noon are both 12, the rest
    /// count 1–11.
    pub const fn hour12(&self) -> u32 {
        match self.hour % 12 {
            0 => 12,
            hour => hour,
        }
    }

    /// `AM` or `PM`, as the reference writes it.
    pub const fn meridiem_name(&self) -> &'static str {
        match self.meridiem() {
            Meridiem::Am => "AM",
            Meridiem::Pm => "PM",
        }
    }

    /// `3:47 PM 12/10/2026` — the whole thing, as F5 inserts it.
    pub fn time_and_date(&self) -> String {
        format!("{} {}", self.time(), self.date())
    }

    /// `3:47 PM` — the time alone.
    pub fn time(&self) -> String {
        format!(
            "{}:{:02} {}",
            self.hour12(),
            self.minute,
            self.meridiem_name()
        )
    }

    /// `12/10/2026` — the date alone, month first, without leading zeros.
    pub fn date(&self) -> String {
        format!("{}/{}/{}", self.month, self.day, self.year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inserted_text_is_the_time_and_then_the_date() {
        let moment = TimeDate::new(2026, 12, 10, 15, 47, 6);
        assert_eq!(moment.time_and_date(), "3:47 PM 12/10/2026");
    }

    #[test]
    fn midnight_and_noon_are_twelve_on_a_twelve_hour_clock() {
        assert_eq!(TimeDate::new(2026, 1, 1, 0, 5, 0).time(), "12:05 AM");
        assert_eq!(TimeDate::new(2026, 1, 1, 12, 5, 0).time(), "12:05 PM");
    }
}
