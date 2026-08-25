//! Bounded same-origin document discovery for configured official entry points.

use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use url::Url;

use super::sources::SourceKind;

/// Maximum number of document links followed from one official entry point.
pub const MAX_DOCUMENT_CANDIDATES_PER_ENTRY: usize = 8;

/// Coarse document class used to route deterministic claim extraction rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentKind {
    Filing,
    Earnings,
    InvestorDay,
    Engineering,
    AiAutomation,
    Organization,
    ProductPlatform,
    Careers,
}

impl DocumentKind {
    /// Returns the stable document-kind label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Filing => "filing",
            Self::Earnings => "earnings",
            Self::InvestorDay => "investor_day",
            Self::Engineering => "engineering",
            Self::AiAutomation => "ai_automation",
            Self::Organization => "organization",
            Self::ProductPlatform => "product_platform",
            Self::Careers => "careers",
        }
    }
}

/// A bounded, same-origin document link discovered from an entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentCandidate {
    company_id: String,
    source_kind: SourceKind,
    url: String,
    title: String,
    document_kind: DocumentKind,
    published_or_effective_date: Option<NaiveDate>,
    provenance: String,
}

impl DocumentCandidate {
    /// Returns the company identity.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the source family that exposed the document.
    pub const fn source_kind(&self) -> SourceKind {
        self.source_kind
    }

    /// Returns the canonical document URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the link title or fallback URL label.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the deterministic document classification.
    pub const fn document_kind(&self) -> DocumentKind {
        self.document_kind
    }

    /// Returns the optional explicit publication/effective date.
    pub const fn published_or_effective_date(&self) -> Option<NaiveDate> {
        self.published_or_effective_date
    }

    /// Returns bounded discovery provenance.
    pub fn provenance(&self) -> &str {
        &self.provenance
    }
}

/// Discovers relevant same-origin links without guessing paths or following an
/// unbounded crawl.
pub fn discover_documents(
    company_id: &str,
    source_kind: SourceKind,
    entry_url: &str,
    html: &str,
    observed_at: DateTime<Utc>,
) -> Vec<DocumentCandidate> {
    let Ok(base) = Url::parse(entry_url) else {
        return Vec::new();
    };
    let Ok(anchor_regex) =
        Regex::new(r#"(?is)<a\b[^>]*?href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a\s*>"#)
    else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    for captures in anchor_regex.captures_iter(html) {
        let Some(href) = captures.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };
        let link_text = captures
            .get(2)
            .map(|value| normalize_markup(value.as_str()))
            .unwrap_or_default();
        let Some(url) = canonical_same_origin_url(&base, href) else {
            continue;
        };
        if url == base {
            continue;
        }
        let classification_text = format!("{} {}", url, link_text).to_ascii_lowercase();
        let Some(document_kind) = classify_document(&classification_text) else {
            continue;
        };
        let title = if link_text.is_empty() {
            url.path()
                .rsplit('/')
                .next()
                .unwrap_or("document")
                .to_owned()
        } else {
            link_text
        };
        candidates.push(DocumentCandidate {
            company_id: company_id.to_owned(),
            source_kind,
            url: url.to_string(),
            title,
            document_kind,
            published_or_effective_date: None,
            provenance: format!(
                "discovered from {entry_url}; observed_at={}",
                observed_at.to_rfc3339()
            ),
        });
    }
    candidates.sort_by(|left, right| left.url.cmp(&right.url));
    candidates.dedup_by(|left, right| left.url == right.url);
    candidates.truncate(MAX_DOCUMENT_CANDIDATES_PER_ENTRY);
    candidates
}

