use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Supported CA data source types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    #[serde(rename = "bigquery")]
    BigQuery,
    #[serde(rename = "looker")]
    Looker,
    #[serde(rename = "looker_studio")]
    LookerStudio,
    #[serde(rename = "alloy_db")]
    AlloyDb,
    #[serde(rename = "spanner")]
    Spanner,
    #[serde(rename = "cloud_sql")]
    CloudSql,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::BigQuery => write!(f, "bigquery"),
            SourceType::Looker => write!(f, "looker"),
            SourceType::LookerStudio => write!(f, "looker_studio"),
            SourceType::AlloyDb => write!(f, "alloydb"),
            SourceType::Spanner => write!(f, "spanner"),
            SourceType::CloudSql => write!(f, "cloud_sql"),
        }
    }
}

/// Which CA API family a source type belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileFamily {
    /// BigQuery, Looker, Looker Studio → Chat / DataAgent API
    ChatDataAgent,
    /// AlloyDB, Spanner, Cloud SQL → QueryData API
    QueryData,
}

impl SourceType {
    /// Returns the API family for this source type.
    pub fn family(&self) -> ProfileFamily {
        match self {
            SourceType::BigQuery | SourceType::Looker | SourceType::LookerStudio => {
                ProfileFamily::ChatDataAgent
            }
            SourceType::AlloyDb | SourceType::Spanner | SourceType::CloudSql => {
                ProfileFamily::QueryData
            }
        }
    }

    /// Whether this source type supports DataAgent creation.
    pub fn supports_create_agent(&self) -> bool {
        self.family() == ProfileFamily::ChatDataAgent
    }
}

/// A CA source profile. Covers all six source types via optional
/// source-specific fields. Validation at load time ensures the right
/// fields are present for the declared source_type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaProfile {
    pub name: String,
    pub source_type: SourceType,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    // ── BigQuery (Chat/DataAgent) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<Vec<String>>,

    // ── Looker (Chat/DataAgent) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub looker_instance_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub looker_explores: Option<Vec<String>>,
    /// Looker OAuth client ID (for inline credentials).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub looker_client_id: Option<String>,
    /// Looker OAuth client secret (for inline credentials).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub looker_client_secret: Option<String>,

    // ── Looker Studio (Chat/DataAgent) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studio_datasource_id: Option<String>,

    // ── Database sources: AlloyDB, Spanner, Cloud SQL (QueryData) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_set_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_ref: Option<String>,

    // ── Cloud SQL-specific ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_name: Option<String>,
}

impl CaProfile {
    /// Validate that source-specific required fields are present.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            bail!("Profile name cannot be empty");
        }
        if self.project.is_empty() {
            bail!("Profile project cannot be empty");
        }

        match self.source_type {
            SourceType::BigQuery => {
                // BigQuery needs either agent or tables (or neither for bare ask)
            }
            SourceType::Looker => {
                if self
                    .looker_instance_url
                    .as_ref()
                    .map_or(true, |u| u.is_empty())
                {
                    bail!(
                        "Profile '{}': looker_instance_url is required for Looker sources",
                        self.name
                    );
                }
                if self.looker_explores.as_ref().map_or(true, |e| e.is_empty()) {
                    bail!(
                        "Profile '{}': at least one looker_explores entry is required for Looker sources",
                        self.name
                    );
                }
                let explores = self.looker_explores.as_ref().unwrap();
                if explores.len() > 5 {
                    bail!(
                        "Profile '{}': Looker supports at most 5 explores, got {}",
                        self.name,
                        explores.len()
                    );
                }
                for explore in explores {
                    if parse_looker_explore(explore).is_err() {
                        bail!(
                            "Profile '{}': invalid explore format '{}'. \
                             Expected 'model/explore' (e.g. 'sales_model/orders')",
                            self.name,
                            explore
                        );
                    }
                }
                // OAuth credentials must be both present or both absent.
                let has_id = self.looker_client_id.is_some();
                let has_secret = self.looker_client_secret.is_some();
                if has_id != has_secret {
                    bail!(
                        "Profile '{}': looker_client_id and looker_client_secret must be provided together",
                        self.name
                    );
                }
            }
            SourceType::LookerStudio => {
                if self
                    .studio_datasource_id
                    .as_ref()
                    .map_or(true, |id| id.is_empty())
                {
                    bail!(
                        "Profile '{}': studio_datasource_id is required for Looker Studio sources",
                        self.name
                    );
                }
            }
            SourceType::AlloyDb | SourceType::Spanner => {
                if self.context_set_id.is_none() {
                    bail!(
                        "Profile '{}': context_set_id is required for {} sources",
                        self.name,
                        self.source_type
                    );
                }
                if self.datasource_ref.is_none() {
                    bail!(
                        "Profile '{}': datasource_ref is required for {} sources",
                        self.name,
                        self.source_type
                    );
                }
            }
            SourceType::CloudSql => {
                if self.context_set_id.is_none() {
                    bail!(
                        "Profile '{}': context_set_id is required for Cloud SQL sources",
                        self.name
                    );
                }
                if self.datasource_ref.is_none() {
                    bail!(
                        "Profile '{}': datasource_ref is required for Cloud SQL sources",
                        self.name
                    );
                }
                if self.connection_name.is_none() {
                    bail!(
                        "Profile '{}': connection_name is required for Cloud SQL sources",
                        self.name
                    );
                }
            }
        }

        Ok(())
    }
}

