//! Free, provider-specific source adapters behind provider-neutral observations.
//!
//! The adapters intentionally make a small, bounded number of public GET
//! requests. Provider response types and parsing stay in this module; callers
//! receive only normalized text, source classification, status, and provenance.

use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use serde::de::{DeserializeOwned, Error as DeError, IgnoredAny, SeqAccess, Visitor};
use serde::Deserialize;
use std::marker::PhantomData;

use super::config::{is_safe_source_identifier, CompanyConfig};
use super::http::{HttpClient, HttpResponse, MAX_HTTP_RESPONSE_BODY_BYTES};
use super::model::Provenance;

/// Maximum response body size accepted by source adapters.
pub const MAX_SOURCE_BODY_BYTES: usize = MAX_HTTP_RESPONSE_BODY_BYTES;
/// Maximum number of Greenhouse or Lever records retained from one response.
pub const MAX_HIRING_RECORDS: usize = 100;
const MAX_GDELT_ARTICLES: usize = 10;
const GDELT_ENDPOINT: &str = "https://api.gdeltproject.org/api/v2/doc/doc";
const GDELT_USER_AGENT: &str = "ORG-X weekly-radar source adapter";

/// Provider-neutral source family for one runtime observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Configured official investor-relations material.
    OfficialIr,
    /// Configured official careers material.
    Careers,
    /// Configured official engineering or AI material.
    EngineeringAiBlog,
    /// Public Greenhouse job-board material.
    Greenhouse,
    /// Public Lever job-board material.
    Lever,
    /// GDELT article discovery material.
    Gdelt,
}

impl SourceKind {
    /// Returns the stable source-family label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OfficialIr => "official_ir",
            Self::Careers => "careers",
            Self::EngineeringAiBlog => "engineering_ai_blog",
            Self::Greenhouse => "greenhouse",
            Self::Lever => "lever",
            Self::Gdelt => "gdelt",
        }
    }
}

/// Status of one source observation, independent of downstream fact status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceStatus {
    /// A usable deterministic source observation was retained.
    Known,
    /// The source responded, but no usable observation could be extracted.
    Unknown,
    /// The source was absent or could not be reached.
    Unavailable,
    /// The optional source was not configured for this company.
    NotConfigured,
    /// The record is intentionally limited to discovery and corroboration.
    DiscoveryOnly,
}

impl SourceStatus {
    /// Returns the stable status label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Known => "KNOWN",
            Self::Unknown => "UNKNOWN",
            Self::Unavailable => "UNAVAILABLE",
            Self::NotConfigured => "NOT_CONFIGURED",
            Self::DiscoveryOnly => "DISCOVERY_ONLY",
        }
    }
}

/// Authority tier assigned by the source adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceTier {
    /// Official company material used as primary evidence.
    OfficialPrimary,
    /// Structured public hiring material.
    StructuredHiring,
    /// Secondary discovery material that cannot be authoritative here.
    DiscoveryOnly,
}

impl SourceTier {
    /// Returns the stable authority-tier label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OfficialPrimary => "OFFICIAL_PRIMARY",
            Self::StructuredHiring => "STRUCTURED_HIRING",
            Self::DiscoveryOnly => "DISCOVERY_ONLY",
        }
    }

    /// Returns whether this tier can be used as primary authority by this
    /// adapter boundary.
    pub const fn is_authoritative(self) -> bool {
        matches!(self, Self::OfficialPrimary)
    }
}

/// Provider-neutral source material with explicit status and provenance.
///
/// This is an output-only observation. It intentionally implements `Serialize`
/// but not `Deserialize`, so serialized snapshots cannot be supplied back to
/// the public runtime boundary as forged source observations.
///
/// ```compile_fail
/// use org_x::features::weekly_radar::runtime::sources::SourceObservation;
/// let _: SourceObservation = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct SourceObservation {
    company_id: String,
    kind: SourceKind,
    status: SourceStatus,
    tier: SourceTier,
    url: Option<String>,
    title: Option<String>,
    text: String,
    provenance: Provenance,
}

struct SourceObservationInput {
    company_id: String,
    kind: SourceKind,
    status: SourceStatus,
    tier: SourceTier,
    url: Option<String>,
    title: Option<String>,
    text: String,
    source_uri: String,
    source_field_or_passage: String,
    observed_at: DateTime<Utc>,
    effective_date: Option<NaiveDate>,
}

