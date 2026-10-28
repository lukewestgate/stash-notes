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

/// How daily notes are named by default: `2026-10-22`.
pub const ISO_FORMAT: &str = "yyyy-MM-dd";

/// The tokens of a date format, longest first so `MMMM` isn't read as `MM` twice.
const TOKENS: [(&str, &str); 10] = [
    ("yyyy", "%Y"),
    ("yy", "%y"),
    ("MMMM", "%B"),
    ("MMM", "%b"),
    ("MM", "%m"),
    ("M", "%-m"),
    ("EEEE", "%A"),
    ("EEE", "%a"),
    ("dd", "%d"),
    ("d", "%-d"),
];

/// Format `date` with a pattern like `dd/MM/yyyy` or `EEE d MMM yyyy`:
/// `yyyy` `yy` year, `MMMM` `MMM` `MM` `M` month, `EEEE` `EEE` weekday,
/// `dd` `d` day. Anything else is kept as written.
pub fn format(date: NaiveDate, pattern: &str) -> String {
    let mut spec = String::new();
    let mut rest = pattern;
    'outer: while let Some(c) = rest.chars().next() {
        for (token, s) in TOKENS {
            if let Some(r) = rest.strip_prefix(token) {
                spec.push_str(s);
                rest = r;
                continue 'outer;
            }
        }
        if c == '%' {
            spec.push('%');
        }
        spec.push(c);
        rest = &rest[c.len_utf8()..];
    }
    date.format(&spec).to_string()
}

/// Whether a format puts the day before the month (`dd/MM/yyyy`), so a
/// typed `3/10` means 3 October.
pub fn day_first(pattern: &str) -> bool {
    match (pattern.find('d'), pattern.find('M')) {
        (Some(d), Some(m)) => d < m,
        _ => false,
    }
}

/// `28/10`, `28/10/2026`, `28.10.2026` (or month first): numbers with `/`
/// between them, or `.` with a year (so `1.5` stays a version number).
fn parse_numeric(q: &str, day_first: bool, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = q.split(['/', '.']).collect();
    if !(2..=3).contains(&parts.len()) || (q.contains('.') && parts.len() < 3) || parts.iter().any(|p| p.is_empty() || p.len() > 4 || !p.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    let a: u32 = parts[0].parse().ok()?;
    let b: u32 = parts[1].parse().ok()?;
    let (day, month) = if day_first { (a, b) } else { (b, a) };
    match parts.get(2) {
        Some(y) => {
            let year: i32 = y.parse().ok()?;
            let year = if y.len() <= 2 { 2000 + year } else { year };
            NaiveDate::from_ymd_opt(year, month, day)
        }
        None => latest(today, month, day),
    }
}

/// The most recent `month`/`day` that isn't in the future.
fn latest(today: NaiveDate, month: u32, day: u32) -> Option<NaiveDate> {
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day);
    match this_year {
        Some(d) if d <= today => Some(d),
        _ => NaiveDate::from_ymd_opt(today.year() - 1, month, day),
    }
}

/// `3`, `three`, `a`: a count of days or weeks.
fn count_from(word: &str) -> Option<i64> {
    const WORDS: [&str; 11] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    match word {
        "a" | "an" => Some(1),
        _ => word.parse().ok().filter(|n| *n <= 3660).or_else(|| WORDS.iter().position(|w| *w == word).map(|n| n as i64)),
    }
}

/// Phrases relative to today: `day before yesterday`, `3 days ago`,
/// `a week ago`, and a weekday with a week: `last tuesday` (the latest one
/// before today), `next friday`, `tuesday last week`, `last week tue`,
/// `this week fri`, `2 weeks ago monday`. Weeks start on Monday.
fn parse_relative(words: &[&str], today: NaiveDate) -> Option<NaiveDate> {
    let days = |n: i64| today.checked_sub_signed(Duration::days(n));
    match words {
        ["day", "before", "yesterday"] => return days(2),
        ["day", "after", "tomorrow"] => return days(-2),
        [n, unit, "ago"] if unit.starts_with("day") => return days(count_from(n)?),
        [n, unit, "ago"] if unit.starts_with("week") => return days(7 * count_from(n)?),
        _ => {}
    }
    // One weekday; the other words say which week it's in.
    let (at, wd) = words.iter().enumerate().find_map(|(i, w)| weekday_from(w).map(|d| (i, d)))?;
    let rest: Vec<&str> = words.iter().enumerate().filter(|(i, _)| *i != at).map(|(_, w)| *w).collect();
    let (now, want) = (today.weekday().num_days_from_monday() as i64, wd.num_days_from_monday() as i64);
    let in_week = |weeks: i64| days(now - want + 7 * weeks);
    match rest.as_slice() {
        ["last"] | ["previous"] => days((now - want + 6).rem_euclid(7) + 1),
        ["next"] => days(-((want - now + 6).rem_euclid(7) + 1)),
        ["this"] | ["this", "week"] => in_week(0),
        ["last", "week"] | ["previous", "week"] => in_week(1),
        ["next", "week"] => in_week(-1),
        [n, unit, "ago"] if unit.starts_with("week") => in_week(count_from(n)?),
        _ => None,
    }
}

