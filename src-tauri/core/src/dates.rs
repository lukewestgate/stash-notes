//! Turning switcher queries like `today`, `oct 22` or `2026-10-22` into dates.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

const MONTHS: [&str; 12] = [
    "january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november",
    "december",
];

fn month_from(word: &str) -> Option<u32> {
    if word.len() < 3 {
        return None;
    }
    MONTHS.iter().position(|m| m.starts_with(word)).map(|i| i as u32 + 1)
}

fn weekday_from(word: &str) -> Option<Weekday> {
    if word.len() < 3 {
        return None;
    }
    const DAYS: [(&str, Weekday); 7] = [
        ("monday", Weekday::Mon),
        ("tuesday", Weekday::Tue),
        ("wednesday", Weekday::Wed),
        ("thursday", Weekday::Thu),
        ("friday", Weekday::Fri),
        ("saturday", Weekday::Sat),
        ("sunday", Weekday::Sun),
    ];
    DAYS.iter().find(|(name, _)| name.starts_with(word)).map(|(_, d)| *d)
}

/// Parse an ISO date `YYYY-MM-DD`.
pub fn parse_iso(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Interpret a query as a date relative to `today`. Month-day queries without
/// a year pick the most recent such date that is not in the future.
pub fn parse_query(query: &str, today: NaiveDate) -> Option<NaiveDate> {
    let q = query.trim().to_lowercase();
    match q.as_str() {
        "today" => return Some(today),
        "yesterday" => return Some(today - Duration::days(1)),
        "tomorrow" => return Some(today + Duration::days(1)),
        _ => {}
    }
    if let Some(d) = parse_iso(&q) {
        return Some(d);
    }
    let words: Vec<&str> = q.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).collect();
    if words.len() == 1 {
        if let Some(wd) = weekday_from(words[0]) {
            let back = (today.weekday().num_days_from_monday() + 7 - wd.num_days_from_monday()) % 7;
            return Some(today - Duration::days(back as i64));
        }
        return None;
    }
    if words.len() < 2 || words.len() > 3 {
        return None;
    }
    let strip = |w: &str| w.trim_end_matches(|c: char| c.is_ascii_alphabetic()).to_string();
    let (month, day) = if let Some(m) = month_from(words[0]) {
        (m, strip(words[1]).parse::<u32>().ok()?)
    } else if let Some(m) = month_from(words[1]) {
        (m, strip(words[0]).parse::<u32>().ok()?)
    } else {
        return None;
    };
    if words.len() == 3 {
        let year: i32 = words[2].parse().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this_year > today {
        NaiveDate::from_ymd_opt(today.year() - 1, month, day)
    } else {
        Some(this_year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        parse_iso(s).unwrap()
    }

    #[test]
    fn relative_words() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("today", t), Some(t));
        assert_eq!(parse_query("Yesterday", t), Some(d("2026-10-21")));
        assert_eq!(parse_query("mon", t), Some(d("2026-10-19")));
        assert_eq!(parse_query("thursday", t), Some(t));
    }

    #[test]
    fn month_day() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("oct 22", t), Some(t));
        assert_eq!(parse_query("22 October", t), Some(t));
        assert_eq!(parse_query("1st oct", t), Some(d("2026-10-01")));
        assert_eq!(parse_query("dec 25", t), Some(d("2025-12-25")));
        assert_eq!(parse_query("dec 25 2027", t), Some(d("2027-12-25")));
        assert_eq!(parse_query("2026-10-22", t), Some(t));
    }

    #[test]
    fn non_dates() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("rollout plan", t), None);
        assert_eq!(parse_query("ma", t), None);
        assert_eq!(parse_query("123", t), None);
        assert_eq!(parse_query("2026-13-01", t), None);
    }
}
