use std::io::IsTerminal;

/// Returns `true` when both stdin and stderr are attached to a terminal.
///
/// Checks stderr (not stdout) because dcx writes all interactive messages
/// to stderr — stdout is reserved for structured data. If either stdin or
/// stderr is piped, interactive flows (auth login, confirmation dialogs)
/// are unsafe and should require `--yes` or exit with a structured
/// confirmation envelope.
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}
