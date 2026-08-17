//! Pure Universe Domain model and deterministic eligibility rules.

use std::collections::BTreeSet;
use std::fmt;

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, UniverseDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(UniverseDomainError::EmptyValue { field });
    }
    Ok(value)
}

macro_rules! identity_type {
    ($name:ident, $field:literal) => {
        #[doc = concat!("Validated identity for a ", $field, ".")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates an identity and rejects blank values.
            pub fn new(value: impl Into<String>) -> Result<Self, UniverseDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the stable identity text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identity_type!(CompanyId, "company id");
identity_type!(SecurityId, "security id");
identity_type!(ListingId, "listing id");
identity_type!(SnapshotId, "snapshot id");

/// Domain validation failures for Universe facts and references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UniverseDomainError {
    /// A required identity or display value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A company reference could not be resolved inside a snapshot.
    UnknownCompany {
        /// Security carrying the invalid reference.
        security_id: SecurityId,
        /// Company identity that was not present.
        company_id: CompanyId,
    },
    /// A listing reference could not be resolved inside a snapshot.
    UnknownSecurityForListing {
        /// Listing carrying the invalid reference.
        listing_id: ListingId,
        /// Security identity that was not present.
        security_id: SecurityId,
    },
    /// An index membership referenced a security absent from a snapshot.
    UnknownSecurityForMembership {
        /// Security identity that was not present.
        security_id: SecurityId,
    },
    /// Two entities of the same kind used one identity in a snapshot.
    DuplicateId {
        /// Entity kind whose identity was duplicated.
        entity: &'static str,
        /// Duplicated identity text.
        id: String,
    },
}

impl fmt::Display for UniverseDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::UnknownCompany {
                security_id,
                company_id,
            } => write!(
                formatter,
                "security {} references unknown company {}",
                security_id.as_str(),
                company_id.as_str()
            ),
            Self::UnknownSecurityForListing {
                listing_id,
                security_id,
            } => write!(
                formatter,
                "listing {} references unknown security {}",
                listing_id.as_str(),
                security_id.as_str()
            ),
            Self::UnknownSecurityForMembership { security_id } => write!(
                formatter,
                "index membership references unknown security {}",
                security_id.as_str()
            ),
            Self::DuplicateId { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for UniverseDomainError {}

/// Company identity and legal display name supplied to the Universe context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Company {
    id: CompanyId,
    legal_name: String,
}

impl Company {
    /// Creates a company with a non-blank legal name.
    pub fn new(id: CompanyId, legal_name: impl Into<String>) -> Result<Self, UniverseDomainError> {
        Ok(Self {
            id,
            legal_name: non_empty("company legal name", legal_name)?,
        })
    }

    /// Returns the company identity.
    pub fn id(&self) -> &CompanyId {
        &self.id
    }

    /// Returns the supplied legal name.
    pub fn legal_name(&self) -> &str {
        &self.legal_name
    }
}

/// Instrument classifications needed by the MVP eligibility rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstrumentType {
    /// Common equity share.
    CommonEquity,
    /// Preferred equity, which the MVP excludes.
    PreferredEquity,
    /// A supplied classification not interpreted by this policy.
    Other(String),
}

/// Security identity, issuer relationship, and instrument classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Security {
    id: SecurityId,
    company_id: CompanyId,
    instrument_type: InstrumentType,
}

impl Security {
    /// Creates a security fact for a company identity.
    pub fn new(
        id: SecurityId,
        company_id: CompanyId,
        instrument_type: InstrumentType,
    ) -> Result<Self, UniverseDomainError> {
        Ok(Self {
            id,
            company_id,
            instrument_type,
        })
    }

    /// Returns the security identity.
    pub fn id(&self) -> &SecurityId {
        &self.id
    }

    /// Returns the referenced company identity.
    pub fn company_id(&self) -> &CompanyId {
        &self.company_id
    }

    /// Returns the supplied instrument classification.
    pub fn instrument_type(&self) -> &InstrumentType {
        &self.instrument_type
    }
}

/// Exchange facts understood by the MVP policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Exchange {
    /// New York Stock Exchange.
    Nyse,
    /// Nasdaq exchange.
    Nasdaq,
    /// Any supplied exchange outside the MVP allow-list.
    Other(String),
}

/// A security's tradable listing fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Listing {
    id: ListingId,
    security_id: SecurityId,
    exchange: Exchange,
    ticker: String,
    active: bool,
}

impl Listing {
    /// Creates a listing with a non-blank ticker.
    pub fn new(
        id: ListingId,
        security_id: SecurityId,
        exchange: Exchange,
        ticker: impl Into<String>,
        active: bool,
    ) -> Result<Self, UniverseDomainError> {
        Ok(Self {
            id,
            security_id,
            exchange,
            ticker: non_empty("listing ticker", ticker)?,
            active,
        })
    }

    /// Returns the listing identity.
    pub fn id(&self) -> &ListingId {
        &self.id
    }

    /// Returns the referenced security identity.
    pub fn security_id(&self) -> &SecurityId {
        &self.security_id
    }

    /// Returns the exchange fact.
    pub fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    /// Returns the ticker supplied by the outer fact source.
    pub fn ticker(&self) -> &str {
        &self.ticker
    }

    /// Reports whether the listing is active in the supplied snapshot.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Index membership facts accepted by the MVP universe policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UniverseIndex {
    /// S&P 500 membership.
    Sp500,
    /// Nasdaq 100 membership.
    Nasdaq100,
    /// A supplied index not interpreted by this policy.
    Other(String),
}

