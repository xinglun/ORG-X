//! Versioned Weekly Radar company/source registry.

use std::{
    collections::HashSet,
    fs,
    net::{Ipv4Addr, Ipv6Addr},
    path::Path,
};

use serde::{Deserialize, Serialize};
use url::{Host, Url};

use super::error::RuntimeError;

/// The registry schema version implemented by this runtime.
pub const REGISTRY_VERSION: u32 = 1;

/// Versioned collection of companies and explicitly configured source references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanySourceRegistry {
    /// Registry schema version.
    pub version: u32,
    /// Companies included in the configured research universe.
    pub companies: Vec<CompanyConfig>,
}

impl CompanySourceRegistry {
    /// Creates and validates a registry without reading from disk.
    pub fn new(version: u32, companies: Vec<CompanyConfig>) -> Result<Self, RuntimeError> {
        let registry = Self { version, companies };
        registry.validate()?;
        Ok(registry)
    }

    /// Decodes and validates a registry JSON document.
    pub fn from_json(json: &str) -> Result<Self, RuntimeError> {
        let registry: Self = serde_json::from_str(json).map_err(|_| RuntimeError::JsonDecode {
            context: "company source registry".to_owned(),
        })?;
        registry.validate()?;
        Ok(registry)
    }

    /// Reads, decodes, and validates a registry from a local path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        let path = path.as_ref();
        let json = fs::read_to_string(path).map_err(|_| RuntimeError::ConfigurationIo {
            path: path.display().to_string(),
        })?;
        Self::from_json(&json)
    }

    /// Validates the schema version, required identity fields, and uniqueness.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.version != REGISTRY_VERSION {
            return Err(RuntimeError::invalid_configuration(format!(
                "unsupported registry version {}; expected {}",
                self.version, REGISTRY_VERSION
            )));
        }
        if self.companies.is_empty() {
            return Err(RuntimeError::invalid_configuration(
                "registry must contain at least one company",
            ));
        }

        let mut ids = HashSet::with_capacity(self.companies.len());
        let mut tickers = HashSet::with_capacity(self.companies.len());
        for company in &self.companies {
            company.validate()?;
            if !ids.insert(company.id.as_str()) {
                return Err(RuntimeError::invalid_configuration(format!(
                    "duplicate company id {}",
                    company.id
                )));
            }
            if !tickers.insert(company.ticker.as_str()) {
                return Err(RuntimeError::invalid_configuration(format!(
                    "duplicate ticker {}",
                    company.ticker
                )));
            }
        }
        Ok(())
    }

    /// Returns the registry schema version.
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns all companies in configured order.
    pub fn companies(&self) -> &[CompanyConfig] {
        &self.companies
    }

    /// Looks up a company by its stable configured identifier.
    pub fn company(&self, id: &str) -> Option<&CompanyConfig> {
        self.companies.iter().find(|company| company.id == id)
    }
}

/// One company identity and its explicitly configured optional source references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Stable identifier used by normalized facts.
    #[serde(alias = "company_id")]
    pub id: String,
    /// Human-readable company name.
    #[serde(alias = "company_name")]
    pub name: String,
    /// Public ticker symbol.
    pub ticker: String,
    /// Optional SEC Central Index Key.
    #[serde(default, alias = "cik")]
    pub sec_cik: Option<String>,
    /// Optional official investor-relations page.
    #[serde(default, alias = "official_ir_url", alias = "ir_url")]
    pub official_ir: Option<String>,
    /// Optional official careers page.
    #[serde(default, alias = "careers_url")]
    pub careers: Option<String>,
    /// Optional official engineering or AI blog page.
    #[serde(
        default,
        alias = "engineering_blog",
        alias = "engineering_blog_url",
        alias = "engineering_ai_blog_url"
    )]
    pub engineering_ai_blog: Option<String>,
    /// Additional explicitly configured official research entrypoints.
    ///
    /// These are bounded source indexes, not guessed document URLs. They are
    /// retained as a list so a candidate can expose separate organization,
    /// production-system, and diffusion material without replacing its IR or
    /// engineering source.
    #[serde(default, alias = "official_research_urls")]
    pub official_research_sources: Vec<String>,
    /// Optional Greenhouse board identifier.
    #[serde(default, alias = "greenhouse_board_id")]
    pub greenhouse_board: Option<String>,
    /// Optional Lever site identifier.
    #[serde(default, alias = "lever_site_id")]
    pub lever_site: Option<String>,
}

