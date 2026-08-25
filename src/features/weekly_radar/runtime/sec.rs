//! SEC EDGAR/Company Facts acquisition with provider-private payload models.

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::config::CompanyConfig;
use super::discovery::document_metadata;
use super::error::RuntimeError;
use super::http::HttpClient;
use super::model::{Confidence, FactStatus, NormalizedFact, Provenance};
use super::rules::extract_employee_candidate;

const SEC_SUBMISSIONS_ROOT: &str = "https://data.sec.gov/submissions";
const SEC_FACTS_ROOT: &str = "https://data.sec.gov/api/xbrl/companyfacts";
const SEC_ARCHIVES_ROOT: &str = "https://www.sec.gov/Archives/edgar/data";

/// Maximum number of recent filing documents retained as discovery candidates.
pub const MAX_SEC_DOCUMENT_CANDIDATES: usize = 3;

/// Finite response limit for SEC JSON payloads parsed by the runtime.
///
/// Company Facts and submissions contain complete registrant histories and
/// are materially larger than the default limit used for ordinary public
/// pages. The limit remains bounded so an unexpectedly large response still
/// fails closed before it can cause an unbounded allocation.
pub const SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Finite response limit for individual SEC filing documents.
pub const SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES: usize = 8 * 1024 * 1024;

// SEC submissions uses the same finite JSON envelope as Company Facts. Keep
// this adapter-specific alias separate from the generic transport limit so
// ordinary public pages and discovery sources remain capped at 1 MiB.
const SEC_SUBMISSIONS_MAX_RESPONSE_BODY_BYTES: usize = SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES;

const REVENUE_ALIASES: &[&str] = &[
    "RevenueFromContractWithCustomerExcludingAssessedTax",
    "Revenues",
    "SalesRevenueNet",
];
const OPERATING_INCOME_ALIASES: &[&str] = &["OperatingIncomeLoss"];
const NET_INCOME_ALIASES: &[&str] = &["NetIncomeLoss", "ProfitLoss"];
const OPERATING_CASH_FLOW_ALIASES: &[&str] = &["NetCashProvidedByUsedInOperatingActivities"];
const CAPEX_ALIASES: &[&str] = &[
    "PaymentsToAcquirePropertyPlantAndEquipment",
    "PaymentsToAcquireProductiveAssets",
    "CapitalExpendituresIncurredButNotYetPaid",
];
const R_AND_D_ALIASES: &[&str] = &[
    "ResearchAndDevelopmentExpense",
    "ResearchAndDevelopmentExpenseExcludingAcquiredInProcessCost",
    "ResearchAndDevelopmentExpenseIncludingAcquiredInProcessCost",
];
const SBC_ALIASES: &[&str] = &[
    "ShareBasedCompensation",
    "AllocatedShareBasedCompensationExpense",
    "ShareBasedCompensationArrangementByShareBasedPaymentAwardEquityInstrumentsOtherThanOptionsGrantsInPeriodTotal",
];
const EMPLOYEE_ALIASES: &[&str] = &[
    "EntityNumberOfEmployees",
    "NumberOfEmployees",
    "EmployeeCount",
];

const FACT_SPECS: &[FactSpec] = &[
    FactSpec {
        kind: "revenue",
        aliases: REVENUE_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "operating_income",
        aliases: OPERATING_INCOME_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "net_income",
        aliases: NET_INCOME_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "operating_cash_flow",
        aliases: OPERATING_CASH_FLOW_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "capex",
        aliases: CAPEX_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "r_and_d",
        aliases: R_AND_D_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "sbc",
        aliases: SBC_ALIASES,
        units: UnitKind::Currency,
    },
    FactSpec {
        kind: "employee_count",
        aliases: EMPLOYEE_ALIASES,
        units: UnitKind::Employees,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompanyEvidence {
    company_id: String,
    facts: Vec<NormalizedFact>,
    documents: Vec<SecDocumentCandidate>,
    failures: Vec<SecStageFailure>,
}

/// Safe diagnostic for one independently acquired SEC stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecStageFailure {
    stage: String,
    reason: String,
}

/// Safe retrieval status for one SEC filing document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecDocumentStatus {
    /// The filing body contained usable normalized text.
    Known,
    /// The filing response succeeded but did not contain usable text.
    Unknown,
    /// The filing could not be retrieved or exceeded its finite limit.
    Unavailable,
}