impl SourceObservation {
    fn new(input: SourceObservationInput) -> Self {
        let provenance = Provenance::new(
            input.source_uri,
            input.source_field_or_passage,
            input.observed_at,
            input.effective_date,
        )
        .expect("source adapter must construct nonblank provenance");
        Self {
            company_id: input.company_id,
            kind: input.kind,
            status: input.status,
            tier: input.tier,
            url: input.url,
            title: input.title,
            text: input.text,
            provenance,
        }
    }

    /// Returns the configured company identity.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the source family.
    pub const fn kind(&self) -> SourceKind {
        self.kind
    }

    /// Returns the source status.
    pub const fn status(&self) -> SourceStatus {
        self.status
    }

    /// Returns the source authority tier.
    pub const fn tier(&self) -> SourceTier {
        self.tier
    }

    /// Returns the source family as a stable label.
    pub const fn source_kind(&self) -> SourceKind {
        self.kind()
    }

    /// Returns the source status as a stable enum.
    pub const fn source_status(&self) -> SourceStatus {
        self.status()
    }

    /// Returns the source tier as a stable enum.
    pub const fn source_tier(&self) -> SourceTier {
        self.tier()
    }

    /// Returns the optional record URL. Unconfigured sources never get a
    /// guessed URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the optional source title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns normalized source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns complete source provenance.
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Returns whether this observation is eligible as primary authority.
    pub const fn is_authoritative(&self) -> bool {
        self.tier.is_authoritative()
    }

    /// Returns whether this observation is explicitly discovery-only.
    pub const fn is_discovery_only(&self) -> bool {
        matches!(self.tier, SourceTier::DiscoveryOnly)
    }
}

/// Collects all configured official, hiring, and discovery observations for a
/// company using the injected HTTP boundary.
///
/// The number of requests is bounded to three configured official pages, one
/// Greenhouse endpoint, one Lever endpoint, and, when a company source
/// endpoint is configured, one GDELT query. Individual failures become
/// explicit source statuses so optional source gaps do not abort collection
/// for the remaining sources.
pub fn collect_configured_sources(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
) -> Vec<SourceObservation> {
    let mut observations = Vec::new();
    collect_official(
        company,
        http,
        observed_at,
        SourceKind::OfficialIr,
        company.official_ir_url(),
        &mut observations,
    );
    collect_official(
        company,
        http,
        observed_at,
        SourceKind::Careers,
        company.careers_url(),
        &mut observations,
    );
    collect_official(
        company,
        http,
        observed_at,
        SourceKind::EngineeringAiBlog,
        company.engineering_ai_blog_url(),
        &mut observations,
    );
    collect_greenhouse(company, http, observed_at, &mut observations);
    collect_lever(company, http, observed_at, &mut observations);
    if has_configured_source_endpoint(company) {
        collect_gdelt(company, http, observed_at, &mut observations);
    }
    observations
}

fn has_configured_source_endpoint(company: &CompanyConfig) -> bool {
    company.official_ir_url().is_some()
        || company.careers_url().is_some()
        || company.engineering_ai_blog_url().is_some()
        || company.greenhouse_board().is_some()
        || company.lever_site().is_some()
}

fn collect_official(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    kind: SourceKind,
    configured_url: Option<&str>,
    observations: &mut Vec<SourceObservation>,
) {
    let Some(url) = configured_url else {
        observations.push(not_configured_observation(
            company,
            kind,
            SourceTier::OfficialPrimary,
            None,
            observed_at,
            "optional official source is not configured",
        ));
        return;
    };

    let result = http.get(
        url,
        &[(
            "Accept".to_owned(),
            "text/html,application/xhtml+xml".to_owned(),
        )],
    );
    match result {
        Ok(response) if response.is_success() => {
            let body = match bounded_body(&response) {
                Ok(body) => body,
                Err(FetchFailure::Unavailable) => {
                    observations.push(unavailable_observation(
                        company,
                        kind,
                        SourceTier::OfficialPrimary,
                        Some(url),
                        observed_at,
                        "official page response exceeds size limit",
                    ));
                    return;
                }
                Err(FetchFailure::InvalidPayload) => unreachable!("body bounds do not decode"),
            };
            let text = normalize_html_text(body);
            let status = if text.is_empty() {
                SourceStatus::Unknown
            } else {
                SourceStatus::Known
            };
            observations.push(SourceObservation::new(SourceObservationInput {
                company_id: company.id().to_owned(),
                kind,
                status,
                tier: SourceTier::OfficialPrimary,
                url: Some(url.to_owned()),
                title: None,
                text,
                source_uri: url.to_owned(),
                source_field_or_passage: "official page text".to_owned(),
                observed_at,
                effective_date: None,
            }));
        }
        Ok(_) | Err(_) => observations.push(unavailable_observation(
            company,
            kind,
            SourceTier::OfficialPrimary,
            Some(url),
            observed_at,
            "official page request unavailable",
        )),
    }
}

