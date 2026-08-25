//! Free, provider-specific source adapters behind provider-neutral observations.
//!
//! The adapters intentionally make a small, bounded number of public GET
//! requests. Provider response types and parsing stay in this module; callers
//! receive only normalized text, source classification, status, and provenance.

use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use serde::de::{DeserializeOwned, Error as DeError, IgnoredAny, SeqAccess, Visitor};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::marker::PhantomData;

use super::config::{is_safe_source_identifier, CompanyConfig};
use super::discovery::{
    discover_documents, document_metadata, DocumentCandidate, DocumentKind,
    MAX_DOCUMENT_OBSERVATIONS_PER_ENTRY,
};
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
    /// SEC EDGAR filing material.
    Sec,
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
            Self::Sec => "sec_filing",
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
    /// The source family does not apply because no primary source context is configured.
    NotApplicable,
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
            Self::NotApplicable => "NOT_APPLICABLE",
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

/// Role of the material represented by one source observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMaterialKind {
    /// A configured page that is an entry point to further research.
    EntryPoint,
    /// A bounded document discovered from an entry point or filing index.
    Document,
    /// A structured public hiring record.
    HiringRecord,
    /// A secondary article retained only as a discovery lead.
    DiscoveryArticle,
    /// An unavailable, unknown, or configuration-status observation.
    Status,
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
    material_kind: SourceMaterialKind,
    status: SourceStatus,
    tier: SourceTier,
    url: Option<String>,
    title: Option<String>,
    text: String,
    status_reason: String,
    document_kind: Option<DocumentKind>,
    provenance: Provenance,
}

struct SourceObservationInput {
    company_id: String,
    kind: SourceKind,
    material_kind: SourceMaterialKind,
    status: SourceStatus,
    tier: SourceTier,
    url: Option<String>,
    title: Option<String>,
    text: String,
    status_reason: String,
    document_kind: Option<DocumentKind>,
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
            material_kind: input.material_kind,
            status: input.status,
            tier: input.tier,
            url: input.url,
            title: input.title,
            text: input.text,
            status_reason: input.status_reason,
            document_kind: input.document_kind,
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

