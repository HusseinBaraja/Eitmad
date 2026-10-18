//! Shared account administration. Results are returned only after the server commits.
use crate::authenticated_http::{AuthenticatedHttpClient, HttpError};
use eitmad_authorization::MutationContext;
use eitmad_contracts::{
    accounts::{DesktopAccountPage, DesktopAccountSummary},
    commands::Command,
    identity::{AuthorizationContext, OrganizationId, ScopeId},
    secrets::SecretId,
    server::RegisterBranchRequest,
    server_accounts::{
        ChangeServerAccount, ListServerAccounts, ServerAccount, ServerAccountChange,
        ServerAccountPage, ServerAccountResult, ServerAccountStatus, ServerAccountWatch,
        WatchServerAccounts,
    },
};
use eitmad_engine_runtime::{DesktopAccountAuthority, DesktopAccountError};
use eitmad_secret_storage::SecretStore;
use std::time::{Duration, Instant};

pub struct DirectAccountClient {
    http: AuthenticatedHttpClient,
    organization: OrganizationId,
    branch: ScopeId,
    credential: SecretId,
}

impl DirectAccountClient {
    #[must_use]
    pub fn from_config(
        config: crate::DirectServerConfig,
        secrets: SecretStore,
        credential: SecretId,
        organization: OrganizationId,
        branch: ScopeId,
    ) -> Self {
        Self {
            http: AuthenticatedHttpClient::from_config(
                config,
                secrets,
                credential.clone(),
                "eitmad.capability.shared-accounts.v1",
                25,
            ),
            organization,
            branch,
            credential,
        }
    }

    fn summary(
        &self,
        account: ServerAccount,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        let organization = account
            .organizations
            .iter()
            .find(|o| o.organization_id == self.organization)
            .ok_or(DesktopAccountError::Invalid)?;
        let role = match organization.assignments.as_slice() {
            [assignment] => Some(assignment.role),
            _ => None,
        };
        Ok(DesktopAccountSummary {
            account_id: account.account_id,
            user_id: account.user_id,
            display_name: account.display_name,
            username: account.username,
            role,
            active: account.status == ServerAccountStatus::Active,
            revision: account.revision,
        })
    }

    /// Waits for account-policy changes, then authenticates this device's current session again.
    /// # Errors
    /// Distinguishes a confirmed denial from a lost connection; only a denial revokes local grants.
    pub fn watch(
        &self,
        actor: &AuthorizationContext,
        revision: u64,
    ) -> Result<ServerAccountWatch, DesktopAccountError> {
        // Missing local credentials do not prove that a server revoked the account.
        self.http
            .driver
            .load_credential(&crate::account_credential_id(
                &self.credential,
                actor.identity.principal_id,
            ))
            .map_err(|_| DesktopAccountError::Unavailable)?;
        self.http
            .request(
                actor,
                "/v1/accounts/watch",
                &WatchServerAccounts {
                    registration: RegisterBranchRequest {
                        organization_id: self.organization,
                        branch_id: self.branch,
                    },
                    revision,
                },
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|error| map_error(&error))
    }
}

impl DesktopAccountAuthority for DirectAccountClient {
    fn authorize(&self, actor: &AuthorizationContext) -> Result<(), DesktopAccountError> {
        self.http
            .request::<ServerAccountPage>(
                actor,
                "/v1/accounts/list",
                &ListServerAccounts {
                    after: None,
                    limit: 1,
                },
                Instant::now() + Duration::from_secs(10),
            )
            .map(|_| ())
            .map_err(|error| map_error(&error))
    }

    fn list(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<DesktopAccountPage, DesktopAccountError> {
        let mut after = None;
        let mut accounts = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let page: ServerAccountPage = self
                .http
                .request(
                    actor,
                    "/v1/accounts/list",
                    &ListServerAccounts { after, limit: 100 },
                    deadline,
                )
                .map_err(|error| map_error(&error))?;
            for account in page.accounts {
                if account
                    .organizations
                    .iter()
                    .any(|o| o.organization_id == self.organization)
                {
                    accounts.push(self.summary(account)?);
                }
            }
            match page.next_after {
                Some(next) if Some(next) != after => after = Some(next),
                None => return Ok(DesktopAccountPage { accounts }),
                _ => return Err(DesktopAccountError::Unavailable),
            }
        }
    }

    fn change(
        &self,
        context: &MutationContext,
        command: &Command,
    ) -> Result<DesktopAccountSummary, DesktopAccountError> {
        let change = match command {
            Command::CreateDesktopAccount(account) => ServerAccountChange::CreateDesktop {
                organization_id: self.organization,
                branch_id: self.branch,
                account: account.clone(),
            },
            Command::UpdateDesktopAccount(account) => ServerAccountChange::UpdateDesktop {
                organization_id: self.organization,
                branch_id: self.branch,
                account: account.clone(),
            },
            Command::DeactivateDesktopAccount(account) => ServerAccountChange::Deactivate {
                account_id: account.account_id,
                expected_revision: account.expected_revision,
            },
            _ => return Err(DesktopAccountError::Invalid),
        };
        let result: ServerAccountResult = self
            .http
            .request(
                &context.authorization,
                "/v1/accounts/transition",
                &ChangeServerAccount {
                    idempotency_key: context.idempotency_key,
                    change,
                },
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|error| map_error(&error))?;
        self.summary(result.receipt.account)
    }
}

fn map_error(error: &HttpError) -> DesktopAccountError {
    match error {
        HttpError::Denied => DesktopAccountError::Denied,
        HttpError::Conflict => DesktopAccountError::RevisionConflict,
        HttpError::Invalid | HttpError::NotFound => DesktopAccountError::Invalid,
        _ => DesktopAccountError::Unavailable,
    }
}
