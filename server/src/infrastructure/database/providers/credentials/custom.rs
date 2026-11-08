//! Generic credential store: the API key or bearer token of a user-registered
//! provider. A custom provider has no dedicated module of its own, so its
//! `providers.credentials` blob is read here.

use serde_json::Value;

use crate::constants;
use crate::error::APIError;
use crate::infrastructure::database::AppDatabase;

use super::credential_string;

/// The credential a custom provider signs its upstream request with. Never
/// enters a response or a log line.
#[derive(Clone, Debug, Default)]
pub struct CustomCredentials {
    pub id: String,
    pub api_key: Option<String>,
    pub access_token: Option<String>,
    /// Extra headers the operator supplied, sent as-is on every request.
    pub custom_headers: Vec<(String, String)>,
}

impl CustomCredentials {
    /// The token to sign with: the access token when present, else the API key,
    /// mirroring the Node executors' `accessToken || apiKey`.
    pub fn token(&self) -> Option<&str> {
        self.access_token
            .as_deref()
            .or(self.api_key.as_deref())
            .filter(|value| !value.trim().is_empty())
    }
}

/// Loads the credentials of one stored provider row by its internal id.
pub async fn load_custom_credentials(
    database: &AppDatabase,
    id: &str,
) -> Result<Option<CustomCredentials>, APIError> {
    let Some(pool) = database.sqlite_pool() else {
        return Ok(None);
    };

    let raw = sqlx::query_scalar::<_, String>("SELECT credentials FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|error| {
            APIError::new(
                500,
                constants::database::with_context("read custom provider credentials", error),
            )
        })?;

    let Some(raw) = raw else {
        return Ok(None);
    };

    let mut creds = parse_custom_credentials(&raw);
    creds.id = id.to_owned();
    Ok(Some(creds))
}

/// Loads all enabled credentials for a custom provider, including its primary row
/// and any attached sub-connections (where provider_id = base_id).
pub async fn load_all_custom_credentials(
    database: &AppDatabase,
    base_id: &str,
) -> Result<Vec<CustomCredentials>, APIError> {
    let Some(pool) = database.sqlite_pool() else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query(
        "SELECT id, credentials FROM providers \
         WHERE (id = ? OR provider_id = ?) AND enabled = 1 \
         ORDER BY created_at DESC",
    )
    .bind(base_id)
    .bind(base_id)
    .fetch_all(&pool)
    .await
    .map_err(|error| {
        APIError::new(
            500,
            constants::database::with_context("read custom provider connection list", error),
        )
    })?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        use crate::infrastructure::database::row::text;
        let id_val = text(&row, "id")?;
        let cred_raw = text(&row, "credentials")?;
        let mut creds = parse_custom_credentials(&cred_raw);
        creds.id = id_val;
        if creds.token().is_some() {
            result.push(creds);
        }
    }

    Ok(result)
}

/// Reads one credential blob. The build writes snake_case; the camelCase
/// spellings are accepted too, so a row written by the Node build still reads.
pub fn parse_custom_credentials(raw: &str) -> CustomCredentials {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return CustomCredentials::default();
    };
    let Some(object) = value.as_object() else {
        return CustomCredentials::default();
    };

    CustomCredentials {
        id: String::new(),
        api_key: credential_string(object, &["api_key", "apiKey"]),
        access_token: credential_string(object, &["access_token", "accessToken"]),
        custom_headers: custom_headers(object, &value),
    }
}

/// Reads the custom headers from the credential blob's `meta.custom_headers`
/// object when it sits there, falling back to a top-level `custom_headers` map.
fn custom_headers(
    credentials: &serde_json::Map<String, Value>,
    whole: &Value,
) -> Vec<(String, String)> {
    let source = credentials.get("custom_headers").or_else(|| {
        whole
            .get("meta")
            .and_then(|meta| meta.get("custom_headers"))
    });

    source
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(name, value)| {
                    value.as_str().map(|value| (name.clone(), value.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_custom_credentials;

    #[test]
    fn reads_snake_and_camel_case_credentials() {
        let snake = parse_custom_credentials(r#"{"api_key":"sk-a"}"#);
        assert_eq!(snake.token(), Some("sk-a"));

        let camel = parse_custom_credentials(r#"{"apiKey":"sk-b"}"#);
        assert_eq!(camel.token(), Some("sk-b"));
    }

    #[test]
    fn access_token_wins_over_api_key() {
        let credentials = parse_custom_credentials(r#"{"api_key":"sk-a","access_token":"tok-b"}"#);
        assert_eq!(credentials.token(), Some("tok-b"));
    }

    #[test]
    fn reads_custom_headers_from_the_meta_object() {
        let credentials = parse_custom_credentials(
            r#"{"api_key":"k","meta":{"custom_headers":{"X-Tenant":"acme"}}}"#,
        );
        assert_eq!(
            credentials.custom_headers,
            vec![("X-Tenant".to_owned(), "acme".to_owned())]
        );
    }
}