impl SecStageFailure {
    fn new(stage: &'static str, error: &RuntimeError) -> Self {
        let reason = match error {
            RuntimeError::JsonDecode { .. } => "invalid JSON response",
            RuntimeError::HttpRequest => "HTTP request unavailable",
            RuntimeError::HttpResponse => "HTTP response unavailable",
            RuntimeError::HttpResponseTooLarge => "response exceeded finite limit",
            RuntimeError::FixtureMissing => "fixture response unavailable",
            RuntimeError::FixtureState => "fixture transport unavailable",
            RuntimeError::InvalidConfiguration { .. }
            | RuntimeError::InvalidModel { .. }
            | RuntimeError::ConfigurationIo { .. } => "SEC response could not be normalized",
        };
        Self {
            stage: stage.to_owned(),
            reason: reason.to_owned(),
        }
    }

    /// Returns the stable SEC stage label.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// Returns the safe failure category without response payloads.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// One bounded filing document candidate derived from SEC submissions metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecDocumentCandidate {
    accession_number: String,
    form: String,
    filing_date: NaiveDate,
    report_date: Option<NaiveDate>,
    primary_document: String,
    source_uri: String,
    title: String,
    text: String,
    status: SecDocumentStatus,
    status_reason: String,
}

impl SecDocumentCandidate {
    /// Returns the SEC accession identity.
    pub fn accession_number(&self) -> &str {
        &self.accession_number
    }

    /// Returns the filing form.
    pub fn form(&self) -> &str {
        &self.form
    }

    /// Returns the filing date.
    pub const fn filing_date(&self) -> NaiveDate {
        self.filing_date
    }

    /// Returns the optional report period end date.
    pub const fn report_date(&self) -> Option<NaiveDate> {
        self.report_date
    }

    /// Returns the validated primary document name.
    pub fn primary_document(&self) -> &str {
        &self.primary_document
    }

    /// Returns the SEC archive URI constructed from validated metadata.
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Returns the normalized filing title, or the SEC primary-document name.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns bounded normalized filing text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the safe filing-body retrieval status.
    pub const fn status(&self) -> SecDocumentStatus {
        self.status
    }

    /// Returns the safe reason associated with the filing-body status.
    pub fn status_reason(&self) -> &str {
        &self.status_reason
    }

    fn record_body(&mut self, body: &str) {
        let (title, _body_date, text) = document_metadata(body, self.primary_document());
        self.title = title;
        self.text = text;
        self.status = if self.text.is_empty() {
            SecDocumentStatus::Unknown
        } else {
            SecDocumentStatus::Known
        };
        self.status_reason = if self.status == SecDocumentStatus::Known {
            "SEC filing returned usable text".to_owned()
        } else {
            "SEC filing contained no usable text".to_owned()
        };
    }

    fn record_failure(&mut self, reason: &str) {
        self.title = self.primary_document.clone();
        self.text.clear();
        self.status = SecDocumentStatus::Unavailable;
        self.status_reason = reason.to_owned();
    }
}

impl CompanyEvidence {
    fn new(
        company_id: impl Into<String>,
        facts: Vec<NormalizedFact>,
        documents: Vec<SecDocumentCandidate>,
        failures: Vec<SecStageFailure>,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            facts,
            documents,
            failures,
        }
    }

    /// Returns the configured company identity.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns normalized facts in stable concept order.
    pub fn facts(&self) -> &[NormalizedFact] {
        &self.facts
    }

    /// Returns one normalized fact by its provider-neutral kind.
    pub fn fact(&self, kind: &str) -> Option<&NormalizedFact> {
        self.facts.iter().find(|fact| fact.kind() == kind)
    }

    /// Returns bounded filing document candidates.
    pub fn documents(&self) -> &[SecDocumentCandidate] {
        &self.documents
    }

    /// Returns safe failures for independently acquired SEC stages.
    pub fn failures(&self) -> &[SecStageFailure] {
        &self.failures
    }
}

/// SEC adapter that collects Company Facts and the latest 10-K when needed.
#[derive(Clone, Copy, Debug, Default)]
pub struct SecClient;

