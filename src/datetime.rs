//! # Date and Time Values
//!
//! TOML-style date and time literals, as they appear in NOML and TOML files:
//!
//! ```noml
//! created  = 1979-05-27T07:32:00Z         # offset date-time
//! local    = 1979-05-27T07:32:00.999      # local date-time
//! birthday = 1979-05-27                   # local date
//! alarm    = 07:32:00                     # local time
//! ```
//!
//! A [`Datetime`] keeps exactly the parts that were written: a date, a time,
//! an offset, or a combination. It does not depend on any date library; with
//! the `chrono` feature it converts to and from `chrono` types.
//!
//! ```rust
//! use noml::Datetime;
//!
//! let dt: Datetime = "1979-05-27T07:32:00-08:00".parse()?;
//! assert_eq!(dt.date.unwrap().year, 1979);
//! assert_eq!(dt.to_string(), "1979-05-27T07:32:00-08:00");
//! # Ok::<(), noml::NomlError>(())
//! ```

use crate::error::NomlError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A date, a time, or both, with an optional UTC offset.
///
/// The four TOML forms map to these field combinations:
///
/// | Form             | `date` | `time` | `offset` |
/// |------------------|--------|--------|----------|
/// | Offset date-time | yes    | yes    | yes      |
/// | Local date-time  | yes    | yes    | no       |
/// | Local date       | yes    | no     | no       |
/// | Local time       | no     | yes    | no       |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Datetime {
    /// Calendar date
    pub date: Option<Date>,
    /// Time of day
    pub time: Option<Time>,
    /// Offset from UTC (only together with a date and a time)
    pub offset: Option<Offset>,
}

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year, 0000 to 9999
    pub year: u16,
    /// Month, 1 to 12
    pub month: u8,
    /// Day of the month, 1 to 31
    pub day: u8,
}

/// A time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    /// Hour, 0 to 23
    pub hour: u8,
    /// Minute, 0 to 59
    pub minute: u8,
    /// Second, 0 to 60 (60 allows a leap second)
    pub second: u8,
    /// Fraction of a second in nanoseconds, 0 to 999,999,999
    pub nanosecond: u32,
}

/// Offset from UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Offset {
    /// UTC, written `Z`
    Z,
    /// A fixed offset, written `+HH:MM` or `-HH:MM`
    Custom {
        /// Offset in minutes, -1439 to 1439
        minutes: i16,
    },
}

impl Datetime {
    /// True for a date and time with an offset (an absolute instant)
    pub fn is_offset_datetime(&self) -> bool {
        self.date.is_some() && self.time.is_some() && self.offset.is_some()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        if self.nanosecond != 0 {
            let digits = format!("{:09}", self.nanosecond);
            write!(f, ".{}", digits.trim_end_matches('0'))?;
        }
        Ok(())
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Offset::Z => f.write_str("Z"),
            Offset::Custom { minutes } => {
                let sign = if minutes < 0 { '-' } else { '+' };
                let minutes = minutes.unsigned_abs();
                write!(f, "{sign}{:02}:{:02}", minutes / 60, minutes % 60)
            }
        }
    }
}

impl fmt::Display for Datetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(date) = &self.date {
            write!(f, "{date}")?;
            if self.time.is_some() {
                f.write_str("T")?;
            }
        }
        if let Some(time) = &self.time {
            write!(f, "{time}")?;
        }
        if let Some(offset) = &self.offset {
            write!(f, "{offset}")?;
        }
        Ok(())
    }
}

impl FromStr for Datetime {
    type Err = NomlError;

    /// Parse a TOML date/time such as `1979-05-27T07:32:00Z`,
    /// `1979-05-27 07:32`, `1979-05-27` or `07:32:00.5`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match lex(s) {
            Ok(Some((dt, len))) if len == s.len() => Ok(dt),
            Ok(_) => Err(NomlError::validation(format!("Invalid date/time: '{s}'"))),
            Err(message) => Err(NomlError::validation(format!("{message}: '{s}'"))),
        }
    }
}

impl Serialize for Datetime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Datetime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Read `n` ASCII digits at `pos` as a number
fn digits(bytes: &[u8], pos: usize, n: usize) -> Option<u32> {
    let slice = bytes.get(pos..pos + n)?;
    if !slice.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        slice
            .iter()
            .fold(0, |acc, b| acc * 10 + u32::from(b - b'0')),
    )
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// True if `bytes[pos..]` starts like `HH:MM`
fn time_shaped(bytes: &[u8], pos: usize) -> bool {
    digits(bytes, pos, 2).is_some() && bytes.get(pos + 2) == Some(&b':')
}