/// A security's membership in one supplied index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexMembership {
    security_id: SecurityId,
    index: UniverseIndex,
}

impl IndexMembership {
    /// Creates an index membership fact.
    pub fn new(security_id: SecurityId, index: UniverseIndex) -> Self {
        Self { security_id, index }
    }

    /// Returns the referenced security identity.
    pub fn security_id(&self) -> &SecurityId {
        &self.security_id
    }

    /// Returns the supplied index.
    pub fn index(&self) -> &UniverseIndex {
        &self.index
    }
}

/// Facts passed to a policy without any external acquisition or inference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityFacts {
    instrument_type: InstrumentType,
    exchange: Exchange,
    active_listing: bool,
    supported_index_membership: bool,
}

impl EligibilityFacts {
    /// Creates facts explicitly supplied by an outer context.
    pub fn new(
        instrument_type: InstrumentType,
        exchange: Exchange,
        active_listing: bool,
        supported_index_membership: bool,
    ) -> Self {
        Self {
            instrument_type,
            exchange,
            active_listing,
            supported_index_membership,
        }
    }
}

/// Deterministic MVP policy for the initial ORG-X observation universe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityPolicy;

impl EligibilityPolicy {
    /// Returns the policy for active common US-listed equity in S&P 500 or
    /// Nasdaq 100 membership.
    pub fn mvp() -> Self {
        Self
    }

    /// Evaluates supplied facts without fetching or inferring anything.
    pub fn is_eligible(&self, facts: &EligibilityFacts) -> bool {
        matches!(facts.instrument_type, InstrumentType::CommonEquity)
            && matches!(facts.exchange, Exchange::Nyse | Exchange::Nasdaq)
            && facts.active_listing
            && facts.supported_index_membership
    }

    fn supports_index(&self, index: &UniverseIndex) -> bool {
        matches!(index, UniverseIndex::Sp500 | UniverseIndex::Nasdaq100)
    }
}

/// Validated aggregate of supplied Universe facts for one opaque snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniverseSnapshot {
    id: SnapshotId,
    companies: Vec<Company>,
    securities: Vec<Security>,
    listings: Vec<Listing>,
    memberships: Vec<IndexMembership>,
}

impl UniverseSnapshot {
    /// Validates entity identities and all company/security references.
    pub fn new(
        id: SnapshotId,
        companies: Vec<Company>,
        securities: Vec<Security>,
        listings: Vec<Listing>,
        memberships: Vec<IndexMembership>,
    ) -> Result<Self, UniverseDomainError> {
        ensure_unique_companies(&companies)?;
        ensure_unique_securities(&securities)?;
        ensure_unique_listings(&listings)?;

        let company_ids: BTreeSet<_> = companies.iter().map(|company| &company.id).collect();
        for security in &securities {
            if !company_ids.contains(&security.company_id) {
                return Err(UniverseDomainError::UnknownCompany {
                    security_id: security.id.clone(),
                    company_id: security.company_id.clone(),
                });
            }
        }

        let security_ids: BTreeSet<_> = securities.iter().map(|security| &security.id).collect();
        for listing in &listings {
            if !security_ids.contains(&listing.security_id) {
                return Err(UniverseDomainError::UnknownSecurityForListing {
                    listing_id: listing.id.clone(),
                    security_id: listing.security_id.clone(),
                });
            }
        }
        for membership in &memberships {
            if !security_ids.contains(&membership.security_id) {
                return Err(UniverseDomainError::UnknownSecurityForMembership {
                    security_id: membership.security_id.clone(),
                });
            }
        }

        Ok(Self {
            id,
            companies,
            securities,
            listings,
            memberships,
        })
    }

    /// Returns the opaque snapshot identity.
    pub fn id(&self) -> &SnapshotId {
        &self.id
    }

    /// Returns eligible security identities once, in stable identity order.
    pub fn eligible_security_ids(&self, policy: &EligibilityPolicy) -> Vec<SecurityId> {
        let mut eligible = BTreeSet::new();

        for security in &self.securities {
            let supported_index_membership = self.memberships.iter().any(|membership| {
                membership.security_id == security.id && policy.supports_index(&membership.index)
            });

            for listing in self
                .listings
                .iter()
                .filter(|listing| listing.security_id == security.id)
            {
                let facts = EligibilityFacts::new(
                    security.instrument_type.clone(),
                    listing.exchange.clone(),
                    listing.active,
                    supported_index_membership,
                );
                if policy.is_eligible(&facts) {
                    eligible.insert(security.id.clone());
                    break;
                }
            }
        }

        eligible.into_iter().collect()
    }
}

fn ensure_unique_companies(companies: &[Company]) -> Result<(), UniverseDomainError> {
    let mut ids = BTreeSet::new();
    for company in companies {
        if !ids.insert(&company.id) {
            return Err(UniverseDomainError::DuplicateId {
                entity: "company",
                id: company.id.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_securities(securities: &[Security]) -> Result<(), UniverseDomainError> {
    let mut ids = BTreeSet::new();
    for security in securities {
        if !ids.insert(&security.id) {
            return Err(UniverseDomainError::DuplicateId {
                entity: "security",
                id: security.id.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_listings(listings: &[Listing]) -> Result<(), UniverseDomainError> {
    let mut ids = BTreeSet::new();
    for listing in listings {
        if !ids.insert(&listing.id) {
            return Err(UniverseDomainError::DuplicateId {
                entity: "listing",
                id: listing.id.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod mod_test;
