use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "bqx", version, about = "Agent-native BigQuery CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// GCP project ID
    #[arg(long, global = true, env = "BQX_PROJECT")]
    pub project_id: Option<String>,

    /// BigQuery dataset
    #[arg(long, global = true, env = "BQX_DATASET")]
    pub dataset_id: Option<String>,

    /// BigQuery location
    #[arg(long, global = true, env = "BQX_LOCATION", default_value = "US")]
    pub location: String,

    /// Table name
    #[arg(long, global = true, default_value = "agent_events")]
    pub table: String,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    pub format: OutputFormat,
}

#[derive(Subcommand)]
pub enum Command {
    /// BigQuery jobs operations
    Jobs {
        #[command(subcommand)]
        command: JobsCommand,
    },
    /// Agent analytics operations
    Analytics {
        #[command(subcommand)]
        command: AnalyticsCommand,
    },
}

#[derive(Subcommand)]
pub enum JobsCommand {
    /// Execute a SQL query
    Query {
        /// SQL query string
        #[arg(long)]
        query: String,

        /// Use legacy SQL
        #[arg(long, default_value = "false")]
        use_legacy_sql: bool,

        /// Dry run (show request without executing)
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum AnalyticsCommand {
    /// Health check on BigQuery table and configuration
    Doctor,

    /// Evaluate agent sessions against a threshold
    Evaluate {
        /// Evaluator type
        #[arg(long)]
        evaluator: EvaluatorType,

        /// Pass/fail threshold (ms for latency, 0-1 for rates)
        #[arg(long)]
        threshold: f64,

        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Return exit code 1 on evaluation failure
        #[arg(long)]
        exit_code: bool,
    },

    /// Retrieve a session trace
    GetTrace {
        /// Session ID to retrieve
        #[arg(long)]
        session_id: String,
    },
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
}

#[derive(Clone, ValueEnum)]
pub enum EvaluatorType {
    Latency,
    ErrorRate,
}