fn collect_greenhouse(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    observations: &mut Vec<SourceObservation>,
) {
    let Some(board) = company.greenhouse_board() else {
        observations.push(not_configured_observation(
            company,
            SourceKind::Greenhouse,
            SourceTier::StructuredHiring,
            None,
            observed_at,
            "optional Greenhouse board is not configured",
        ));
        return;
    };
    if !is_safe_source_identifier(board) {
        observations.push(unavailable_observation(
            company,
            SourceKind::Greenhouse,
            SourceTier::StructuredHiring,
            None,
            observed_at,
            "Greenhouse board identifier is unsafe",
        ));
        return;
    }

    let endpoint = format!("https://boards-api.greenhouse.io/v1/boards/{board}/jobs?content=true");
    let response = match get_json::<GreenhouseResponse>(http, &endpoint) {
        Ok(response) => response,
        Err(FetchFailure::Unavailable) => {
            observations.push(unavailable_observation(
                company,
                SourceKind::Greenhouse,
                SourceTier::StructuredHiring,
                Some(&endpoint),
                observed_at,
                "Greenhouse request unavailable",
            ));
            return;
        }
        Err(FetchFailure::InvalidPayload) => {
            observations.push(unknown_observation(
                company,
                SourceKind::Greenhouse,
                SourceTier::StructuredHiring,
                Some(&endpoint),
                observed_at,
                "greenhouse payload",
            ));
            return;
        }
    };

    let mut retained = 0;
    for job in response.jobs.into_records() {
        let title = normalize_plain_text(&job.title);
        if title.is_empty() {
            continue;
        }
        let text = job
            .content
            .as_deref()
            .map(normalize_html_text)
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| title.clone());
        let url = job.absolute_url.clone();
        let source_uri = url.clone().unwrap_or_else(|| endpoint.clone());
        let field = format!("greenhouse.jobs[{}].content", job.id);
        observations.push(SourceObservation::new(SourceObservationInput {
            company_id: company.id().to_owned(),
            kind: SourceKind::Greenhouse,
            status: SourceStatus::Known,
            tier: SourceTier::StructuredHiring,
            url,
            title: Some(title),
            text,
            source_uri,
            source_field_or_passage: field,
            observed_at,
            effective_date: job.updated_at.as_deref().and_then(parse_rfc3339_date),
        }));
        retained += 1;
    }
    if retained == 0 {
        observations.push(unknown_observation(
            company,
            SourceKind::Greenhouse,
            SourceTier::StructuredHiring,
            Some(&endpoint),
            observed_at,
            "greenhouse jobs list",
        ));
    }
}

fn collect_lever(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    observations: &mut Vec<SourceObservation>,
) {
    let Some(site) = company.lever_site() else {
        observations.push(not_configured_observation(
            company,
            SourceKind::Lever,
            SourceTier::StructuredHiring,
            None,
            observed_at,
            "optional Lever site is not configured",
        ));
        return;
    };
    if !is_safe_source_identifier(site) {
        observations.push(unavailable_observation(
            company,
            SourceKind::Lever,
            SourceTier::StructuredHiring,
            None,
            observed_at,
            "Lever site identifier is unsafe",
        ));
        return;
    }

    let endpoint = format!("https://api.lever.co/v0/postings/{site}?mode=json");
    let postings = match get_json::<LimitedSequence<LeverPosting>>(http, &endpoint) {
        Ok(postings) => postings,
        Err(FetchFailure::Unavailable) => {
            observations.push(unavailable_observation(
                company,
                SourceKind::Lever,
                SourceTier::StructuredHiring,
                Some(&endpoint),
                observed_at,
                "Lever request unavailable",
            ));
            return;
        }
        Err(FetchFailure::InvalidPayload) => {
            observations.push(unknown_observation(
                company,
                SourceKind::Lever,
                SourceTier::StructuredHiring,
                Some(&endpoint),
                observed_at,
                "lever payload",
            ));
            return;
        }
    };

    let mut retained = 0;
    for posting in postings.into_records() {
        let title = normalize_plain_text(&posting.text);
        if title.is_empty() {
            continue;
        }
        let (text, field) = match posting
            .description_plain
            .as_deref()
            .map(normalize_plain_text)
            .filter(|text| !text.is_empty())
        {
            Some(text) => (text, "descriptionPlain".to_owned()),
            None => (
                posting
                    .description
                    .as_deref()
                    .map(normalize_html_text)
                    .unwrap_or_else(|| title.clone()),
                "description".to_owned(),
            ),
        };
        let url = posting.hosted_url.or(posting.apply_url);
        let source_uri = url.clone().unwrap_or_else(|| endpoint.clone());
        let field = format!("lever.postings[{}].{field}", posting.id);
        observations.push(SourceObservation::new(SourceObservationInput {
            company_id: company.id().to_owned(),
            kind: SourceKind::Lever,
            status: SourceStatus::Known,
            tier: SourceTier::StructuredHiring,
            url,
            title: Some(title),
            text,
            source_uri,
            source_field_or_passage: field,
            observed_at,
            effective_date: posting
                .updated_at
                .or(posting.created_at)
                .and_then(parse_epoch_date),
        }));
        retained += 1;
    }
    if retained == 0 {
        observations.push(unknown_observation(
            company,
            SourceKind::Lever,
            SourceTier::StructuredHiring,
            Some(&endpoint),
            observed_at,
            "lever postings list",
        ));
    }
}