/// Extracts a bounded title, explicit HTML time/date, and normalized body text.
pub fn document_metadata(html: &str, fallback_title: &str) -> (String, Option<NaiveDate>, String) {
    let title = Regex::new(r"(?is)<title\b[^>]*>(.*?)</title\s*>")
        .ok()
        .and_then(|regex| regex.captures(html))
        .and_then(|captures| {
            captures
                .get(1)
                .map(|value| normalize_markup(value.as_str()))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback_title.to_owned());
    let date = Regex::new(r#"(?i)<time\b[^>]*datetime\s*=\s*["'](\d{4}-\d{2}-\d{2})["'][^>]*>"#)
        .ok()
        .and_then(|regex| regex.captures(html))
        .and_then(|captures| captures.get(1))
        .and_then(|value| NaiveDate::parse_from_str(value.as_str(), "%Y-%m-%d").ok());
    (title, date, normalize_document_body(html))
}

fn canonical_same_origin_url(base: &Url, href: &str) -> Option<Url> {
    let mut url = base.join(href).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.scheme() != base.scheme()
        || url.host_str() != base.host_str()
        || url.port_or_known_default() != base.port_or_known_default()
        || url.username() != base.username()
        || url.password() != base.password()
    {
        return None;
    }
    url.set_fragment(None);
    Some(url)
}

fn classify_document(value: &str) -> Option<DocumentKind> {
    if contains_any(value, &["10-k", "10-q", "8-k", "filing", "sec.gov"]) {
        Some(DocumentKind::Filing)
    } else if contains_any(value, &["investor day", "investorday", "investor-day"]) {
        Some(DocumentKind::InvestorDay)
    } else if contains_any(
        value,
        &["earnings", "financial results", "shareholder letter"],
    ) {
        Some(DocumentKind::Earnings)
    } else if contains_any(value, &["engineering", "developer blog"]) {
        Some(DocumentKind::Engineering)
    } else if contains_any(
        value,
        &[
            "automation",
            "agent-first",
            "agent first",
            "artificial intelligence",
            " ai ",
        ],
    ) {
        Some(DocumentKind::AiAutomation)
    } else if contains_any(
        value,
        &["organization", "reorganiz", "restructur", "responsibility"],
    ) {
        Some(DocumentKind::Organization)
    } else if contains_any(value, &["product", "platform", "launch"]) {
        Some(DocumentKind::ProductPlatform)
    } else if contains_any(value, &["career", "careers", "job", "hiring"]) {
        Some(DocumentKind::Careers)
    } else {
        None
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn normalize_markup(value: &str) -> String {
    let tags = Regex::new(r"(?s)<[^>]+>").expect("valid tag regex");
    tags.replace_all(value, " ")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_document_body(value: &str) -> String {
    let mut without_non_content = value.to_owned();
    for tag in [
        "script", "style", "noscript", "h1", "h2", "h3", "h4", "h5", "h6", "nav", "header",
        "footer", "aside", "form",
    ] {
        let regex = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}\s*>"))
            .expect("valid non-content regex");
        without_non_content = regex.replace_all(&without_non_content, " ").into_owned();
    }
    let boilerplate_container = Regex::new(
        r#"(?is)<(?:div|section|ul|ol)\b[^>]*(?:class|id)\s*=\s*["'][^"']*(?:share|social|menu|breadcrumb|sidebar|navigation|footer|header)[^"']*["'][^>]*>.*?</(?:div|section|ul|ol)\s*>"#,
    )
    .expect("valid boilerplate container regex");
    without_non_content = boilerplate_container
        .replace_all(&without_non_content, " ")
        .into_owned();
    let without_metadata = Regex::new(r"(?is)<title\b[^>]*>.*?</title\s*>|<meta\b[^>]*>")
        .expect("valid metadata regex")
        .replace_all(&without_non_content, " ");
    let paragraphs = Regex::new(r"(?is)<p\b[^>]*>(.*?)</p\s*>")
        .expect("valid paragraph regex")
        .captures_iter(&without_metadata)
        .filter_map(|captures| captures.get(1))
        .map(|paragraph| normalize_markup(paragraph.as_str()))
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>();
    if !paragraphs.is_empty() {
        return paragraphs.join(" ");
    }
    normalize_markup(&without_metadata)
}
