use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use clap::{Arg, ArgMatches, Command};
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::ca::client::CaClient;
use crate::ca::models::CaQuestionResponse;
use crate::ca::profiles::{self, CaProfile, SourceType};
use crate::cli::OutputFormat;
use crate::commands;
use crate::output;

#[derive(Debug, Clone, Serialize)]
pub struct SchemaRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_schema: Option<String>,
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nullable: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaDescribeResult {
    pub profile_name: String,
    pub source_type: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    pub database_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub rows: Vec<SchemaRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseRow {
    pub database_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseListResult {
    pub profile_name: String,
    pub source_type: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub rows: Vec<DatabaseRow>,
}

#[async_trait]
pub trait QueryDataExecutor {
    async fn ask_querydata(&self, profile: &CaProfile, prompt: &str) -> Result<CaQuestionResponse>;
}

#[async_trait]
impl QueryDataExecutor for CaClient {
    async fn ask_querydata(&self, profile: &CaProfile, prompt: &str) -> Result<CaQuestionResponse> {
        CaClient::ask_querydata(self, profile, prompt).await
    }
}

pub fn augment_namespace_command(namespace: &str, ns_cmd: Command) -> Command {
    match namespace {
        "spanner" | "cloudsql" => ns_cmd.subcommand(schema_command()),
        "alloydb" => ns_cmd
            .subcommand(schema_command())
            .subcommand(alloydb_databases_command()),
        "looker" => ns_cmd
            .subcommand(looker_explores_command())
            .subcommand(looker_dashboards_command()),
        _ => ns_cmd,
    }
}

fn looker_explores_command() -> Command {
    Command::new("explores")
        .about("Looker explore operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List all explores from the Looker instance")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Looker profile name or path"),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get detailed metadata for a single explore")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Looker profile name or path"),
                )
                .arg(
                    Arg::new("explore")
                        .long("explore")
                        .required(true)
                        .help("Explore reference (model/explore, e.g. sales_model/orders)"),
                ),
        )
}

fn looker_dashboards_command() -> Command {
    Command::new("dashboards")
        .about("Looker dashboard operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List all dashboards from the Looker instance")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Looker profile name or path"),
                ),
        )
        .subcommand(
            Command::new("get")
                .about("Get detailed metadata for a single dashboard")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Looker profile name or path"),
                )
                .arg(
                    Arg::new("dashboard-id")
                        .long("dashboard-id")
                        .required(true)
                        .help("Dashboard ID"),
                ),
        )
}

pub async fn try_run_namespace_helper(
    namespace: &str,
    group_name: &str,
    action_name: &str,
    action_matches: &ArgMatches,
    root_matches: &ArgMatches,
) -> Option<Result<()>> {
    let profile_ref = action_matches
        .try_get_one::<String>("profile")
        .ok()
        .flatten()
        .map(|s| s.as_str())?;

    let format = root_matches
        .get_one::<OutputFormat>("format")
        .cloned()
        .unwrap_or(OutputFormat::Json);
    let sanitize_template = root_matches
        .get_one::<String>("sanitize")
        .map(|s| s.as_str());
    let auth_opts = AuthOptions {
        token: root_matches.get_one::<String>("token").cloned(),
        credentials_file: root_matches.get_one::<String>("credentials_file").cloned(),
    };

    match (namespace, group_name, action_name) {
        ("spanner", "schema", "describe") => Some(
            run_schema_describe(
                profile_ref,
                SourceType::Spanner,
                &auth_opts,
                &format,
                sanitize_template,
            )
            .await,
        ),
        ("cloudsql", "schema", "describe") => Some(
            run_schema_describe(
                profile_ref,
                SourceType::CloudSql,
                &auth_opts,
                &format,
                sanitize_template,
            )
            .await,
        ),
        ("alloydb", "schema", "describe") => Some(
            run_schema_describe(
                profile_ref,
                SourceType::AlloyDb,
                &auth_opts,
                &format,
                sanitize_template,
            )
            .await,
        ),
        ("alloydb", "databases", "list") => Some(
            run_alloydb_databases_list(profile_ref, &auth_opts, &format, sanitize_template).await,
        ),
        ("looker", "explores", "list") => Some(
            commands::looker::explores::run_list(
                profile_ref,
                &auth_opts,
                &format,
                sanitize_template,
            )
            .await,
        ),
        ("looker", "explores", "get") => {
            let explore = action_matches
                .get_one::<String>("explore")
                .map(|s| s.as_str())
                .unwrap_or("");
            Some(
                commands::looker::explores::run_get(
                    profile_ref,
                    explore,
                    &auth_opts,
                    &format,
                    sanitize_template,
                )
                .await,
            )
        }
        ("looker", "dashboards", "list") => Some(
            commands::looker::dashboards::run_list(
                profile_ref,
                &auth_opts,
                &format,
                sanitize_template,
            )
            .await,
        ),
        ("looker", "dashboards", "get") => {
            let dashboard_id = action_matches
                .get_one::<String>("dashboard-id")
                .map(|s| s.as_str())
                .unwrap_or("");
            Some(
                commands::looker::dashboards::run_get(
                    profile_ref,
                    dashboard_id,
                    &auth_opts,
                    &format,
                    sanitize_template,
                )
                .await,
            )
        }
        _ => None,
    }
}

