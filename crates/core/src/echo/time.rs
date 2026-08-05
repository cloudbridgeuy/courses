//! Timestamp formatting for the echo answer.

/// Formats a Unix timestamp as RFC 3339 in UTC, with milliseconds.
///
/// Hand-written rather than pulled from a date crate: the calendar arithmetic
/// is a pure function of two integers, and it is the only date handling the
/// crate needs.
pub fn format_rfc3339_utc(unix_secs: i64, nanos: u32) -> String {
    let days = unix_secs.div_euclid(86_400);
    let seconds_of_day = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let millis = nanos / 1_000_000;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60,
        seconds_of_day % 60,
    )
}

/// Converts days since the Unix epoch into a proleptic Gregorian date, by
/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01, so leap days land at the end of the cycle.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097; // [0, 146_096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let month_prime = (5 * day_of_year + 2) / 153; // [0, 11], March is 0
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1; // [1, 31]
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = year + i64::from(month <= 2);
    // Both fit their ranges by construction, so the casts cannot truncate.
    (year, month as u32, day as u32)
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::format_rfc3339_utc;

    #[test]
    fn formats_the_epoch() {
        assert_eq!(format_rfc3339_utc(0, 0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn formats_a_known_instant_with_millis() {
        // 2025-01-01T00:00:00Z
        assert_eq!(
            format_rfc3339_utc(1_735_689_600, 123_456_789),
            "2025-01-01T00:00:00.123Z"
        );
    }

    #[test]
    fn formats_a_leap_day() {
        // 2000-02-29T12:34:56Z — 2000 is a leap year despite ending a century.
        assert_eq!(
            format_rfc3339_utc(951_827_696, 0),
            "2000-02-29T12:34:56.000Z"
        );
    }

    #[test]
    fn formats_the_end_of_a_non_leap_february() {
        // 2100 is not a leap year: 2100-03-01 follows 2100-02-28.
        assert_eq!(
            format_rfc3339_utc(4_107_542_400, 0),
            "2100-03-01T00:00:00.000Z"
        );
    }

    #[test]
    fn formats_a_timestamp_before_the_epoch() {
        assert_eq!(format_rfc3339_utc(-1, 0), "1969-12-31T23:59:59.000Z");
    }
}