    /// Returns the material role represented by this observation.
    pub const fn material_kind(&self) -> SourceMaterialKind {
        self.material_kind
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

    /// Returns the deterministic class of a discovered document, when this
    /// observation represents one.
    pub const fn document_kind(&self) -> Option<DocumentKind> {
        self.document_kind
    }

    /// Returns normalized source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the safe, bounded reason for the retained source status.
    pub fn status_reason(&self) -> &str {
        &self.status_reason
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

/// Input required to build one official document observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentObservationInput {
    /// Company identity retained by the observation.
    pub company_id: String,
    /// Source family that exposed the document.
    pub kind: SourceKind,
    /// Canonical document URI.
    pub url: String,
    /// Document title or safe fallback label.
    pub title: String,
    /// Normalized document body.
    pub text: String,
    /// Retrieval status of the document.
    pub status: SourceStatus,
    /// Safe status explanation.
    pub status_reason: String,
    /// Deterministic document classification.
    pub document_kind: DocumentKind,
    /// Discovery or filing provenance field.
    pub source_field_or_passage: String,
    /// Observation timestamp.
    pub observed_at: DateTime<Utc>,
    /// Authoritative document date when available.
    pub effective_date: Option<NaiveDate>,
}

/// Builds one official document observation for the shared runtime path.
///
/// SEC filing documents and configured official documents use this factory so
/// they share the same status, provenance, and claim-extraction boundary.
pub fn document_observation(input: DocumentObservationInput) -> SourceObservation {
    SourceObservation::new(SourceObservationInput {
        company_id: input.company_id,
        kind: input.kind,
        material_kind: SourceMaterialKind::Document,
        status: input.status,
        tier: SourceTier::OfficialPrimary,
        url: Some(input.url.clone()),
        title: Some(input.title),
        text: input.text,
        status_reason: input.status_reason,
        document_kind: Some(input.document_kind),
        source_uri: input.url,
        source_field_or_passage: input.source_field_or_passage,
        observed_at: input.observed_at,
        effective_date: input.effective_date,
    })
}

/// Collects all configured official, hiring, and discovery observations for a
/// company using the injected HTTP boundary.
///
/// The number of requests is bounded to three configured official pages, up to
/// twelve documents per official entry point (including one nested pass), one
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
    } else {
        observations.push(not_applicable_observation(
            company,
            SourceKind::Gdelt,
            SourceTier::DiscoveryOnly,
            None,
            observed_at,
            "GDELT query context",
            "no configured primary source",
        ));
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
            let document_candidates =
                discover_documents(company.id(), kind, url, body, observed_at);
            let text = normalize_html_text(body);
            let status = if text.is_empty() {
                SourceStatus::Unknown
            } else {
                SourceStatus::Known
            };
            let status_reason = if text.is_empty() {
                "response contained no usable text"
            } else {
                "source returned usable text"
            };
            observations.push(SourceObservation::new(SourceObservationInput {
                company_id: company.id().to_owned(),
                kind,
                material_kind: SourceMaterialKind::EntryPoint,
                status,
                tier: SourceTier::OfficialPrimary,
                url: Some(url.to_owned()),
                title: None,
                text,
                status_reason: status_reason.to_owned(),
                document_kind: None,
                source_uri: url.to_owned(),
                source_field_or_passage: "official page text".to_owned(),
                observed_at,
                effective_date: None,
            }));
            collect_discovered_documents(
                company,
                http,
                observed_at,
                document_candidates,
                observations,
            );
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

fn collect_discovered_documents(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    candidates: Vec<DocumentCandidate>,
    observations: &mut Vec<SourceObservation>,
) {
    let mut seen_urls = candidates
        .iter()
        .map(|candidate| candidate.url().to_owned())
        .collect::<BTreeSet<_>>();
    let mut nested_candidates = Vec::new();

    for candidate in candidates {
        if let Some(body) = collect_document(company, http, observed_at, &candidate, observations) {
            nested_candidates.extend(discover_documents(
                company.id(),
                candidate.source_kind(),
                candidate.url(),
                &body,
                observed_at,
            ));
        }
    }

    for candidate in nested_candidates {
        if seen_urls.len() >= MAX_DOCUMENT_OBSERVATIONS_PER_ENTRY {
            break;
        }
        if !seen_urls.insert(candidate.url().to_owned()) {
            continue;
        }
        collect_document(company, http, observed_at, &candidate, observations);
    }
}

fn collect_document(
    company: &CompanyConfig,
    http: &dyn HttpClient,
    observed_at: DateTime<Utc>,
    candidate: &DocumentCandidate,
    observations: &mut Vec<SourceObservation>,
) -> Option<String> {
    let response = http.get(
        candidate.url(),
        &[(
            "Accept".to_owned(),
            "text/html,application/xhtml+xml".to_owned(),
        )],
    );
    let Some(response) = response.ok().filter(|response| response.is_success()) else {
        observations.push(discovered_document_status(
            company,
            candidate,
            observed_at,
            "discovered document request unavailable",
        ));
        return None;
    };
    let body = match bounded_body(&response) {
        Ok(body) => body,
        Err(FetchFailure::Unavailable) => {
            observations.push(discovered_document_status(
                company,
                candidate,
                observed_at,
                "discovered document response exceeds size limit",
            ));
            return None;
        }
        Err(FetchFailure::InvalidPayload) => unreachable!("body bounds do not decode"),
    };
    let (title, effective_date, text) = document_metadata(body, candidate.title());
    let status = if text.is_empty() {
        SourceStatus::Unknown
    } else {
        SourceStatus::Known
    };
    observations.push(document_observation(DocumentObservationInput {
        company_id: company.id().to_owned(),
        kind: candidate.source_kind(),
        url: candidate.url().to_owned(),
        title,
        text,
        status,
        status_reason: if status == SourceStatus::Known {
            "discovered document returned usable text".to_owned()
        } else {
            "discovered document contained no usable text".to_owned()
        },
        document_kind: candidate.document_kind(),
        source_field_or_passage: candidate.provenance().to_owned(),
        observed_at,
        effective_date,
    }));
    Some(body.to_owned())
}

fn discovered_document_status(
    company: &CompanyConfig,
    candidate: &DocumentCandidate,
    observed_at: DateTime<Utc>,
    reason: &str,
) -> SourceObservation {
    SourceObservation::new(SourceObservationInput {
        company_id: company.id().to_owned(),
        kind: candidate.source_kind(),
        material_kind: SourceMaterialKind::Document,
        status: SourceStatus::Unavailable,
        tier: SourceTier::OfficialPrimary,
        url: Some(candidate.url().to_owned()),
        title: Some(candidate.title().to_owned()),
        text: String::new(),
        status_reason: reason.to_owned(),
        document_kind: Some(candidate.document_kind()),
        source_uri: candidate.url().to_owned(),
        source_field_or_passage: candidate.provenance().to_owned(),
        observed_at,
        effective_date: candidate.published_or_effective_date(),
    })
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
            material_kind: SourceMaterialKind::HiringRecord,
            status: SourceStatus::Known,
            tier: SourceTier::StructuredHiring,
            url,
            title: Some(title),
            text,
            status_reason: "source returned usable hiring record".to_owned(),
            document_kind: None,
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
            material_kind: SourceMaterialKind::HiringRecord,
            status: SourceStatus::Known,
            tier: SourceTier::StructuredHiring,
            url,
            title: Some(title),
            text,
            status_reason: "source returned usable hiring record".to_owned(),
            document_kind: None,
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
            material_kind: SourceMaterialKind::DiscoveryArticle,
            status: SourceStatus::DiscoveryOnly,
            tier: SourceTier::DiscoveryOnly,
            url: Some(url.clone()),
            title: Some(title.clone()),
            text: title,
            status_reason: "discovery material only; not authoritative".to_owned(),
            document_kind: None,
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
    observation_with_status_reason(
        company,
        kind,
        tier,
        url,
        observed_at,
        field,
        (SourceStatus::Unavailable, field),
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
    observation_with_status_reason(
        company,
        kind,
        tier,
        url,
        observed_at,
        field,
        (SourceStatus::NotConfigured, field),
    )
}

fn not_applicable_observation(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
    reason: &str,
) -> SourceObservation {
    observation_with_status_reason(
        company,
        kind,
        tier,
        url,
        observed_at,
        field,
        (SourceStatus::NotApplicable, reason),
    )
}

fn observation_with_status_reason(
    company: &CompanyConfig,
    kind: SourceKind,
    tier: SourceTier,
    url: Option<&str>,
    observed_at: DateTime<Utc>,
    field: &str,
    status_and_reason: (SourceStatus, &str),
) -> SourceObservation {
    let (status, status_reason) = status_and_reason;
    let source_uri = url
        .map(str::to_owned)
        .unwrap_or_else(|| format!("source://weekly-radar/{}/{}", company.id(), kind.as_str()));
    SourceObservation::new(SourceObservationInput {
        company_id: company.id().to_owned(),
        kind,
        material_kind: SourceMaterialKind::Status,
        status,
        tier,
        url: url.map(str::to_owned),
        title: None,
        text: String::new(),
        status_reason: status_reason.to_owned(),
        document_kind: None,
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
        material_kind: SourceMaterialKind::Status,
        status: SourceStatus::Unknown,
        tier,
        url: url.map(str::to_owned),
        title: None,
        text: String::new(),
        status_reason: field.to_owned(),
        document_kind: None,
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