/// Parse a Looker explore reference like "model/explore" into (model, explore).
///
/// Exactly one `/` is required — values like `model/explore/extra` are rejected.
pub fn parse_looker_explore(s: &str) -> Result<(&str, &str)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!("Invalid Looker explore: '{s}'. Expected format: model/explore");
    }
    Ok((parts[0], parts[1]))
}

/// Load a profile from a YAML file.
pub fn load_profile(path: &Path) -> Result<CaProfile> {
    let contents = std::fs::read_to_string(path)
        .context(format!("Failed to read profile: {}", path.display()))?;
    let profile: CaProfile = serde_yaml::from_str(&contents)
        .context(format!("Failed to parse profile: {}", path.display()))?;
    profile.validate()?;
    Ok(profile)
}

/// Load all profiles from a directory. Non-YAML files are skipped.
pub fn load_profiles_from_dir(dir: &Path) -> Result<Vec<CaProfile>> {
    let mut profiles = Vec::new();
    if !dir.exists() {
        return Ok(profiles);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            profiles.push(load_profile(&path)?);
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bigquery_profile() -> CaProfile {
        CaProfile {
            name: "test-bq".into(),
            source_type: SourceType::BigQuery,
            project: "my-project".into(),
            location: Some("US".into()),
            agent: Some("agent-analytics".into()),
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        }
    }

    #[test]
    fn bigquery_profile_validates() {
        bigquery_profile().validate().unwrap();
    }

    #[test]
    fn bigquery_family_is_chat() {
        assert_eq!(SourceType::BigQuery.family(), ProfileFamily::ChatDataAgent);
    }

    #[test]
    fn looker_family_is_chat() {
        assert_eq!(SourceType::Looker.family(), ProfileFamily::ChatDataAgent);
    }

    #[test]
    fn alloydb_family_is_query_data() {
        assert_eq!(SourceType::AlloyDb.family(), ProfileFamily::QueryData);
    }

    #[test]
    fn spanner_family_is_query_data() {
        assert_eq!(SourceType::Spanner.family(), ProfileFamily::QueryData);
    }

    #[test]
    fn cloud_sql_family_is_query_data() {
        assert_eq!(SourceType::CloudSql.family(), ProfileFamily::QueryData);
    }

    #[test]
    fn looker_studio_family_is_chat() {
        assert_eq!(
            SourceType::LookerStudio.family(),
            ProfileFamily::ChatDataAgent
        );
    }

    #[test]
    fn empty_name_fails() {
        let mut p = bigquery_profile();
        p.name = "".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn empty_project_fails() {
        let mut p = bigquery_profile();
        p.project = "".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn looker_missing_url_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("looker_instance_url"));
    }

    #[test]
    fn looker_missing_explores_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("looker_explores"));
    }

    #[test]
    fn looker_too_many_explores_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec![
                "a/a".into(),
                "b/b".into(),
                "c/c".into(),
                "d/d".into(),
                "e/e".into(),
                "f/f".into(),
            ]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("at most 5"));
    }

    #[test]
    fn looker_studio_missing_datasource_fails() {
        let p = CaProfile {
            name: "bad-studio".into(),
            source_type: SourceType::LookerStudio,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("studio_datasource_id"));
    }

    #[test]
    fn alloydb_missing_context_set_fails() {
        let p = CaProfile {
            name: "bad-alloy".into(),
            source_type: SourceType::AlloyDb,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: Some("ref".into()),
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("context_set_id"));
    }

    #[test]
    fn alloydb_missing_datasource_ref_fails() {
        let p = CaProfile {
            name: "bad-alloy".into(),
            source_type: SourceType::AlloyDb,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: Some("ctx-123".into()),
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("datasource_ref"));
    }

    #[test]
    fn cloud_sql_missing_connection_name_fails() {
        let p = CaProfile {
            name: "bad-cloudsql".into(),
            source_type: SourceType::CloudSql,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: Some("ctx-123".into()),
            datasource_ref: Some("ref".into()),
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("connection_name"));
    }

    #[test]
    fn spanner_valid_profile() {
        let p = CaProfile {
            name: "finance-spanner".into(),
            source_type: SourceType::Spanner,
            project: "my-project".into(),
            location: Some("us-central1".into()),
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: Some("ctx-finance".into()),
            datasource_ref: Some("projects/my-project/instances/fin/databases/ledger".into()),
            db_type: None,
            connection_name: None,
        };
        p.validate().unwrap();
    }

    #[test]
    fn create_agent_support() {
        assert!(SourceType::BigQuery.supports_create_agent());
        assert!(SourceType::Looker.supports_create_agent());
        assert!(SourceType::LookerStudio.supports_create_agent());
        assert!(!SourceType::AlloyDb.supports_create_agent());
        assert!(!SourceType::Spanner.supports_create_agent());
        assert!(!SourceType::CloudSql.supports_create_agent());
    }

    #[test]
    fn roundtrip_yaml_bigquery() {
        let p = bigquery_profile();
        let yaml = serde_yaml::to_string(&p).unwrap();
        let parsed: CaProfile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "test-bq");
        assert_eq!(parsed.source_type, SourceType::BigQuery);
        assert_eq!(parsed.agent.as_deref(), Some("agent-analytics"));
    }

    #[test]
    fn roundtrip_yaml_looker() {
        let p = CaProfile {
            name: "sales-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let yaml = serde_yaml::to_string(&p).unwrap();
        let parsed: CaProfile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.source_type, SourceType::Looker);
        assert_eq!(
            parsed.looker_instance_url.as_deref(),
            Some("https://looker.example.com")
        );
        parsed.validate().unwrap();
    }

    #[test]
    fn source_type_display() {
        assert_eq!(SourceType::BigQuery.to_string(), "bigquery");
        assert_eq!(SourceType::Looker.to_string(), "looker");
        assert_eq!(SourceType::LookerStudio.to_string(), "looker_studio");
        assert_eq!(SourceType::AlloyDb.to_string(), "alloydb");
        assert_eq!(SourceType::Spanner.to_string(), "spanner");
        assert_eq!(SourceType::CloudSql.to_string(), "cloud_sql");
    }

    #[test]
    fn parse_looker_explore_valid() {
        let (model, explore) = parse_looker_explore("sales_model/orders").unwrap();
        assert_eq!(model, "sales_model");
        assert_eq!(explore, "orders");
    }

    #[test]
    fn parse_looker_explore_rejects_extra_slashes() {
        assert!(parse_looker_explore("model/explore/sub").is_err());
    }

    #[test]
    fn parse_looker_explore_invalid_no_slash() {
        assert!(parse_looker_explore("just_explore").is_err());
    }

    #[test]
    fn parse_looker_explore_invalid_empty_parts() {
        assert!(parse_looker_explore("/explore").is_err());
        assert!(parse_looker_explore("model/").is_err());
    }

    #[test]
    fn looker_partial_oauth_id_only_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: Some("my-client-id".into()),
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err
            .to_string()
            .contains("looker_client_id and looker_client_secret must be provided together"));
    }

    #[test]
    fn looker_partial_oauth_secret_only_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: None,
            looker_client_secret: Some("my-secret".into()),
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err
            .to_string()
            .contains("looker_client_id and looker_client_secret must be provided together"));
    }

    #[test]
    fn looker_empty_instance_url_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("".into()),
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("looker_instance_url"));
    }

    #[test]
    fn studio_empty_datasource_id_fails() {
        let p = CaProfile {
            name: "bad-studio".into(),
            source_type: SourceType::LookerStudio,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: Some("".into()),
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("studio_datasource_id"));
    }

    #[test]
    fn looker_invalid_explore_format_fails() {
        let p = CaProfile {
            name: "bad-looker".into(),
            source_type: SourceType::Looker,
            project: "my-project".into(),
            location: None,
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec!["no_slash".into()]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("invalid explore format"));
    }
}
