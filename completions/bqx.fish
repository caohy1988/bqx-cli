# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_bqx_global_optspecs
	string join \n project-id= dataset-id= location= table= format= token= credentials-file= sanitize= h/help V/version
end

function __fish_bqx_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_bqx_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_bqx_using_subcommand
	set -l cmd (__fish_bqx_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c bqx -n "__fish_bqx_needs_command" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_needs_command" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_needs_command" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_needs_command" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_needs_command" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_needs_command" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_needs_command" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_needs_command" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_needs_command" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_needs_command" -s V -l version -d 'Print version'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "jobs" -d 'BigQuery jobs operations'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "analytics" -d 'Agent analytics operations'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "auth" -d 'Authentication management'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "ca" -d 'Conversational Analytics operations'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "generate-skills" -d 'Generate SKILL.md and agents/openai.yaml for BigQuery API commands'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "completions" -d 'Generate shell completion scripts'
complete -c bqx -n "__fish_bqx_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -f -a "query" -d 'Execute a SQL query'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and not __fish_seen_subcommand_from query help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l query -d 'SQL query string' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l use-legacy-sql -d 'Use legacy SQL'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -l dry-run -d 'Dry run (show request without executing)'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from query" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "query" -d 'Execute a SQL query'
complete -c bqx -n "__fish_bqx_using_subcommand jobs; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "doctor" -d 'Health check on BigQuery table and configuration'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "evaluate" -d 'Evaluate agent sessions against a threshold'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "get-trace" -d 'Retrieve a session trace'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "list-traces" -d 'List recent traces matching filter criteria'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "insights" -d 'Generate comprehensive agent insights report'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "drift" -d 'Run drift detection against a golden question set'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "distribution" -d 'Analyze event distribution patterns'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "hitl-metrics" -d 'Show human-in-the-loop interaction metrics'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "views" -d 'Manage per-event-type BigQuery views'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and not __fish_seen_subcommand_from doctor evaluate get-trace list-traces insights drift distribution hitl-metrics views help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from doctor" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l evaluator -d 'Evaluator type' -r -f -a "latency\t''
error-rate\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l threshold -d 'Pass/fail threshold (ms for latency, 0-1 for rates)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -l exit-code -d 'Return exit code 1 on evaluation failure'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from evaluate" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l session-id -d 'Session ID to retrieve' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from get-trace" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l limit -d 'Maximum number of traces to return' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from list-traces" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from insights" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l golden-dataset -d 'Golden dataset table name (in the same dataset)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l min-coverage -d 'Minimum coverage threshold (0.0-1.0)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -l exit-code -d 'Return exit code 1 on drift failure'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from drift" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from distribution" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l last -d 'Time window (e.g., 1h, 24h, 7d)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l agent-id -d 'Filter by agent name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l limit -d 'Maximum number of sessions to return' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from hitl-metrics" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -f -a "create-all" -d 'Create views for all 18 standard event types'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from views" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "doctor" -d 'Health check on BigQuery table and configuration'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "evaluate" -d 'Evaluate agent sessions against a threshold'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "get-trace" -d 'Retrieve a session trace'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "list-traces" -d 'List recent traces matching filter criteria'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "insights" -d 'Generate comprehensive agent insights report'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "drift" -d 'Run drift detection against a golden question set'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "distribution" -d 'Analyze event distribution patterns'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "hitl-metrics" -d 'Show human-in-the-loop interaction metrics'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "views" -d 'Manage per-event-type BigQuery views'
complete -c bqx -n "__fish_bqx_using_subcommand analytics; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -f -a "login" -d 'Authenticate with Google OAuth (opens browser)'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -f -a "status" -d 'Show current authentication status'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -f -a "logout" -d 'Clear stored credentials'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and not __fish_seen_subcommand_from login status logout help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from login" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from logout" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "login" -d 'Authenticate with Google OAuth (opens browser)'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "status" -d 'Show current authentication status'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "logout" -d 'Clear stored credentials'
complete -c bqx -n "__fish_bqx_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -f -a "ask" -d 'Ask a natural language question via Conversational Analytics'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -f -a "create-agent" -d 'Create a new Conversational Analytics data agent'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -f -a "list-agents" -d 'List data agents in the project'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -f -a "add-verified-query" -d 'Add a verified query to an existing data agent'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and not __fish_seen_subcommand_from ask create-agent list-agents add-verified-query help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l agent -d 'Data agent to use (e.g. agent-analytics)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l tables -d 'Table references for context (e.g. project.dataset.table)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from ask" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l name -d 'Agent name / ID (alphanumeric, hyphens, underscores, dots)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l tables -d 'Table references (project.dataset.table)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l views -d 'View references to include as additional data sources' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l verified-queries -d 'Path to verified queries YAML file (defaults to bundled)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l instructions -d 'System instructions for the agent' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from create-agent" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from list-agents" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l agent -d 'Agent ID to add the query to' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l question -d 'Natural language question' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l query -d 'SQL query to associate with the question' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from add-verified-query" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from help" -f -a "ask" -d 'Ask a natural language question via Conversational Analytics'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from help" -f -a "create-agent" -d 'Create a new Conversational Analytics data agent'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from help" -f -a "list-agents" -d 'List data agents in the project'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from help" -f -a "add-verified-query" -d 'Add a verified query to an existing data agent'
complete -c bqx -n "__fish_bqx_using_subcommand ca; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l output-dir -d 'Output directory for generated skill files' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l filter -d 'Generate only skills matching these names (e.g. bqx-datasets)' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand generate-skills" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l project-id -d 'GCP project ID' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l dataset-id -d 'BigQuery dataset' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l location -d 'BigQuery location' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l table -d 'Table name' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l format -d 'Output format' -r -f -a "json\t''
table\t''
text\t''"
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l token -d 'Bearer token for authentication (overrides all other auth methods)' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l credentials-file -d 'Path to service account credentials JSON file' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -l sanitize -d 'Model Armor template for response sanitization (e.g. projects/my-proj/locations/us-central1/templates/my-template)' -r
complete -c bqx -n "__fish_bqx_using_subcommand completions" -s h -l help -d 'Print help'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "jobs" -d 'BigQuery jobs operations'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "analytics" -d 'Agent analytics operations'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "auth" -d 'Authentication management'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "ca" -d 'Conversational Analytics operations'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "generate-skills" -d 'Generate SKILL.md and agents/openai.yaml for BigQuery API commands'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "completions" -d 'Generate shell completion scripts'
complete -c bqx -n "__fish_bqx_using_subcommand help; and not __fish_seen_subcommand_from jobs analytics auth ca generate-skills completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from jobs" -f -a "query" -d 'Execute a SQL query'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "doctor" -d 'Health check on BigQuery table and configuration'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "evaluate" -d 'Evaluate agent sessions against a threshold'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "get-trace" -d 'Retrieve a session trace'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "list-traces" -d 'List recent traces matching filter criteria'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "insights" -d 'Generate comprehensive agent insights report'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "drift" -d 'Run drift detection against a golden question set'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "distribution" -d 'Analyze event distribution patterns'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "hitl-metrics" -d 'Show human-in-the-loop interaction metrics'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from analytics" -f -a "views" -d 'Manage per-event-type BigQuery views'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "login" -d 'Authenticate with Google OAuth (opens browser)'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "status" -d 'Show current authentication status'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "logout" -d 'Clear stored credentials'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from ca" -f -a "ask" -d 'Ask a natural language question via Conversational Analytics'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from ca" -f -a "create-agent" -d 'Create a new Conversational Analytics data agent'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from ca" -f -a "list-agents" -d 'List data agents in the project'
complete -c bqx -n "__fish_bqx_using_subcommand help; and __fish_seen_subcommand_from ca" -f -a "add-verified-query" -d 'Add a verified query to an existing data agent'
