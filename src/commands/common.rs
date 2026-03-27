use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::output;

/// Apply Model Armor sanitization if configured, then render.
///
/// Shared by all source-native command families (Looker, Spanner, AlloyDB, Cloud SQL).
pub async fn maybe_sanitize_and_render<T: serde::Serialize>(
    response: &T,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    if let Some(template) = sanitize_template {
        let resolved = auth::resolve(auth_opts).await?;
        let json_val = serde_json::to_value(response)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, format);
        }
    }

    output::render(response, format)
}

/// Extract the last path segment from a GCP resource name.
///
/// e.g. "projects/p/instances/my-inst" → "my-inst"
pub fn resource_id(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_extracts_last_segment() {
        assert_eq!(resource_id("projects/p/instances/my-inst"), "my-inst");
        assert_eq!(
            resource_id("projects/p/locations/us-central1/clusters/c1"),
            "c1"
        );
        assert_eq!(resource_id("simple"), "simple");
        assert_eq!(resource_id(""), "");
    }
}