pub fn schema_command() -> Command {
    Command::new("schema")
        .about("Profile-aware schema helpers")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("describe")
                .about("Describe database schema from a source profile")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Profile name or path to profile YAML file"),
                ),
        )
}

pub fn alloydb_databases_command() -> Command {
    Command::new("databases")
        .about("Profile-aware AlloyDB database helpers")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List databases visible to the AlloyDB profile")
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .required(true)
                        .help("Profile name or path to profile YAML file"),
                ),
        )
}

pub async fn run_schema_describe(
    profile_ref: &str,
    expected_source: SourceType,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let profile = resolve_profile_for_source(profile_ref, std::slice::from_ref(&expected_source))?;
    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let result = run_schema_describe_with_executor(&client, &profile).await?;
    render_with_optional_sanitization(
        &result,
        auth_opts,
        format,
        sanitize_template,
        render_schema_text,
    )
    .await
}

pub async fn run_alloydb_databases_list(
    profile_ref: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let profile = resolve_profile_for_source(profile_ref, &[SourceType::AlloyDb])?;
    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let result = run_alloydb_databases_list_with_executor(&client, &profile).await?;
    render_with_optional_sanitization(
        &result,
        auth_opts,
        format,
        sanitize_template,
        render_database_list_text,
    )
    .await
}

pub async fn run_schema_describe_with_executor(
    executor: &dyn QueryDataExecutor,
    profile: &CaProfile,
) -> Result<SchemaDescribeResult> {
    match profile.source_type {
        SourceType::Spanner | SourceType::CloudSql | SourceType::AlloyDb => {}
        _ => bail!(
            "Profile '{}' is source_type '{}', expected 'spanner', 'cloud_sql', or 'alloydb'",
            profile.name,
            profile.source_type
        ),
    }

    let prompt = schema_prompt(profile);
    let response = executor.ask_querydata(profile, &prompt).await?;

    Ok(SchemaDescribeResult {
        profile_name: profile.name.clone(),
        source_type: profile.source_type.to_string(),
        project: profile.project.clone(),
        location: profile.location.clone(),
        database_id: profile.database_id.clone().unwrap_or_default(),
        sql: response.sql,
        explanation: response.explanation,
        rows: extract_schema_rows(&response.results)?,
    })
}

pub async fn run_alloydb_databases_list_with_executor(
    executor: &dyn QueryDataExecutor,
    profile: &CaProfile,
) -> Result<DatabaseListResult> {
    if profile.source_type != SourceType::AlloyDb {
        bail!(
            "Profile '{}' is source_type '{}', expected 'alloydb'",
            profile.name,
            profile.source_type
        );
    }

    let response = executor
        .ask_querydata(profile, alloydb_database_list_prompt())
        .await?;

    Ok(DatabaseListResult {
        profile_name: profile.name.clone(),
        source_type: profile.source_type.to_string(),
        project: profile.project.clone(),
        location: profile.location.clone(),
        cluster_id: profile.cluster_id.clone(),
        instance_id: profile.instance_id.clone(),
        sql: response.sql,
        explanation: response.explanation,
        rows: extract_database_rows(&response.results)?,
    })
}

fn resolve_profile_for_source(
    profile_ref: &str,
    expected_sources: &[SourceType],
) -> Result<CaProfile> {
    let profile = profiles::resolve_profile(profile_ref)?;
    if expected_sources.contains(&profile.source_type) {
        return Ok(profile);
    }

    let expected = expected_sources
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("' or '");
    bail!(
        "Profile '{}' is source_type '{}', expected '{}'",
        profile.name,
        profile.source_type,
        expected
    )
}

