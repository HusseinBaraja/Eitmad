//! Fresh offline host preparation. Native installation mechanics stay in the Windows adapter.
#[cfg(windows)]
mod preparation;

pub(super) async fn prepare() -> Result<(), super::MainError> {
    #[cfg(windows)]
    {
        preparation::prepare().await
    }
    #[cfg(not(windows))]
    {
        Err(super::MainError::Setup)
    }
}
