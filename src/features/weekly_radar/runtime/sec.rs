//! SEC EDGAR/Company Facts acquisition with provider-private payload models.

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::config::CompanyConfig;
use super::error::RuntimeError;
use super::http::{HttpClient, MAX_HTTP_RESPONSE_BODY_BYTES};
use super::model::{Confidence, FactStatus, NormalizedFact, Provenance};
use super::rules::extract_employee_candidate;

const SEC_SUBMISSIONS_ROOT: &str = "https://data.sec.gov/submissions";
const SEC_FACTS_ROOT: &str = "https://data.sec.gov/api/xbrl/companyfacts";
const SEC_ARCHIVES_ROOT: &str = "https://www.sec.gov/Archives/edgar/data";

/// Finite response limit for SEC JSON payloads parsed by the runtime.
///
/// Company Facts and submissions contain complete registrant histories and
/// are materially larger than the default limit used for ordinary public
/// pages. The limit remains bounded so an unexpectedly large response still
/// fails closed before it can cause an unbounded allocation.
pub const SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES: usize = 16 * 1024 * 1024;

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
}

impl CompanyEvidence {
    fn new(company_id: impl Into<String>, facts: Vec<NormalizedFact>) -> Self {
        Self {
            company_id: company_id.into(),
            facts,
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
        let submissions: SubmissionsDocument = get_json(
            http,
            &submissions_url,
            user_agent,
            SEC_SUBMISSIONS_MAX_RESPONSE_BODY_BYTES,
            "SEC submissions",
        )?;
        let facts: CompanyFactsDocument = get_json(
            http,
            &facts_url,
            user_agent,
            SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES,
            "SEC Company Facts",
        )?;
        let latest_filing = submissions.latest_10k();
        let retrieved_at = Utc::now();
        let mut normalized = Vec::with_capacity(FACT_SPECS.len());

        for spec in FACT_SPECS {
            if let Some(selected) = select_annual_observation(&facts, spec) {
                let value = value_as_text(&selected.observation.val).ok_or_else(|| {
                    RuntimeError::invalid_model(format!(
                        "SEC annual {} value is not scalar",
                        spec.kind
                    ))
                })?;
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

            if spec.kind == "employee_count" {
                if let Some(filing) = latest_filing.as_ref() {
                    let filing_url = filing.url(cik_path);
                    let body = get_response_body(
                        http,
                        &filing_url,
                        user_agent,
                        MAX_HTTP_RESPONSE_BODY_BYTES,
                    )?;
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

            normalized.push(unknown_fact(
                company,
                spec.kind,
                facts_url.clone(),
                "SEC Company Facts: no unambiguous annual value",
                latest_filing.as_ref().and_then(FilingMetadata::report_date),
                retrieved_at,
            )?);
        }

        Ok(CompanyEvidence::new(company.id(), normalized))
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
    let provenance = Provenance::new(
        source_uri,
        source_field_or_passage,
        retrieved_at,
        effective_date,
    )?;
    NormalizedFact::without_value(
        company.id(),
        kind,
        FactStatus::Unknown,
        Confidence::Unknown,
        provenance,
    )
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
            let candidate = FilingMetadata {
                accession_number: self.accession_numbers.get(index)?.clone(),
                filing_date: self.filing_dates.get(index)?.clone(),
                report_date_value: self.report_dates.get(index).cloned(),
                primary_document: self.primary_documents.get(index)?.clone(),
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
}

impl SubmissionsDocument {
    fn latest_10k(&self) -> Option<FilingMetadata> {
        self.filings.recent.latest_10k()
    }
}
