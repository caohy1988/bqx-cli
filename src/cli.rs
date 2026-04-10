use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "dcx", version, about = "Agent-native Data Cloud CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// GCP project ID
    #[arg(long, global = true, env = "DCX_PROJECT")]
    pub project_id: Option<String>,

    /// BigQuery dataset
    #[arg(long, global = true, env = "DCX_DATASET")]
    pub dataset_id: Option<String>,

    /// BigQuery location
    #[arg(long, global = true, env = "DCX_LOCATION", default_value = "US")]
    pub location: String,

    /// Table name
    #[arg(long, global = true, default_value = "agent_events")]
    pub table: String,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    pub format: OutputFormat,

    /// Bearer token for authentication (overrides all other auth methods)
    #[arg(long, global = true, env = "DCX_TOKEN", hide = true)]
    pub token: Option<String>,

    /// Path to service account credentials JSON file
    #[arg(long, global = true, env = "DCX_CREDENTIALS_FILE")]
    pub credentials_file: Option<String>,

    /// Model Armor template for response sanitization
    /// (e.g. projects/my-proj/locations/us-central1/templates/my-template)
    #[arg(long, global = true, env = "DCX_SANITIZE_TEMPLATE")]
    pub sanitize: Option<String>,

    /// Pagination token from a previous response's next_page_token
    #[arg(long, global = true)]
    pub page_token: Option<String>,

    /// Automatically fetch all pages (follows nextPageToken until exhausted)
    #[arg(long, global = true)]
    pub page_all: bool,

    /// Skip confirmation prompts (required for non-interactive / CI execution)
    #[arg(long, global = true, env = "DCX_YES")]
    pub yes: bool,
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
    /// Authentication management
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Conversational Analytics operations
    Ca {
        #[command(subcommand)]
        command: CaCommand,
    },
    /// Generate SKILL.md and agents/openai.yaml for BigQuery API commands
    GenerateSkills {
        /// Output directory for generated skill files
        #[arg(long, default_value = "./skills")]
        output_dir: String,

        /// Generate only skills matching these names (e.g. dcx-datasets)
        #[arg(long)]
        filter: Vec<String>,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellType,
    },
    /// Manage and inspect source profiles
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    /// Machine-readable command contract introspection
    Meta {
        #[command(subcommand)]
        command: MetaCommand,
    },
    /// Start MCP (Model Context Protocol) server on stdio
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Subcommand)]
pub enum McpCommand {
    /// Start the MCP server (reads JSON-RPC from stdin, writes to stdout)
    Serve,
}

#[derive(Clone, ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
}

#[derive(Subcommand)]
pub enum MetaCommand {
    /// List all commands with domain and synopsis
    Commands,
    /// Describe a single command's full contract
    Describe {
        /// Command path segments (e.g., analytics evaluate)
        #[arg(required = true)]
        path: Vec<String>,
    },
    /// Generate Gemini extension manifest from the command contract
    GeminiTools,
}

#[derive(Subcommand)]
pub enum ProfilesCommand {
    /// List all discoverable profiles
    List,
    /// Show a profile's resolved configuration (secrets redacted)
    Show {
        /// Profile name or path to YAML file
        #[arg(long)]
        profile: String,
    },
    /// Validate a profile's structure
    Validate {
        /// Profile name or path to YAML file
        #[arg(long)]
        profile: String,
    },
    /// Validate structure and test authentication (network call)
    Test {
        /// Profile name or path to YAML file
        #[arg(long)]
        profile: String,
    },
}

#[derive(Subcommand)]
pub enum LookerCommand {
    /// Looker explore operations
    Explores {
        #[command(subcommand)]
        command: LookerExploresCommand,
    },
    /// Looker dashboard operations
    Dashboards {
        #[command(subcommand)]
        command: LookerDashboardsCommand,
    },
}

#[derive(Subcommand)]
pub enum LookerExploresCommand {
    /// List all explores from the Looker instance
    List {
        /// Looker profile name or path
        #[arg(long)]
        profile: String,
    },
    /// Get detailed metadata for a single explore
    Get {
        /// Looker profile name or path
        #[arg(long)]
        profile: String,
        /// Explore reference (model/explore, e.g. sales_model/orders)
        #[arg(long)]
        explore: String,
    },
}

#[derive(Subcommand)]
pub enum LookerDashboardsCommand {
    /// List all dashboards from the Looker instance
    List {
        /// Looker profile name or path
        #[arg(long)]
        profile: String,
    },
    /// Get detailed metadata for a single dashboard
    Get {
        /// Looker profile name or path
        #[arg(long)]
        profile: String,
        /// Dashboard ID
        #[arg(long)]
        dashboard_id: String,
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

        /// LLM judge criterion (correctness, hallucination, sentiment)
        #[arg(long, default_value = "correctness")]
        criterion: String,

        /// Maximum number of sessions to evaluate
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Fail sessions with unparseable judge output
        #[arg(long)]
        strict: bool,

        /// AI.GENERATE endpoint for LLM judge
        #[arg(long)]
        endpoint: Option<String>,

        /// BQ connection ID for AI.GENERATE
        #[arg(long)]
        connection_id: Option<String>,
    },

    /// Retrieve a session trace
    GetTrace {
        /// Session ID to retrieve
        #[arg(long)]
        session_id: Option<String>,

        /// Trace ID to retrieve
        #[arg(long)]
        trace_id: Option<String>,
    },