impl SecClient {
    /// Collects normalized SEC evidence for one configured company.
    pub fn collect(
        company: &CompanyConfig,
        http: &dyn HttpClient,
        user_agent: &str,
    ) -> Result<CompanyEvidence, RuntimeError> {
        if user_agent.trim().is_empty() {
            return Err(RuntimeError::invalid_configuration(
                "SEC User-Agent cannot be blank",
            ));
        }
        let cik = company.sec_cik().ok_or_else(|| {
            RuntimeError::invalid_configuration(format!(
                "SEC CIK is required for company {}",
                company.id()
            ))
        })?;
        let cik_path = cik.trim_start_matches('0');
        let cik_path = if cik_path.is_empty() { "0" } else { cik_path };
        let submissions_url = format!("{SEC_SUBMISSIONS_ROOT}/CIK{cik}.json");
        let facts_url = format!("{SEC_FACTS_ROOT}/CIK{cik}.json");
        let mut failures = Vec::new();
        let submissions: Option<SubmissionsDocument> = match get_json(
            http,
            &submissions_url,
            user_agent,
            SEC_SUBMISSIONS_MAX_RESPONSE_BODY_BYTES,
            "SEC submissions",
        ) {
            Ok(value) => Some(value),
            Err(error) => {
                failures.push(SecStageFailure::new("submissions", &error));
                None
            }
        };
        let facts: Option<CompanyFactsDocument> = match get_json(
            http,
            &facts_url,
            user_agent,
            SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES,
            "SEC Company Facts",
        ) {
            Ok(value) => Some(value),
            Err(error) => {
                failures.push(SecStageFailure::new("company_facts", &error));
                None
            }
        };
        let mut documents = submissions
            .as_ref()
            .map(|value| value.filing_documents(cik_path))
            .unwrap_or_default();
        for document in &mut documents {
            match get_response_body(
                http,
                document.source_uri(),
                user_agent,
                SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES,
            ) {
                Ok(body) => document.record_body(&body),
                Err(error) => {
                    let failure = SecStageFailure::new("filing_document", &error);
                    document.record_failure(failure.reason());
                    failures.push(failure);
                }
            }
        }
        let latest_filing = submissions
            .as_ref()
            .and_then(SubmissionsDocument::latest_10k);
        let retrieved_at = Utc::now();
        let mut normalized = Vec::with_capacity(FACT_SPECS.len());

        for spec in FACT_SPECS {
            if let Some(facts) = facts.as_ref() {
                if let Some(selected) = select_annual_observation(facts, spec) {
                    let Some(value) = value_as_text(&selected.observation.val) else {
                        failures.push(SecStageFailure {
                            stage: "company_facts".to_owned(),
                            reason: "annual value was not scalar".to_owned(),
                        });
                        normalized.push(unknown_fact(
                            company,
                            spec.kind,
                            facts_url.clone(),
                            "SEC Company Facts: annual value was not scalar",
                            parse_date(selected.observation.end.as_deref()),
                            retrieved_at,
                        )?);
                        continue;
                    };
                    let effective_date = parse_date(selected.observation.end.as_deref());
                    let provenance = Provenance::new(
                        facts_url.clone(),
                        observation_description(&selected, &value),
                        retrieved_at,
                        effective_date,
                    )?;
                    normalized.push(NormalizedFact::new(
                        company.id(),
                        spec.kind,
                        value,
                        FactStatus::Known,
                        Confidence::High,
                        provenance,
                    )?);
                    continue;
                }
            }

            if spec.kind == "employee_count" {
                if let Some(filing) = latest_filing.as_ref() {
                    let filing_url = filing.url(cik_path);
                    let body = if let Some(document) = documents
                        .iter()
                        .find(|document| document.accession_number() == filing.accession_number)
                    {
                        match document.status() {
                            SecDocumentStatus::Known => document.text().to_owned(),
                            SecDocumentStatus::Unknown => {
                                normalized.push(unknown_fact(
                                    company,
                                    spec.kind,
                                    filing_url,
                                    "SEC filing document contained no usable text",
                                    filing.report_date(),
                                    retrieved_at,
                                )?);
                                continue;
                            }
                            SecDocumentStatus::Unavailable => {
                                normalized.push(unavailable_fact(
                                    company,
                                    spec.kind,
                                    filing_url,
                                    "SEC filing document unavailable",
                                    filing.report_date(),
                                    retrieved_at,
                                )?);
                                continue;
                            }
                        }
                    } else {
                        match get_response_body(
                            http,
                            &filing_url,
                            user_agent,
                            SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES,
                        ) {
                            Ok(body) => document_metadata(&body, &filing.primary_document).2,
                            Err(error) => {
                                failures.push(SecStageFailure::new("filing_document", &error));
                                normalized.push(unavailable_fact(
                                    company,
                                    spec.kind,
                                    filing_url,
                                    "SEC filing document unavailable",
                                    filing.report_date(),
                                    retrieved_at,
                                )?);
                                continue;
                            }
                        }
                    };
                    if let Some(candidate) =
                        extract_employee_candidate(&body, filing.report_date(), &filing_url)
                    {
                        let provenance = Provenance::new(
                            filing_url,
                            candidate.passage.clone(),
                            retrieved_at,
                            candidate.effective_date,
                        )?;
                        normalized.push(NormalizedFact::new(
                            company.id(),
                            spec.kind,
                            candidate.value,
                            FactStatus::Known,
                            if candidate.approximate {
                                Confidence::Approximate
                            } else {
                                Confidence::High
                            },
                            provenance,
                        )?);
                        continue;
                    }
                    normalized.push(unknown_fact(
                        company,
                        spec.kind,
                        filing_url,
                        "SEC 10-K employee passage: no unambiguous workforce candidate",
                        filing.report_date(),
                        retrieved_at,
                    )?);
                    continue;
                }
            }

            let status = if facts.is_none() {
                FactStatus::Unavailable
            } else {
                FactStatus::Unknown
            };
            let passage = if status == FactStatus::Unavailable {
                "SEC Company Facts unavailable"
            } else if submissions.is_none() {
                "SEC submissions unavailable; no filing metadata"
            } else {
                "SEC Company Facts: no unambiguous annual value"
            };
            normalized.push(missing_fact(
                company,
                spec.kind,
                facts_url.clone(),
                passage,
                latest_filing.as_ref().and_then(FilingMetadata::report_date),
                retrieved_at,
                status,
            )?);
        }

        Ok(CompanyEvidence::new(
            company.id(),
            normalized,
            documents,
            failures,
        ))
    }
}