/// Parse `HH:MM[:SS[.fraction]]` at `pos`; returns the time and end position
fn lex_time(bytes: &[u8], pos: usize) -> Result<(Time, usize), String> {
    let hour = digits(bytes, pos, 2).ok_or("Invalid hour")?;
    let minute = digits(bytes, pos + 3, 2).ok_or("Invalid minute")?;
    let mut end = pos + 5;
    let mut second = 0;
    let mut nanosecond = 0;
    if bytes.get(end) == Some(&b':') {
        second = digits(bytes, end + 1, 2).ok_or("Invalid second")?;
        end += 3;
        if bytes.get(end) == Some(&b'.') {
            let start = end + 1;
            let mut i = start;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == start {
                return Err("Missing digits after '.' in time".into());
            }
            // Keep nanosecond precision; extra digits are truncated
            for (k, b) in bytes[start..i].iter().take(9).enumerate() {
                nanosecond += u32::from(b - b'0') * 10u32.pow(8 - k as u32);
            }
            end = i;
        }
    }
    if hour > 23 || minute > 59 || second > 60 {
        return Err("Time out of range".into());
    }
    Ok((
        Time {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            nanosecond,
        },
        end,
    ))
}

/// Recognise a date/time at the start of `input`.
///
/// Returns `Ok(None)` if the input does not start like one (so it can be
/// read as a number instead), `Ok(Some((datetime, bytes_used)))` on success,
/// and `Err` for something shaped like a date/time but invalid.
pub(crate) fn lex(input: &str) -> Result<Option<(Datetime, usize)>, String> {
    let bytes = input.as_bytes();
    let date_shaped = digits(bytes, 0, 4).is_some()
        && bytes.get(4) == Some(&b'-')
        && digits(bytes, 5, 2).is_some()
        && bytes.get(7) == Some(&b'-')
        && digits(bytes, 8, 2).is_some();

    let mut result = Datetime {
        date: None,
        time: None,
        offset: None,
    };
    let mut end;

    if date_shaped {
        let year = digits(bytes, 0, 4).unwrap_or_default();
        let month = digits(bytes, 5, 2).unwrap_or_default();
        let day = digits(bytes, 8, 2).unwrap_or_default();
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err("Invalid date".into());
        }
        result.date = Some(Date {
            year: year as u16,
            month: month as u8,
            day: day as u8,
        });
        end = 10;

        // Time after `T`, `t`, or a single space
        let separator = bytes.get(end).copied();
        let has_time = matches!(separator, Some(b'T' | b't'))
            || (separator == Some(b' ') && time_shaped(bytes, end + 1));
        if has_time {
            let (time, after) = lex_time(bytes, end + 1)?;
            result.time = Some(time);
            end = after;

            match bytes.get(end) {
                Some(b'Z' | b'z') => {
                    result.offset = Some(Offset::Z);
                    end += 1;
                }
                Some(&sign @ (b'+' | b'-')) => {
                    let hours = digits(bytes, end + 1, 2).ok_or("Invalid offset")?;
                    if bytes.get(end + 3) != Some(&b':') {
                        return Err("Invalid offset".into());
                    }
                    let minutes = digits(bytes, end + 4, 2).ok_or("Invalid offset")?;
                    if hours > 23 || minutes > 59 {
                        return Err("Offset out of range".into());
                    }
                    let total = (hours * 60 + minutes) as i16;
                    result.offset = Some(Offset::Custom {
                        minutes: if sign == b'-' { -total } else { total },
                    });
                    end += 6;
                }
                _ => {}
            }
        }
    } else if time_shaped(bytes, 0) && digits(bytes, 3, 2).is_some() {
        let (time, after) = lex_time(bytes, 0)?;
        result.time = Some(time);
        end = after;
    } else {
        return Ok(None);
    }

    // Must not run straight into more of a word or number
    if bytes
        .get(end)
        .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-' | b'+'))
    {
        return Err("Invalid date/time".into());
    }
    Ok(Some((result, end)))
}