    /// List recent traces matching filter criteria
    ListTraces {
        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: String,

        /// Filter by session ID
        #[arg(long)]
        session_id: Option<String>,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Maximum number of traces to return
        #[arg(long, default_value = "100")]
        limit: u32,
    },

    /// Generate comprehensive agent insights report
    Insights {
        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Maximum number of sessions to analyze
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Max sessions for insights pipeline
        #[arg(long, default_value = "50")]
        max_sessions: u32,
    },

    /// Run drift detection against a golden question set
    Drift {
        /// Golden dataset table name (in the same dataset)
        #[arg(long)]
        golden_dataset: String,

        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long, default_value = "7d")]
        last: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Maximum number of sessions
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Minimum coverage threshold (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        min_coverage: f64,

        /// Return exit code 1 on drift failure
        #[arg(long)]
        exit_code: bool,
    },

    /// Analyze event distribution patterns
    Distribution {
        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Maximum number of sessions
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Analysis mode
        #[arg(long, default_value = "auto_group_using_semantics")]
        mode: String,

        /// Top items per category
        #[arg(long, default_value = "20")]
        top_k: u32,
    },

    /// Show human-in-the-loop interaction metrics
    HitlMetrics {
        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Maximum number of sessions to return
        #[arg(long, default_value = "100")]
        limit: u32,
    },

    /// Manage per-event-type BigQuery views
    Views {
        #[command(subcommand)]
        command: ViewsCommand,
    },

    /// Run categorical evaluation over agent traces
    CategoricalEval {
        /// JSON file with metric definitions
        #[arg(long)]
        metrics_file: String,

        /// Filter by agent name
        #[arg(long)]
        agent_id: Option<String>,

        /// Time window (e.g., 1h, 24h, 7d)
        #[arg(long)]
        last: Option<String>,

        /// Maximum number of sessions to evaluate
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Model endpoint for classification
        #[arg(long)]
        endpoint: Option<String>,

        /// Disable justification in output (justification included by default)
        #[arg(long)]
        no_include_justification: bool,

        /// Write results to BigQuery
        #[arg(long)]
        persist: bool,

        /// Destination table for persisted results
        #[arg(long)]
        results_table: Option<String>,

        /// Prompt version tag for reproducibility
        #[arg(long)]
        prompt_version: Option<String>,
    },

    /// Create dashboard views over categorical evaluation results
    CategoricalViews {
        /// Source results table name
        #[arg(long, default_value = "categorical_results")]
        results_table: String,

        /// View name prefix
        #[arg(long, default_value = "")]
        prefix: String,
    },
}

#[derive(Subcommand)]
pub enum ViewsCommand {
    /// Create views for all 18 standard event types
    CreateAll {
        /// Prefix for view names (e.g., "adk_" → adk_llm_request)
        #[arg(long, default_value = "")]
        prefix: String,

        /// Dry run (show SQL without executing)
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a view for a single event type
    Create {
        /// Event type to create a view for (e.g. LLM_REQUEST)
        event_type: String,

        /// Prefix for view names (e.g., "adk_" → adk_llm_request)
        #[arg(long, default_value = "")]
        prefix: String,

        /// Dry run (show SQL without executing)
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum CaCommand {
    /// Ask a natural language question via Conversational Analytics
    Ask {
        /// The question to ask
        #[arg()]
        question: String,

        /// CA source profile name or path to profile YAML file
        #[arg(long)]
        profile: Option<String>,

        /// Data agent to use (e.g. agent-analytics)
        #[arg(long)]
        agent: Option<String>,

        /// Table references for context (e.g. project.dataset.table)
        #[arg(long, value_delimiter = ',')]
        tables: Option<Vec<String>>,
    },

    /// Create a new Conversational Analytics data agent
    CreateAgent {
        /// Agent name / ID (alphanumeric, hyphens, underscores, dots)
        #[arg(long)]
        name: String,

        /// Table references (project.dataset.table)
        #[arg(long, value_delimiter = ',')]
        tables: Vec<String>,

        /// View references to include as additional data sources
        #[arg(long, value_delimiter = ',')]
        views: Option<Vec<String>>,

        /// Path to verified queries YAML file (defaults to bundled)
        #[arg(long)]
        verified_queries: Option<String>,

        /// System instructions for the agent
        #[arg(long)]
        instructions: Option<String>,

        /// Dry run (show request without executing)
        #[arg(long)]
        dry_run: bool,
    },

    /// List data agents in the project
    ListAgents,

    /// Add a verified query to an existing data agent
    AddVerifiedQuery {
        /// Agent ID to add the query to
        #[arg(long)]
        agent: String,

        /// Natural language question
        #[arg(long)]
        question: String,

        /// SQL query to associate with the question
        #[arg(long)]
        query: String,

        /// Dry run (show request without executing)
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum AuthCommand {
    /// Authenticate with Google OAuth (opens browser)
    Login,
    /// Show current authentication status
    Status,
    /// Clear stored credentials
    Logout,
    /// Preflight credential check (structured output for CI / agents)
    Check,
}

#[derive(Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Text,
}

#[derive(Clone, ValueEnum)]
pub enum EvaluatorType {
    Latency,
    ErrorRate,
    TurnCount,
    TokenEfficiency,
    Ttft,
    Cost,
    LlmJudge,
}
