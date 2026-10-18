//! Server-owned desktop account administration.
use eitmad_authorization::MutationContext;
use eitmad_contracts::{
    accounts::{DesktopAccountPage, DesktopAccountSummary},
    identity::AuthorizationContext,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopAccountError {
    Denied,
    Invalid,
    RevisionConflict,
    Unavailable,
}

/// Authenticated server authority for connected installations. No local fallback is permitted.
pub trait DesktopAccountAuthority: Send + Sync {
    /// Lists accounts only after the server confirms current administration authority.
    /// # Errors
    /// Returns denial, invalid input, or unavailable authority without a local fallback.
    fn list(&self, actor: &AuthorizationContext)
    -> Result<DesktopAccountPage, DesktopAccountError>;
    /// Checks current account administration authority without loading the account directory.
    /// # Errors
    /// Fails closed on denial or unavailable server authority.
    fn authorize(&self, actor: &AuthorizationContext) -> Result<(), DesktopAccountError>;

    /// Returns the committed server receipt for the authenticated actor's exact intent.
    /// # Errors
    /// Returns authorization, validation, revision, retry-intent, or availability failures.
    fn change(
        &self,
        context: &MutationContext,
        command: &eitmad_contracts::commands::Command,
    ) -> Result<DesktopAccountSummary, DesktopAccountError>;
}
