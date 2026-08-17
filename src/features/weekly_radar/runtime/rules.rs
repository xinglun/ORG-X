//! Deterministic rule extraction for provider-supplied workforce passages.

use chrono::NaiveDate;
use regex::Regex;

use super::model::FactStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EmployeeCandidate {
    pub(crate) value: String,
    pub(crate) approximate: bool,
    pub(crate) passage: String,
    pub(crate) effective_date: Option<NaiveDate>,
}

/// Extracts an employee count only when one workforce candidate has usable
/// date context. Ambiguous, unrelated, or insufficient passages remain
/// unknown.
pub fn extract_employee_count(
    text: &str,
    effective_date: Option<NaiveDate>,
    source: &str,
) -> FactStatus {
    extract_employee_candidate(text, effective_date, source)
        .map(|_| FactStatus::Known)
        .unwrap_or(FactStatus::Unknown)
}

/// Returns the one candidate that can be retained with provenance.
pub(crate) fn extract_employee_candidate(
    text: &str,
    effective_date: Option<NaiveDate>,
    source: &str,
) -> Option<EmployeeCandidate> {
    let _source = source;
    let text = strip_markup(text);
    let candidates = employee_candidates(&text);
    if candidates.len() != 1 {
        return None;
    }

    let mut candidate = candidates.into_iter().next()?;
    let candidate_date = candidate.effective_date.or(effective_date);
    if candidate_date.is_none() {
        return None;
    }
    if candidate.effective_date.is_some()
        && effective_date.is_some()
        && candidate.effective_date != effective_date
    {
        return None;
    }
    candidate.effective_date = candidate_date;
    Some(candidate)
}

fn employee_candidates(text: &str) -> Vec<EmployeeCandidate> {
    let pattern = Regex::new(
        r"(?ix)
        (?P<approx>\b(?:approximately|approx\.?|about|nearly|roughly|more\s+than|over)\b\s+)?
        (?P<value>\d[\d,]*)\s+
        (?P<phrase>employees?|people|workforce\s+members?)
        ",
    )
    .expect("employee extraction regex must be valid");
    let mut candidates = Vec::new();

    for capture in pattern.captures_iter(text) {
        let phrase = capture
            .name("phrase")
            .map(|value| value.as_str())
            .unwrap_or("");
        if phrase.eq_ignore_ascii_case("people")
            && !passage_for_match(text, capture.get(0).expect("full match"))
                .to_lowercase()
                .contains("workforce")
        {
            continue;
        }
        let value = capture
            .name("value")
            .map(|value| value.as_str().replace(',', ""))
            .filter(|value| !value.is_empty())
            .unwrap_or_default();
        if value.is_empty() {
            continue;
        }
        let full_match = capture.get(0).expect("full match");
        let passage = passage_for_match(text, full_match);
        let effective_date = parse_date(&passage);
        let approximate = capture.name("approx").is_some();
        candidates.push(EmployeeCandidate {
            value,
            approximate,
            passage,
            effective_date,
        });
    }

    candidates
}

fn passage_for_match(text: &str, matched: regex::Match<'_>) -> String {
    let start = text[..matched.start()]
        .rfind(|character: char| matches!(character, '.' | '!' | '?' | '\n'))
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = text[matched.end()..]
        .find(|character: char| matches!(character, '.' | '!' | '?' | '\n'))
        .map(|index| matched.end() + index + 1)
        .unwrap_or(text.len());
    text[start..end].trim().to_owned()
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    let iso = Regex::new(r"\b(?P<date>\d{4}-\d{2}-\d{2})\b").expect("ISO date regex must be valid");
    if let Some(date) = iso
        .captures(text)
        .and_then(|capture| capture.name("date"))
        .and_then(|date| NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d").ok())
    {
        return Some(date);
    }

    let long = Regex::new(
        r"(?ix)\b(?P<month>january|february|march|april|may|june|july|august|september|october|november|december)\s+(?P<day>\d{1,2}),\s+(?P<year>\d{4})\b",
    )
    .expect("long date regex must be valid");
    let capture = long.captures(text)?;
    let month = match capture
        .name("month")?
        .as_str()
        .to_ascii_lowercase()
        .as_str()
    {
        "january" => 1,
        "february" => 2,
        "march" => 3,
        "april" => 4,
        "may" => 5,
        "june" => 6,
        "july" => 7,
        "august" => 8,
        "september" => 9,
        "october" => 10,
        "november" => 11,
        "december" => 12,
        _ => return None,
    };
    let day = capture.name("day")?.as_str().parse().ok()?;
    let year = capture.name("year")?.as_str().parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

fn strip_markup(text: &str) -> String {
    Regex::new(r"(?is)<[^>]+>")
        .expect("markup regex must be valid")
        .replace_all(text, " ")
        .into_owned()
}
