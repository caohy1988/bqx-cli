use dcx::config::{parse_duration, validate_agent_id, validate_identifier, validate_session_id};

// ── parse_duration ──

#[test]
fn parse_duration_hours() {
    let d = parse_duration("24h").unwrap();
    assert_eq!(d.interval_sql, "INTERVAL 24 HOUR");
}

#[test]
fn parse_duration_days() {
    let d = parse_duration("7d").unwrap();
    assert_eq!(d.interval_sql, "INTERVAL 7 DAY");
}

#[test]
fn parse_duration_minutes() {
    let d = parse_duration("30m").unwrap();
    assert_eq!(d.interval_sql, "INTERVAL 30 MINUTE");
}

#[test]
fn parse_duration_single_digit() {
    let d = parse_duration("1h").unwrap();
    assert_eq!(d.interval_sql, "INTERVAL 1 HOUR");
}

#[test]
fn parse_duration_rejects_empty() {
    assert!(parse_duration("").is_err());
}

#[test]
fn parse_duration_rejects_no_unit() {
    assert!(parse_duration("24").is_err());
}

#[test]
fn parse_duration_rejects_invalid_unit() {
    assert!(parse_duration("24x").is_err());
}

#[test]
fn parse_duration_rejects_negative() {
    assert!(parse_duration("-1h").is_err());
}

#[test]
fn parse_duration_rejects_float() {
    assert!(parse_duration("1.5h").is_err());
}

#[test]
fn parse_duration_rejects_spaces() {
    assert!(parse_duration("24 h").is_err());
}

// ── validate_identifier ──

#[test]
fn validate_identifier_accepts_alphanumeric() {
    assert!(validate_identifier("my_project", "project_id").is_ok());
}

#[test]
fn validate_identifier_accepts_hyphens() {
    assert!(validate_identifier("my-project-123", "project_id").is_ok());
}

#[test]
fn validate_identifier_accepts_underscores() {
    assert!(validate_identifier("my_dataset_v2", "dataset_id").is_ok());
}

#[test]
fn validate_identifier_rejects_empty() {
    assert!(validate_identifier("", "project_id").is_err());
}

#[test]
fn validate_identifier_rejects_spaces() {
    assert!(validate_identifier("my project", "project_id").is_err());
}

#[test]
fn validate_identifier_rejects_special_chars() {
    assert!(validate_identifier("my@project", "project_id").is_err());
}

#[test]
fn validate_identifier_rejects_leading_hyphen() {
    assert!(validate_identifier("-bad", "project_id").is_err());
}

#[test]
fn validate_identifier_rejects_semicolon() {
    assert!(validate_identifier("proj;DROP", "project_id").is_err());
}

// ── validate_session_id ──

#[test]
fn validate_session_id_accepts_uuid_like() {
    assert!(validate_session_id("adcp-a20d176b82af").is_ok());
}

#[test]
fn validate_session_id_accepts_dots() {
    assert!(validate_session_id("session.v1.abc").is_ok());
}

#[test]
fn validate_session_id_accepts_underscores() {
    assert!(validate_session_id("session_123_abc").is_ok());
}

#[test]
fn validate_session_id_rejects_empty() {
    assert!(validate_session_id("").is_err());
}

#[test]
fn validate_session_id_rejects_spaces() {
    assert!(validate_session_id("bad session").is_err());
}

#[test]
fn validate_session_id_rejects_sql_injection() {
    assert!(validate_session_id("x' OR '1'='1").is_err());
}

// ── validate_agent_id ──

#[test]
fn validate_agent_id_accepts_simple() {
    assert!(validate_agent_id("sales_agent").is_ok());
}

#[test]
fn validate_agent_id_accepts_dotted() {
    assert!(validate_agent_id("agent.v2.prod").is_ok());
}

#[test]
fn validate_agent_id_rejects_empty() {
    assert!(validate_agent_id("").is_err());
}

#[test]
fn validate_agent_id_rejects_special() {
    assert!(validate_agent_id("agent;DROP TABLE").is_err());
}