fn collect_gdelt(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    observations: &mut Vec<SourceObservation>,
) {
    let endpoint = gdelt_endpoint(company.name());
    let response = match get_json::<GdeltResponse>(http, &endpoint) {
        Ok(response) => response,
        Err(FetchFailure::Unavailable) => {
            observations.push(unavailable_observation(
                company,
                SourceKind::Gdelt,
                SourceTier::DiscoveryOnly,
                Some(&endpoint),
                observed_at,
                "GDELT discovery request unavailable",
            ));
            return;
        }
        Err(FetchFailure::InvalidPayload) => {
            observations.push(unknown_observation(
                company,
                SourceKind::Gdelt,
                SourceTier::DiscoveryOnly,
                Some(&endpoint),
                observed_at,
                "GDELT query context",
            ));
            return;
        }
    };

    let mut retained = 0;
    for article in response.articles.into_iter().take(MAX_GDELT_ARTICLES) {
        let (Some(url), Some(title)) = (article.url, article.title) else {
            continue;
        };
        let title = normalize_plain_text(&title);
        if title.is_empty() || url.trim().is_empty() {
            continue;
        }
        observations.push(SourceObservation::new(SourceObservationInput {
            company_id: company.id().to_owned(),
            kind: SourceKind::Gdelt,
            status: SourceStatus::DiscoveryOnly,
            tier: SourceTier::DiscoveryOnly,
            url: Some(url.clone()),
            title: Some(title.clone()),
            text: title,
            source_uri: url,
            source_field_or_passage: format!("GDELT query context: {endpoint}"),
            observed_at,
            effective_date: article.seendate.as_deref().and_then(parse_gdelt_date),
        }));
        retained += 1;
    }
    if retained == 0 {
        observations.push(unknown_observation(
            company,
            SourceKind::Gdelt,
            SourceTier::DiscoveryOnly,
            Some(&endpoint),
            observed_at,
            "GDELT query context",
        ));
    }
}

fn unavailable_observation(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
) -> SourceObservation {
    observation_with_status(
        company,
        kind,
        tier,
        url,
        observed_at,
        field,
        SourceStatus::Unavailable,
    )
}

fn not_configured_observation(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
) -> SourceObservation {
    observation_with_status(
        company,
        kind,
        tier,
        url,
        observed_at,
        field,
        SourceStatus::NotConfigured,
    )
}

fn observation_with_status(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
    status: SourceStatus,
) -> SourceObservation {
    let source_uri = url
        .map(str::to_owned)
        .unwrap_or_else(|| format!("source://weekly-radar/{}/{}", company.id(), kind.as_str()));
    SourceObservation::new(SourceObservationInput {
        company_id: company.id().to_owned(),
        kind,
        status,
        tier,
        url: url.map(str::to_owned),
        title: None,
        text: String::new(),
        source_uri,
        source_field_or_passage: field.to_owned(),
        observed_at,
        effective_date: None,
    })
}

