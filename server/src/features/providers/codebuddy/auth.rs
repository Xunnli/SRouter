//! CodeBuddy credentials and the flavor's base header vocabulary.
//!
//! There is deliberately no token refresh: the OAuth login returns a token that
//! is valid for about a year, so an expired token surfaces as an upstream error
//! and the operator reconnects.

use std::collections::BTreeMap;

use super::executor::CodeBuddyExecutor;
use crate::constants;
use crate::error::APIError;
use crate::features::providers::rotation::round_robin_enabled;
use crate::infrastructure::database::providers::{
    CodeBuddyCredentials, load_all_codebuddy_credentials,
};

impl CodeBuddyExecutor {
    pub(super) async fn candidates(&self) -> Result<Vec<CodeBuddyCredentials>, APIError> {
        let database = self.database.as_ref().ok_or_else(|| {
            APIError::new(500, constants::providers::codebuddy::DATABASE_REQUIRED)
        })?;

        let list = load_all_codebuddy_credentials(database, self.flavor.provider_id()).await?;
        if list.is_empty() {
            return Err(APIError::new(
                401,
                constants::providers::codebuddy::NOT_CONNECTED,
            ));
        }
        Ok(list)
    }

    pub(super) async fn pick_account(
        &self,
        candidates: &[CodeBuddyCredentials],
    ) -> Result<CodeBuddyCredentials, APIError> {
        let database = self.database.as_ref().ok_or_else(|| {
            APIError::new(500, constants::providers::codebuddy::DATABASE_REQUIRED)
        })?;
        let enabled = round_robin_enabled(database, self.flavor.provider_id()).await;
        let ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();
        Ok(candidates[self.rotator.choose(enabled, &ids)].clone())
    }

    pub(super) async fn credentials(&self) -> Result<CodeBuddyCredentials, APIError> {
        let candidates = self.candidates().await?;
        self.pick_account(&candidates).await
    }

    /// The header vocabulary the Node oracle sets on every request, minus the
    /// per-request `Content-Type`/`Accept`/`Authorization`.
    pub(super) fn base_headers(&self) -> BTreeMap<&'static str, String> {
        let mut headers = BTreeMap::new();
        headers.insert("User-Agent", self.flavor.user_agent().to_owned());
        headers.insert("X-Product", "SaaS".to_owned());
        headers.insert("X-IDE-Type", self.flavor.ide_name().to_owned());
        headers.insert("X-IDE-Name", self.flavor.ide_name().to_owned());
        headers.insert("x-requested-with", "XMLHttpRequest".to_owned());
        headers.insert("x-codebuddy-request", "1".to_owned());
        if let Some(domain) = self.endpoints.domain {
            headers.insert("X-Domain", domain.to_owned());
        }
        headers
    }
}
