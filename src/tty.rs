use std::io::IsTerminal;

/// Returns `true` when stdin is attached to a terminal.
///
/// Use this to decide whether interactive prompts (auth login, confirmation
/// dialogs) are safe to present. When `false`, commands that require user
/// interaction should either require `--yes` or exit with a structured
/// confirmation envelope.
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}