/// Interpret a query as a date relative to `today`. Month-day queries without
/// a year pick the most recent such date that is not in the future.
/// `day_first` reads `3/10` as 3 October rather than March 10.
pub fn parse_query(query: &str, today: NaiveDate, day_first: bool) -> Option<NaiveDate> {
    let q = query.trim().to_lowercase();
    match q.as_str() {
        "today" => return Some(today),
        "yesterday" => return Some(today - Duration::days(1)),
        "tomorrow" => return Some(today + Duration::days(1)),
        _ => {}
    }
    if let Some(d) = parse_iso(&q).or_else(|| parse_numeric(&q, day_first, today)) {
        return Some(d);
    }
    let words: Vec<&str> = q.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).collect();
    if words.len() > 1 {
        if let Some(d) = parse_relative(&words, today) {
            return Some(d);
        }
    }
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
    NaiveDate::from_ymd_opt(today.year(), month, day)?;
    latest(today, month, day)
}

/// The words of date phrases, in the order a half-typed one is finished
/// with: `to` is today before tomorrow, `th` this before thursday.
const VOCAB: [&str; 21] = [
    "today", "yesterday", "tomorrow", "last", "this", "next", "previous", "week", "weeks", "days", "day", "ago", "before",
    "after", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday",
];

/// A date phrase still being typed, so the day shows up before the last word
/// is finished: `yest`, `last tu`, `tuesday last we`, `3 days`. The last word
/// is finished from the words date phrases use (a lone word needs 3 letters,
/// one after others any), or `ago`, `yesterday` or `tomorrow` is added after a
/// whole one. None for a query that's already a date, or isn't on its way to one.
pub fn parse_partial(query: &str, today: NaiveDate, day_first: bool) -> Option<NaiveDate> {
    let q = query.trim_start().to_lowercase();
    if q.trim().is_empty() || parse_query(&q, today, day_first).is_some() {
        return None;
    }
    let parse = |candidate: &str| parse_query(candidate, today, day_first);
    if q.ends_with(char::is_whitespace) || q.ends_with(',') {
        return None;
    }
    let (head, last) = match q.rsplit_once(|c: char| c.is_whitespace() || c == ',') {
        Some((h, l)) => (format!("{h} "), l),
        None => (String::new(), q.as_str()),
    };
    let min = if head.is_empty() { 3 } else { 1 };
    let mut candidates: Vec<String> = Vec::new();
    if last.len() >= min && last.chars().all(|c| c.is_ascii_alphabetic()) {
        candidates.extend(VOCAB.iter().filter(|w| w.starts_with(last) && **w != last).map(|w| format!("{head}{w}")));
    }
    // Or the last word is whole: `3 days`, `day before`.
    candidates.push(q.clone());
    // Each as it is, else with the word that would end it: `2 weeks` + `ago`.
    let ended = |c: &String| {
        let more = (!head.is_empty()).then(|| ["ago", "yesterday", "tomorrow"].iter().find_map(|w| parse(&format!("{c} {w}"))));
        parse(c).or(more.flatten())
    };
    candidates.iter().filter(|c| **c != q || !head.is_empty()).find_map(ended)
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
        assert_eq!(parse_query("today", t, false), Some(t));
        assert_eq!(parse_query("Yesterday", t, false), Some(d("2026-10-21")));
        assert_eq!(parse_query("mon", t, false), Some(d("2026-10-19")));
        assert_eq!(parse_query("thursday", t, false), Some(t));
    }

    #[test]
    fn month_day() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("oct 22", t, false), Some(t));
        assert_eq!(parse_query("22 October", t, false), Some(t));
        assert_eq!(parse_query("1st oct", t, false), Some(d("2026-10-01")));
        assert_eq!(parse_query("dec 25", t, false), Some(d("2025-12-25")));
        assert_eq!(parse_query("dec 25 2027", t, false), Some(d("2027-12-25")));
        assert_eq!(parse_query("2026-10-22", t, false), Some(t));
    }

    #[test]
    fn numeric() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("3/10", t, true), Some(d("2026-10-03")));
        assert_eq!(parse_query("3/10", t, false), Some(d("2026-03-10")));
        assert_eq!(parse_query("22/10/2026", t, true), Some(t));
        assert_eq!(parse_query("22.10.26", t, true), Some(t));
        assert_eq!(parse_query("25/12", t, true), Some(d("2025-12-25")));
        assert_eq!(parse_query("10/22", t, true), None);
        assert_eq!(parse_query("1/2/3/4", t, true), None);
        assert_eq!(parse_query("1.5", t, true), None);
    }

    #[test]
    fn formats() {
        let t = d("2026-10-02");
        assert_eq!(format(t, ISO_FORMAT), "2026-10-02");
        assert_eq!(format(t, "dd/MM/yyyy"), "02/10/2026");
        assert_eq!(format(t, "EEE d MMM yyyy"), "Fri 2 Oct 2026");
        assert_eq!(format(t, "EEEE d MMMM yy, 100%"), "Friday 2 October 26, 100%");
        assert!(day_first("dd/MM/yyyy") && day_first("EEE d MMM") && !day_first(ISO_FORMAT) && !day_first("MM/dd/yyyy"));
    }

    #[test]
    fn relative_phrases() {
        // A Thursday.
        let t = d("2026-10-22");
        assert_eq!(parse_query("day before yesterday", t, false), Some(d("2026-10-20")));
        assert_eq!(parse_query("3 days ago", t, false), Some(d("2026-10-19")));
        assert_eq!(parse_query("one day ago", t, false), Some(d("2026-10-21")));
        assert_eq!(parse_query("a week ago", t, false), Some(d("2026-10-15")));
        assert_eq!(parse_query("2 weeks ago", t, false), Some(d("2026-10-08")));
        // Latest before today, so `last thursday` on a Thursday is a week ago.
        assert_eq!(parse_query("last tuesday", t, false), Some(d("2026-10-20")));
        assert_eq!(parse_query("last thu", t, false), Some(d("2026-10-15")));
        assert_eq!(parse_query("next mon", t, false), Some(d("2026-10-26")));
        assert_eq!(parse_query("next thursday", t, false), Some(d("2026-10-29")));
        // The week it's in, whichever order the words come in.
        assert_eq!(parse_query("last week tuesday", t, false), Some(d("2026-10-13")));
        assert_eq!(parse_query("Tuesday, last week", t, false), Some(d("2026-10-13")));
        assert_eq!(parse_query("last week sun", t, false), Some(d("2026-10-18")));
        assert_eq!(parse_query("this week friday", t, false), Some(d("2026-10-23")));
        assert_eq!(parse_query("this mon", t, false), Some(d("2026-10-19")));
        assert_eq!(parse_query("monday next week", t, false), Some(d("2026-10-26")));
        assert_eq!(parse_query("2 weeks ago monday", t, false), Some(d("2026-10-05")));
        // On a Monday, last week runs from the Monday before.
        let mon = d("2026-10-26");
        assert_eq!(parse_query("last week monday", mon, false), Some(d("2026-10-19")));
        assert_eq!(parse_query("last monday", mon, false), Some(d("2026-10-19")));
        assert_eq!(parse_query("monday", mon, false), Some(mon));
        // Other words still aren't dates.
        assert_eq!(parse_query("last week", t, false), None);
        assert_eq!(parse_query("monday standup notes", t, false), None);
        assert_eq!(parse_query("ten days ago", t, false), Some(d("2026-10-12")));
        assert_eq!(parse_query("many days ago", t, false), None);
    }

    #[test]
    fn half_typed_phrases() {
        // A Thursday.
        let t = d("2026-10-22");
        let p = |q: &str| parse_partial(q, t, false);
        assert_eq!(p("yest"), Some(d("2026-10-21")));
        assert_eq!(p("Yesterda"), Some(d("2026-10-21")));
        assert_eq!(p("tod"), Some(t));
        assert_eq!(p("tom"), Some(d("2026-10-23")));
        assert_eq!(p("last tu"), Some(d("2026-10-20")));
        assert_eq!(p("last th"), Some(d("2026-10-15")));
        assert_eq!(p("last week tu"), Some(d("2026-10-13")));
        assert_eq!(p("tuesday last we"), Some(d("2026-10-13")));
        assert_eq!(p("this fr"), Some(d("2026-10-23")));
        assert_eq!(p("3 days"), Some(d("2026-10-19")));
        assert_eq!(p("3 days a"), Some(d("2026-10-19")));
        assert_eq!(p("2 we"), Some(d("2026-10-08")));
        assert_eq!(p("day before"), Some(d("2026-10-20")));
        assert_eq!(p("day before yes"), Some(d("2026-10-20")));
        // Whole dates aren't partial, and nothing is guessed from too little.
        assert_eq!(p("yesterday"), None);
        assert_eq!(p("monday"), None);
        assert_eq!(p("ye"), None);
        assert_eq!(p("to"), None);
        assert_eq!(p("last "), None);
        assert_eq!(p("last"), None);
        assert_eq!(p("last week"), None);
        assert_eq!(p("rollout pl"), None);
        assert_eq!(p("todo"), None);
        assert_eq!(p(""), None);
    }

    #[test]
    fn non_dates() {
        let t = d("2026-10-22");
        assert_eq!(parse_query("rollout plan", t, false), None);
        assert_eq!(parse_query("ma", t, false), None);
        assert_eq!(parse_query("123", t, false), None);
        assert_eq!(parse_query("2026-13-01", t, false), None);
    }
}
