//! Calendar dates as days since 1970-01-01, through venue-specs' own date parser.

use crate::error::{Error, Result};

/// Days since the Unix epoch for a `YYYY-MM-DD` date.
pub fn days(date: &str) -> Result<i64> {
    let ok = date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-';
    let secs = venue_specs::time::parse(date).ok().filter(|_| ok);
    secs.map(|s| s.div_euclid(86_400))
        .ok_or_else(|| Error::input(format!("{date:?} is not a date: use YYYY-MM-DD")))
}

/// The `YYYY-MM-DD` date `n` days after the epoch (Howard Hinnant's civil_from_days).
pub fn date(n: i64) -> String {
    let z = n + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for d in [
            "1970-01-01",
            "2026-11-24",
            "2027-01-08",
            "2028-02-29",
            "2000-03-01",
        ] {
            assert_eq!(date(days(d).unwrap()), d);
        }
        assert_eq!(
            days("2027-01-08").unwrap() - days("2026-11-24").unwrap(),
            45
        );
    }

    #[test]
    fn rejects_bad_dates() {
        for d in ["2026-02-30", "2026-11-24T00:00:00Z", "24/11/2026", ""] {
            assert!(days(d).is_err(), "{d}");
        }
    }
}