fn schema_prompt(profile: &CaProfile) -> String {
    match profile.source_type {
        SourceType::Spanner => "Describe the schema of this Spanner database. Return exactly these columns: table_name, column_name, data_type, is_nullable. Return one row per column, exclude internal tables, and order by table_name then column_name. Do not summarize.".to_string(),
        SourceType::AlloyDb => "Describe the schema of this AlloyDB PostgreSQL database. Return exactly these columns: table_schema, table_name, column_name, data_type, is_nullable. Return one row per column, exclude system schemas (pg_catalog, information_schema, pg_toast), and order by table_schema, table_name, then column_name. Do not summarize.".to_string(),
        SourceType::CloudSql => {
            let engine_hint = match profile.db_type.as_deref() {
                Some("mysql") => "MySQL",
                _ => "PostgreSQL",
            };
            format!(
                "Describe the schema of this {engine_hint} database. Return exactly these columns: table_schema, table_name, column_name, data_type, is_nullable. Return one row per column, exclude system schemas, and order by table_schema, table_name, then column_name. Do not summarize."
            )
        }
        _ => "Describe the database schema. Return one row per column.".to_string(),
    }
}

fn alloydb_database_list_prompt() -> &'static str {
    "List all non-template databases in this AlloyDB PostgreSQL instance. Return exactly one column named database_name. Order by database_name. Do not summarize."
}

fn extract_schema_rows(
    rows: &[serde_json::Map<String, serde_json::Value>],
) -> Result<Vec<SchemaRow>> {
    rows.iter()
        .map(|row| {
            let table_name = required_string(row, &["table_name", "table"])?;
            let column_name = required_string(row, &["column_name", "column"])?;
            let data_type = required_string(row, &["data_type", "type"])?;
            Ok(SchemaRow {
                table_schema: optional_string(row, &["table_schema", "schema_name", "schema"]),
                table_name,
                column_name,
                data_type,
                is_nullable: optional_string(row, &["is_nullable", "nullable"]),
            })
        })
        .collect()
}

fn extract_database_rows(
    rows: &[serde_json::Map<String, serde_json::Value>],
) -> Result<Vec<DatabaseRow>> {
    rows.iter()
        .map(|row| {
            if let Some(name) =
                optional_string(row, &["database_name", "database", "datname", "name"])
            {
                return Ok(DatabaseRow {
                    database_name: name,
                });
            }

            if let Some(value) = row.values().find_map(value_as_string) {
                return Ok(DatabaseRow {
                    database_name: value,
                });
            }

            Err(anyhow!("Database list row is missing a database name"))
        })
        .collect()
}

fn required_string(
    row: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Result<String> {
    optional_string(row, keys)
        .ok_or_else(|| anyhow!("Schema row is missing required field '{}'", keys[0]))
}

fn optional_string(
    row: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    for key in keys {
        if let Some(value) = row.get(*key).and_then(value_as_string) {
            return Some(value);
        }
    }
    None
}

fn value_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => Some(value.to_string()),
    }
}

async fn render_with_optional_sanitization<T, F>(
    response: &T,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
    render_text: F,
) -> Result<()>
where
    T: Serialize,
    F: Fn(&T),
{
    if let Some(template) = sanitize_template {
        let resolved = auth::resolve(auth_opts).await?;
        let json_val = serde_json::to_value(response)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            let safe_format = if matches!(format, OutputFormat::Text) {
                OutputFormat::Json
            } else {
                format.clone()
            };
            return output::render(&sanitize_result.content, &safe_format);
        }
    }

    if *format == OutputFormat::Text {
        render_text(response);
        return Ok(());
    }

    output::render(response, format)
}

fn render_schema_text(result: &SchemaDescribeResult) {
    println!(
        "Profile: {}  Source: {}  Database: {}",
        result.profile_name, result.source_type, result.database_id
    );
    println!("Columns: {}", result.rows.len());
    if let Some(ref sql) = result.sql {
        println!("Generated SQL: {}", sql);
    }
    for row in &result.rows {
        match row.table_schema.as_deref() {
            Some(schema) => println!(
                "  {}.{}.{}  {}{}",
                schema,
                row.table_name,
                row.column_name,
                row.data_type,
                row.is_nullable
                    .as_deref()
                    .map(|v| format!(" nullable={v}"))
                    .unwrap_or_default()
            ),
            None => println!(
                "  {}.{}  {}{}",
                row.table_name,
                row.column_name,
                row.data_type,
                row.is_nullable
                    .as_deref()
                    .map(|v| format!(" nullable={v}"))
                    .unwrap_or_default()
            ),
        }
    }
}

