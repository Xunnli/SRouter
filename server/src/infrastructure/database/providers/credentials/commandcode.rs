//! Command Code credential store: the static key behind `commandcode`.

use serde_json::Value;

use crate::clock::now_ms;
use crate::constants;
use crate::error::APIError;
use crate::infrastructure::database::AppDatabase;
use crate::infrastructure::database::row::text;

use super::{ConnectionRow, credential_string, upsert_connection};

/// The connection row the sign-in routes write once the studio hands a key back.
#[derive(Clone, Debug)]
pub struct CommandCodeConnectionWrite {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub access_token: String,
    /// The account the studio or `/alpha/whoami` named, when it named one. The
    /// account check reads `user_id` as this connection's identity.
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub base_url: String,
}

pub async fn upsert_commandcode_connection(
    database: &AppDatabase,
    write: &CommandCodeConnectionWrite,
) -> Result<(), APIError> {
    let credentials = serde_json::json!({
        "access_token": write.access_token,
        "user_id": write.user_id,
        "user_name": write.user_name,
        "last_refreshed_at": now_ms(),
    });

    upsert_connection(
        database,
        &ConnectionRow {
            id: &write.id,
            provider_id: &write.provider_id,
            name: &write.name,
            category: "oauth",
            protocol: "openai",
            base_url: Some(&write.base_url),
            credentials,
            action: constants::providers::commandcode::STORE_CONNECTION,
        },
    )
    .await
}

/// The key inference needs. Everything else in the row is an account label, so
/// there is no token to refresh and no expiry to sweep.
#[derive(Clone, Debug)]
pub struct CommandCodeCredentials {
    pub id: String,
    pub access_token: String,
}

/// Loads all keys of enabled connections whose `provider_id` matches, newest first.
pub async fn load_all_commandcode_credentials(
    database: &AppDatabase,
    provider_id: &str,
) -> Result<Vec<CommandCodeCredentials>, APIError> {
    let Some(pool) = database.sqlite_pool() else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query(
        "SELECT id, credentials FROM providers
         WHERE provider_id = ? AND enabled = 1
         ORDER BY created_at DESC",
    )
    .bind(provider_id)
    .fetch_all(&pool)
    .await
    .map_err(|error| {
        APIError::new(
            500,
            constants::database::with_context("read Command Code credentials", error),
        )
    })?;

    let mut result = Vec::new();
    for row in &rows {
        let raw = text(row, "credentials")?;

        if let Some(mut credentials) = parse_commandcode_credentials(&raw) {
            credentials.id = text(row, "id")?;
            result.push(credentials);
        }
    }

    Ok(result)
}

/// Loads the key of the newest enabled connection whose `provider_id` matches.
pub async fn load_commandcode_credentials(
    database: &AppDatabase,
    provider_id: &str,
) -> Result<Option<CommandCodeCredentials>, APIError> {
    let list = load_all_commandcode_credentials(database, provider_id).await?;
    Ok(list.into_iter().next())
}

/// Parses a credentials JSON row. Malformed JSON or a missing key yields `None`,
/// which the executor reports as "not connected".
fn parse_commandcode_credentials(raw: &str) -> Option<CommandCodeCredentials> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let object = value.as_object()?;

    Some(CommandCodeCredentials {
        id: String::new(),
        access_token: credential_string(object, &["access_token", "accessToken"])?,
    })
}