impl CompanyConfig {
    /// Creates and validates a company configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        ticker: impl Into<String>,
        sec_cik: Option<String>,
        official_ir: Option<String>,
        careers: Option<String>,
        engineering_ai_blog: Option<String>,
        greenhouse_board: Option<String>,
        lever_site: Option<String>,
    ) -> Result<Self, RuntimeError> {
        let company = Self {
            id: id.into(),
            name: name.into(),
            ticker: ticker.into(),
            sec_cik,
            official_ir,
            careers,
            engineering_ai_blog,
            official_research_sources: Vec::new(),
            greenhouse_board,
            lever_site,
        };
        company.validate()?;
        Ok(company)
    }

    /// Validates identity fields and all explicitly configured optional sources.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        validate_required("company id", &self.id)?;
        validate_required("company name", &self.name)?;
        validate_required("company ticker", &self.ticker)?;

        if let Some(cik) = &self.sec_cik {
            if cik.len() != 10 || !cik.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(RuntimeError::invalid_configuration(format!(
                    "SEC CIK for {} must contain exactly ten digits",
                    self.id
                )));
            }
        }
        validate_optional_url("official IR URL", &self.official_ir)?;
        validate_optional_url("careers URL", &self.careers)?;
        validate_optional_url("engineering/AI blog URL", &self.engineering_ai_blog)?;
        let mut research_sources = HashSet::new();
        for source in &self.official_research_sources {
            validate_optional_url("official research source URL", &Some(source.clone()))?;
            if !research_sources.insert(source) {
                return Err(RuntimeError::invalid_configuration(
                    "duplicate official research source URL",
                ));
            }
        }
        validate_optional_identifier("Greenhouse board", &self.greenhouse_board)?;
        validate_optional_identifier("Lever site", &self.lever_site)?;
        Ok(())
    }

    /// Returns the stable company identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the configured company name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the configured ticker.
    pub fn ticker(&self) -> &str {
        &self.ticker
    }

    /// Returns the optional SEC CIK.
    pub fn sec_cik(&self) -> Option<&str> {
        self.sec_cik.as_deref()
    }

    /// Returns the optional official investor-relations URL.
    pub fn official_ir_url(&self) -> Option<&str> {
        self.official_ir.as_deref()
    }

    /// Returns the optional official careers URL.
    pub fn careers_url(&self) -> Option<&str> {
        self.careers.as_deref()
    }

    /// Returns the optional official engineering or AI blog URL.
    pub fn engineering_ai_blog_url(&self) -> Option<&str> {
        self.engineering_ai_blog.as_deref()
    }

    /// Adds bounded official research entrypoints and revalidates the config.
    pub fn with_official_research_sources(
        mut self,
        sources: Vec<String>,
    ) -> Result<Self, RuntimeError> {
        self.official_research_sources = sources;
        self.validate()?;
        Ok(self)
    }

    /// Returns the explicitly configured official research entrypoints.
    pub fn official_research_source_urls(&self) -> &[String] {
        &self.official_research_sources
    }

    /// Returns the optional Greenhouse board identifier.
    pub fn greenhouse_board(&self) -> Option<&str> {
        self.greenhouse_board.as_deref()
    }

    /// Returns the optional Lever site identifier.
    pub fn lever_site(&self) -> Option<&str> {
        self.lever_site.as_deref()
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), RuntimeError> {
    if value.trim().is_empty() {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} cannot be blank"
        )));
    }
    Ok(())
}

fn validate_optional_url(field: &'static str, value: &Option<String>) -> Result<(), RuntimeError> {
    let Some(value) = value else {
        return Ok(());
    };
    validate_required(field, value)?;
    let parsed = Url::parse(value).map_err(|_| {
        RuntimeError::invalid_configuration(format!("{field} must be a valid HTTP URL"))
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} must use http:// or https://"
        )));
    }
    if parsed.host().is_none() {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} must include a host"
        )));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} must not include user credentials"
        )));
    }
    if parsed.fragment().is_some() {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} must not include a fragment"
        )));
    }
    if has_non_public_host(&parsed) {
        return Err(RuntimeError::invalid_configuration(format!(
            "{field} must not target a local or non-public host"
        )));
    }
    Ok(())
}

fn has_non_public_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            domain == "localhost"
                || domain.ends_with(".localhost")
                || domain.ends_with(".local")
                || domain.ends_with(".internal")
                || domain.ends_with(".lan")
                || domain.ends_with(".home.arpa")
        }
        Some(Host::Ipv4(address)) => non_public_ipv4(address),
        Some(Host::Ipv6(address)) => non_public_ipv6(address),
        None => true,
    }
}

fn non_public_ipv4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_broadcast()
        || (first == 100 && (64..=127).contains(&second))
        || (first == 192 && second == 0)
        || (first == 198 && (18..=19).contains(&second))
}

fn non_public_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    let first = segments[0];
    let ipv4_mapped = if segments[..6] == [0, 0, 0, 0, 0, 0xffff] {
        Some(Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        ))
    } else {
        None
    };

    address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || (first & 0xffc0) == 0xfe80
        || (first & 0xfe00) == 0xfc00
        || ipv4_mapped.is_some_and(non_public_ipv4)
}

fn validate_optional_identifier(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), RuntimeError> {
    if let Some(value) = value {
        validate_required(field, value)?;
        if !is_safe_source_identifier(value) {
            return Err(RuntimeError::invalid_configuration(format!(
                "{field} must contain only ASCII letters, digits, '-' or '_'"
            )));
        }
    }
    Ok(())
}

pub(crate) fn is_safe_source_identifier(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