fn render_database_list_text(result: &DatabaseListResult) {
    println!(
        "Profile: {}  Source: {}",
        result.profile_name, result.source_type
    );
    if let Some(ref cluster_id) = result.cluster_id {
        println!("Cluster: {}", cluster_id);
    }
    if let Some(ref instance_id) = result.instance_id {
        println!("Instance: {}", instance_id);
    }
    println!("Databases: {}", result.rows.len());
    if let Some(ref sql) = result.sql {
        println!("Generated SQL: {}", sql);
    }
    for row in &result.rows {
        println!("  {}", row.database_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::profiles::SourceType;

    struct MockQueryDataExecutor {
        response: CaQuestionResponse,
    }

    #[async_trait]
    impl QueryDataExecutor for MockQueryDataExecutor {
        async fn ask_querydata(
            &self,
            _profile: &CaProfile,
            _prompt: &str,
        ) -> Result<CaQuestionResponse> {
            Ok(self.response.clone())
        }
    }

    fn spanner_profile() -> CaProfile {
        CaProfile {
            name: "spanner-finance".into(),
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
            cluster_id: None,
            instance_id: Some("finance".into()),
            database_id: Some("ledger".into()),
            db_type: None,
        }
    }

    fn alloydb_profile() -> CaProfile {
        CaProfile {
            name: "alloydb-ops".into(),
            source_type: SourceType::AlloyDb,
            project: "my-project".into(),
            location: Some("us-central1".into()),
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: Some("ctx-ops".into()),
            cluster_id: Some("ops".into()),
            instance_id: Some("primary".into()),
            database_id: Some("opsdb".into()),
            db_type: None,
        }
    }

    #[test]
    fn schema_prompt_mentions_spanner() {
        let prompt = schema_prompt(&spanner_profile());
        assert!(prompt.contains("Spanner"));
        assert!(prompt.contains("table_name"));
    }

    #[test]
    fn schema_prompt_mentions_alloydb_postgresql() {
        let prompt = schema_prompt(&alloydb_profile());
        assert!(prompt.contains("AlloyDB PostgreSQL"));
        assert!(prompt.contains("table_schema"));
        assert!(prompt.contains("pg_catalog"));
    }

    #[test]
    fn extract_schema_rows_accepts_aliases() {
        let rows = vec![serde_json::json!({
            "schema_name": "public",
            "table": "users",
            "column": "id",
            "type": "STRING",
            "nullable": "NO"
        })
        .as_object()
        .unwrap()
        .clone()];

        let parsed = extract_schema_rows(&rows).unwrap();
        assert_eq!(parsed[0].table_schema.as_deref(), Some("public"));
        assert_eq!(parsed[0].table_name, "users");
        assert_eq!(parsed[0].column_name, "id");
    }

    #[tokio::test]
    async fn run_schema_describe_with_executor_builds_result() {
        let executor = MockQueryDataExecutor {
            response: CaQuestionResponse {
                question: "schema".into(),
                agent: None,
                sql: Some("SELECT ...".into()),
                results: vec![serde_json::json!({
                    "table_name": "users",
                    "column_name": "id",
                    "data_type": "STRING",
                    "is_nullable": "NO"
                })
                .as_object()
                .unwrap()
                .clone()],
                explanation: Some("schema summary".into()),
            },
        };

        let result = run_schema_describe_with_executor(&executor, &spanner_profile())
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.database_id, "ledger");
        assert_eq!(result.rows[0].table_name, "users");
    }

    #[test]
    fn extract_database_rows_uses_first_string_value_as_fallback() {
        let rows = vec![serde_json::json!({
            "datname": "postgres"
        })
        .as_object()
        .unwrap()
        .clone()];

        let parsed = extract_database_rows(&rows).unwrap();
        assert_eq!(parsed[0].database_name, "postgres");
    }

    #[tokio::test]
    async fn run_alloydb_databases_list_with_executor_builds_result() {
        let executor = MockQueryDataExecutor {
            response: CaQuestionResponse {
                question: "dbs".into(),
                agent: None,
                sql: Some("SELECT datname FROM pg_database".into()),
                results: vec![serde_json::json!({
                    "database_name": "opsdb"
                })
                .as_object()
                .unwrap()
                .clone()],
                explanation: None,
            },
        };

        let result = run_alloydb_databases_list_with_executor(&executor, &alloydb_profile())
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].database_name, "opsdb");
    }

    #[tokio::test]
    async fn run_schema_describe_with_executor_works_for_alloydb() {
        let executor = MockQueryDataExecutor {
            response: CaQuestionResponse {
                question: "schema".into(),
                agent: None,
                sql: Some("SELECT table_schema, table_name, column_name, data_type, is_nullable FROM information_schema.columns".into()),
                results: vec![serde_json::json!({
                    "table_schema": "public",
                    "table_name": "orders",
                    "column_name": "id",
                    "data_type": "integer",
                    "is_nullable": "NO"
                })
                .as_object()
                .unwrap()
                .clone()],
                explanation: Some("AlloyDB schema".into()),
            },
        };

        let result = run_schema_describe_with_executor(&executor, &alloydb_profile())
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.source_type, SourceType::AlloyDb.to_string());
        assert_eq!(result.database_id, "opsdb");
        assert_eq!(result.rows[0].table_schema.as_deref(), Some("public"));
        assert_eq!(result.rows[0].table_name, "orders");
    }
}
