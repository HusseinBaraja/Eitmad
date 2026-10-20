//! Fresh offline host preparation. Native installation mechanics stay in the Windows adapter.
#[cfg(windows)]
mod guided;
#[cfg(windows)]
mod installation;
#[cfg(windows)]
mod preparation;

pub(super) async fn guided() -> Result<(), super::MainError> {
    #[cfg(windows)]
    {
        guided::execute().await
    }
    #[cfg(not(windows))]
    {
        Err(super::MainError::Setup)
    }
}

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

pub(super) async fn install(command: eitmad_server::ServerCommand) -> Result<(), super::MainError> {
    #[cfg(windows)]
    {
        installation::execute(command).await
    }
    #[cfg(not(windows))]
    {
        let _ = command;
        Err(super::MainError::Setup)
    }
}