#[cfg(feature = "chrono")]
mod chrono_support {
    use super::{Date, Datetime, Offset, Time};
    use chrono::{Datelike, FixedOffset, NaiveDate, NaiveTime, TimeZone, Timelike};

    impl Datetime {
        /// Convert an offset date-time to a `chrono` date-time.
        ///
        /// Returns `None` for local dates, times and date-times, which do not
        /// name a single instant.
        pub fn to_chrono(&self) -> Option<chrono::DateTime<FixedOffset>> {
            let (date, time, offset) = (self.date?, self.time?, self.offset?);
            let seconds = match offset {
                Offset::Z => 0,
                Offset::Custom { minutes } => i32::from(minutes) * 60,
            };
            let naive = NaiveDate::from_ymd_opt(
                i32::from(date.year),
                u32::from(date.month),
                u32::from(date.day),
            )?
            .and_time(NaiveTime::from_hms_nano_opt(
                u32::from(time.hour),
                u32::from(time.minute),
                u32::from(time.second.min(59)),
                time.nanosecond,
            )?);
            FixedOffset::east_opt(seconds)?
                .from_local_datetime(&naive)
                .single()
        }
    }

    impl<Tz: TimeZone> From<chrono::DateTime<Tz>> for Datetime {
        fn from(dt: chrono::DateTime<Tz>) -> Self {
            let fixed = dt.fixed_offset();
            let minutes = (fixed.offset().local_minus_utc() / 60) as i16;
            Datetime {
                date: Some(Date {
                    year: fixed.year().clamp(0, 9999) as u16,
                    month: fixed.month() as u8,
                    day: fixed.day() as u8,
                }),
                time: Some(Time {
                    hour: fixed.hour() as u8,
                    minute: fixed.minute() as u8,
                    second: fixed.second() as u8,
                    nanosecond: fixed.nanosecond() % 1_000_000_000,
                }),
                offset: Some(if minutes == 0 {
                    Offset::Z
                } else {
                    Offset::Custom { minutes }
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> Datetime {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn all_toml_forms_round_trip() {
        for s in [
            "1979-05-27T07:32:00Z",
            "1979-05-27T00:32:00-07:00",
            "1979-05-27T00:32:00.999999+05:30",
            "1979-05-27T07:32:00",
            "1979-05-27",
            "07:32:00",
            "00:32:00.5",
        ] {
            assert_eq!(dt(s).to_string(), s);
        }
        // Accepted spellings that print in canonical form
        assert_eq!(
            dt("1979-05-27 07:32:00z").to_string(),
            "1979-05-27T07:32:00Z"
        );
        assert_eq!(dt("1979-05-27t07:32").to_string(), "1979-05-27T07:32:00");
        assert_eq!(dt("07:32").to_string(), "07:32:00");
    }

    #[test]
    fn invalid_values_are_rejected() {
        for s in [
            "1979-13-01",
            "1979-02-30",
            "1900-02-29",
            "1979-05-27T24:00:00",
            "1979-05-27T07:60:00",
            "1979-05-27T07:32:00+24:00",
            "1979-05-27T07:32:00.",
            "1979-05-27x",
            "07:32:00Z",
        ] {
            assert!(s.parse::<Datetime>().is_err(), "{s} should be rejected");
        }
        assert!("2000-02-29".parse::<Datetime>().is_ok(), "leap year");
    }

    #[test]
    fn non_dates_are_not_claimed() {
        assert_eq!(lex("1979"), Ok(None));
        assert_eq!(lex("12.5"), Ok(None));
        assert_eq!(lex("0x1F"), Ok(None));
    }

    #[test]
    fn parts_are_exposed() {
        let value = dt("1979-05-27T07:32:00.25-08:30");
        assert!(value.is_offset_datetime());
        assert_eq!(value.time.unwrap().nanosecond, 250_000_000);
        assert_eq!(value.offset, Some(Offset::Custom { minutes: -510 }));
        assert!(!dt("1979-05-27").is_offset_datetime());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_round_trip() {
        let value = dt("1979-05-27T07:32:00-08:00");
        let chrono_value = value.to_chrono().unwrap();
        assert_eq!(chrono_value.to_rfc3339(), "1979-05-27T07:32:00-08:00");
        assert_eq!(Datetime::from(chrono_value), value);
        assert!(dt("1979-05-27").to_chrono().is_none());
    }
}