fn unknown_fact(
    company: &CompanyConfig,
    kind: &str,
    source_uri: String,
    source_field_or_passage: &str,
    effective_date: Option<NaiveDate>,
    retrieved_at: chrono::DateTime<Utc>,
) -> Result<NormalizedFact, RuntimeError> {
    missing_fact(
        company,
        kind,
        source_uri,
        source_field_or_passage,
        effective_date,
        retrieved_at,
        FactStatus::Unknown,
    )
}

fn unavailable_fact(
    company: &CompanyConfig,
    kind: &str,
    source_uri: String,
    source_field_or_passage: &str,
    effective_date: Option<NaiveDate>,
    retrieved_at: chrono::DateTime<Utc>,
) -> Result<NormalizedFact, RuntimeError> {
    missing_fact(
        company,
        kind,
        source_uri,
        source_field_or_passage,
        effective_date,
        retrieved_at,
        FactStatus::Unavailable,
    )
}

fn missing_fact(
    company: &CompanyConfig,
    kind: &str,
    source_uri: String,
    source_field_or_passage: &str,
    effective_date: Option<NaiveDate>,
    retrieved_at: chrono::DateTime<Utc>,
    status: FactStatus,
) -> Result<NormalizedFact, RuntimeError> {
    let provenance = Provenance::new(
        source_uri,
        source_field_or_passage,
        retrieved_at,
        effective_date,
    )?;
    NormalizedFact::without_value(company.id(), kind, status, Confidence::Unknown, provenance)
}

fn get_json<T: for<'de> Deserialize<'de>>(
    http: &dyn HttpClient,
    url: &str,
    user_agent: &str,
    max_body_bytes: usize,
    context: &'static str,
) -> Result<T, RuntimeError> {
    let body = get_response_body(http, url, user_agent, max_body_bytes)?;
    serde_json::from_str(&body).map_err(|_| RuntimeError::JsonDecode {
        context: context.to_owned(),
    })
}