fn unknown_observation(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
) -> SourceObservation {
    let source_uri = url
        .map(str::to_owned)
        .unwrap_or_else(|| format!("source://weekly-radar/{}/{}", company.id(), kind.as_str()));
    SourceObservation::new(SourceObservationInput {
        company_id: company.id().to_owned(),
        kind,
        status: SourceStatus::Unknown,
        tier,
        url: url.map(str::to_owned),
        title: None,
        text: String::new(),
        source_uri,
        source_field_or_passage: field.to_owned(),
        observed_at,
        effective_date: None,
    })
}

enum FetchFailure {
    Unavailable,
    InvalidPayload,
}

fn bounded_body(response: &HttpResponse) -> Result<&str, FetchFailure> {
    if response.body().len() > MAX_SOURCE_BODY_BYTES {
        return Err(FetchFailure::Unavailable);
    }
    Ok(response.body())
}

fn get_json<T: DeserializeOwned>(http: &dyn HttpClient, url: &str) -> Result<T, FetchFailure> {
    let response = http
        .get(
            url,
            &[
                ("Accept".to_owned(), "application/json".to_owned()),
                ("User-Agent".to_owned(), GDELT_USER_AGENT.to_owned()),
            ],
        )
        .map_err(|_| FetchFailure::Unavailable)?;
    if !response.is_success() {
        return Err(FetchFailure::Unavailable);
    }
    let body = bounded_body(&response)?;
    serde_json::from_str(body).map_err(|_| FetchFailure::InvalidPayload)
}

fn normalize_html_text(html: &str) -> String {
    let scripts = Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").expect("valid script regex");
    let styles = Regex::new(r"(?is)<style\b[^>]*>.*?</style\s*>").expect("valid style regex");
    let tags = Regex::new(r"(?s)<[^>]+>").expect("valid HTML tag regex");
    let text = scripts.replace_all(html, " ");
    let text = styles.replace_all(&text, " ");
    let text = tags.replace_all(&text, " ");
    normalize_plain_text(&decode_html_entities(&text))
}

fn normalize_plain_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}

fn gdelt_endpoint(company_name: &str) -> String {
    format!(
        "{GDELT_ENDPOINT}?query=%22{}%22&mode=artlist&format=json&maxrecords={MAX_GDELT_ARTICLES}&sort=HybridRel",
        percent_encode_query(company_name)
    )
}

fn percent_encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!("nibble is at most four bits"),
    }
}

fn parse_rfc3339_date(value: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.date_naive())
}

fn parse_epoch_date(value: i64) -> Option<NaiveDate> {
    DateTime::from_timestamp_millis(value).map(|date| date.date_naive())
}

fn parse_gdelt_date(value: &str) -> Option<NaiveDate> {
    value
        .get(..8)
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y%m%d").ok())
}

#[derive(Debug, Deserialize)]
struct GreenhouseResponse {
    jobs: LimitedSequence<GreenhouseJob>,
}

#[derive(Debug)]
struct LimitedSequence<T> {
    records: Vec<T>,
}

impl<'de, T> Deserialize<'de> for LimitedSequence<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LimitedSequenceVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for LimitedSequenceVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = LimitedSequence<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON array within the configured record limit")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut records = Vec::with_capacity(MAX_HIRING_RECORDS);
                loop {
                    if records.len() == MAX_HIRING_RECORDS {
                        let extra = sequence.next_element::<IgnoredAny>()?;
                        if extra.is_some() {
                            return Err(A::Error::custom("source record limit exceeded"));
                        }
                        break;
                    }

                    match sequence.next_element::<T>()? {
                        Some(record) => records.push(record),
                        None => break,
                    }
                }
                Ok(LimitedSequence { records })
            }
        }

        deserializer.deserialize_seq(LimitedSequenceVisitor(PhantomData))
    }
}

impl<T> LimitedSequence<T> {
    fn into_records(self) -> Vec<T> {
        self.records
    }
}

#[derive(Debug, Deserialize)]
struct GreenhouseJob {
    id: u64,
    title: String,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    absolute_url: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LeverPosting {
    id: String,
    text: String,
    #[serde(default, rename = "hostedUrl")]
    hosted_url: Option<String>,
    #[serde(default, rename = "applyUrl")]
    apply_url: Option<String>,
    #[serde(default, rename = "descriptionPlain")]
    description_plain: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "createdAt")]
    created_at: Option<i64>,
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GdeltResponse {
    #[serde(default)]
    articles: Vec<GdeltArticle>,
}

#[derive(Debug, Deserialize)]
struct GdeltArticle {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    seendate: Option<String>,
}