fn get_response_body(
    http: &dyn HttpClient,
    url: &str,
    user_agent: &str,
    max_body_bytes: usize,
) -> Result<String, RuntimeError> {
    let headers = [("User-Agent".to_owned(), user_agent.to_owned())];
    let response = http.get_with_max_body_bytes(url, &headers, max_body_bytes)?;
    if !response.is_success() {
        return Err(RuntimeError::HttpResponse);
    }
    Ok(response.body().to_owned())
}

#[derive(Clone, Copy)]
struct FactSpec {
    kind: &'static str,
    aliases: &'static [&'static str],
    units: UnitKind,
}

#[derive(Clone, Copy)]
enum UnitKind {
    Currency,
    Employees,
}

#[derive(Debug, Deserialize)]
struct CompanyFactsDocument {
    #[serde(default)]
    facts: BTreeMap<String, BTreeMap<String, FactConcept>>,
}

#[derive(Debug, Deserialize)]
struct FactConcept {
    #[serde(default)]
    units: BTreeMap<String, Vec<FactObservation>>,
}

#[derive(Debug, Deserialize)]
struct FactObservation {
    #[serde(default)]
    start: Option<String>,
    #[serde(default)]
    end: Option<String>,
    val: Value,
    #[serde(default)]
    accn: Option<String>,
    #[serde(default)]
    fp: Option<String>,
    #[serde(default)]
    form: Option<String>,
    #[serde(default)]
    filed: Option<String>,
}

struct SelectedObservation<'a> {
    namespace: &'a str,
    concept: &'a str,
    unit: &'a str,
    observation: &'a FactObservation,
}

fn select_annual_observation<'a>(
    document: &'a CompanyFactsDocument,
    spec: &FactSpec,
) -> Option<SelectedObservation<'a>> {
    let mut candidates = Vec::new();
    for (namespace, concepts) in &document.facts {
        for alias in spec.aliases {
            let Some(concept) = concepts.get(*alias) else {
                continue;
            };
            for (unit, observations) in &concept.units {
                if !unit_matches(unit, spec.units) {
                    continue;
                }
                for observation in observations {
                    if !is_annual_observation(observation, spec.units) {
                        continue;
                    }
                    let candidate = SelectedObservation {
                        namespace,
                        concept: alias,
                        unit,
                        observation,
                    };
                    candidates.push(candidate);
                }
            }
        }
    }

    let latest_end = candidates
        .iter()
        .filter_map(|candidate| parse_date(candidate.observation.end.as_deref()))
        .max()?;
    let latest_filed = candidates
        .iter()
        .filter(|candidate| parse_date(candidate.observation.end.as_deref()) == Some(latest_end))
        .filter_map(|candidate| parse_date(candidate.observation.filed.as_deref()))
        .max();
    let latest_candidates: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| {
            parse_date(candidate.observation.end.as_deref()) == Some(latest_end)
                && parse_date(candidate.observation.filed.as_deref()) == latest_filed
        })
        .collect();

    if latest_candidates.len() == 1 {
        latest_candidates.into_iter().next()
    } else {
        None
    }
}

fn unit_matches(unit: &str, kind: UnitKind) -> bool {
    match kind {
        UnitKind::Currency => unit.eq_ignore_ascii_case("USD"),
        UnitKind::Employees => {
            unit.eq_ignore_ascii_case("employees") || unit.eq_ignore_ascii_case("employee")
        }
    }
}

fn is_annual_observation(observation: &FactObservation, kind: UnitKind) -> bool {
    if !matches!(observation.form.as_deref(), Some("10-K") | Some("10-K/A"))
        || observation.fp.as_deref() != Some("FY")
    {
        return false;
    }
    let Some(end) = parse_date(observation.end.as_deref()) else {
        return false;
    };
    if matches!(kind, UnitKind::Employees) {
        return true;
    }
    let Some(start) = parse_date(observation.start.as_deref()) else {
        return false;
    };
    let duration = end.signed_duration_since(start);
    (Duration::days(300)..=Duration::days(400)).contains(&duration)
}

fn observation_description(selected: &SelectedObservation<'_>, value: &str) -> String {
    format!(
        "{}.{} unit={} accession={} filed={} report_date={} raw_value={value}",
        selected.namespace,
        selected.concept,
        selected.unit,
        selected.observation.accn.as_deref().unwrap_or("unknown"),
        selected.observation.filed.as_deref().unwrap_or("unknown"),
        selected.observation.end.as_deref().unwrap_or("unknown"),
    )
}

fn value_as_text(value: &Value) -> Option<String> {
    match value {
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn parse_date(value: Option<&str>) -> Option<NaiveDate> {
    value.and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
}

#[derive(Debug, Deserialize, Default)]
struct SubmissionsDocument {
    #[serde(default)]
    filings: Filings,
}

#[derive(Debug, Deserialize, Default)]
struct Filings {
    #[serde(default)]
    recent: RecentFilings,
}

#[derive(Debug, Deserialize, Default)]
struct RecentFilings {
    #[serde(default, rename = "accessionNumber")]
    accession_numbers: Vec<String>,
    #[serde(default, rename = "filingDate")]
    filing_dates: Vec<String>,
    #[serde(default, rename = "reportDate")]
    report_dates: Vec<String>,
    #[serde(default)]
    form: Vec<String>,
    #[serde(default, rename = "primaryDocument")]
    primary_documents: Vec<String>,
}

#[derive(Clone, Debug)]
struct FilingMetadata {
    accession_number: String,
    filing_date: String,
    report_date_value: Option<String>,
    primary_document: String,
}

impl RecentFilings {
    fn latest_10k(&self) -> Option<FilingMetadata> {
        let mut latest = None;
        for index in 0..self.form.len() {
            let form = self.form.get(index)?;
            if !form.starts_with("10-K") {
                continue;
            }
            let Some(candidate) = self.at(index) else {
                continue;
            };
            if latest
                .as_ref()
                .is_none_or(|current: &FilingMetadata| candidate.filing_date > current.filing_date)
            {
                latest = Some(candidate);
            }
        }
        latest
    }

    fn filing_documents(&self, cik_path: &str) -> Vec<SecDocumentCandidate> {
        let mut candidates = (0..self.form.len())
            .filter_map(|index| {
                let form = self.form.get(index)?;
                if !matches!(
                    form.as_str(),
                    "10-K" | "10-K/A" | "10-Q" | "10-Q/A" | "8-K" | "8-K/A"
                ) {
                    return None;
                }
                self.at(index)?.document_candidate(cik_path, form)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .filing_date
                .cmp(&left.filing_date)
                .then_with(|| left.accession_number.cmp(&right.accession_number))
        });
        candidates.truncate(MAX_SEC_DOCUMENT_CANDIDATES);
        candidates
    }

    fn at(&self, index: usize) -> Option<FilingMetadata> {
        Some(FilingMetadata {
            accession_number: self.accession_numbers.get(index)?.clone(),
            filing_date: self.filing_dates.get(index)?.clone(),
            report_date_value: self.report_dates.get(index).cloned(),
            primary_document: self.primary_documents.get(index)?.clone(),
        })
    }
}

impl FilingMetadata {
    fn report_date(&self) -> Option<NaiveDate> {
        parse_date(self.report_date_value.as_deref())
    }

    fn url(&self, cik_path: &str) -> String {
        format!(
            "{SEC_ARCHIVES_ROOT}/{cik_path}/{}/{}",
            self.accession_number.replace('-', ""),
            self.primary_document
        )
    }

    fn document_candidate(&self, cik_path: &str, form: &str) -> Option<SecDocumentCandidate> {
        let filing_date = parse_date(Some(&self.filing_date))?;
        if !is_valid_accession(&self.accession_number)
            || !is_safe_document_name(&self.primary_document)
        {
            return None;
        }
        Some(SecDocumentCandidate {
            accession_number: self.accession_number.clone(),
            form: form.to_owned(),
            filing_date,
            report_date: self.report_date(),
            primary_document: self.primary_document.clone(),
            source_uri: self.url(cik_path),
            title: self.primary_document.clone(),
            text: String::new(),
            status: SecDocumentStatus::Unknown,
            status_reason: "SEC filing body not yet retrieved".to_owned(),
        })
    }
}

impl SubmissionsDocument {
    fn latest_10k(&self) -> Option<FilingMetadata> {
        self.filings.recent.latest_10k()
    }

    fn filing_documents(&self, cik_path: &str) -> Vec<SecDocumentCandidate> {
        self.filings.recent.filing_documents(cik_path)
    }
}

fn is_valid_accession(value: &str) -> bool {
    value.len() == 20
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 10 || index == 13 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

fn is_safe_document_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}
