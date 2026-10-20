use super::*;
use eitmad_contracts::{
    accounts::AccountPassword,
    setup::{InitializeWorkshop, PrepareWindowsWorkshop},
};

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("eitmad-install-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
    fn request(&self) -> InstallWindowsServer {
        let package = WindowsInstallPackage {
            directory: self.path("package"),
            release_sha256: "a".repeat(64),
            publisher: "CN=Synthetic".into(),
        };
        InstallWindowsServer {
            version: 1,
            client: InstallWindowsClient {
                version: 1,
                progress_directory: self.path("client-progress"),
                intended_user_sid: "S-1-5-21-123-456-789-1001".into(),
                desktop: package.clone(),
            },
            preparation: PrepareWindowsWorkshop {
                version: 1,
                state_directory: self.path("preparation"),
                platform_script: self.path("prepare.ps1"),
                postgres_bin: self.path("bin"),
                certificate_executable: self.path("certificate.exe"),
                server_name: "synthetic.invalid".into(),
                listen_address: "192.168.231.10".into(),
                server_port: 58443,
                database_port: 55439,
                workshop: InitializeWorkshop {
                    version: 1,
                    workshop_name: "ورشة اختبار".into(),
                    owner_username: "synthetic-owner".into(),
                    owner_password: AccountPassword::new("Synthetic-password-123!"),
                },
            },
            server: package,
            service_state_directory: self.path("host"),
            service_name: "Synthetic".into(),
            server_progress_directory: self.path("server-progress"),
            winsw_executable: self.path("winsw.exe"),
            winsw_sha256: "b".repeat(64),
            first_desktop: None,
            backup: None,
        }
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Default)]
struct FakePlatform {
    calls: Vec<Stage>,
    failure: Option<Stage>,
}
impl FakePlatform {
    fn call(&mut self, stage: Stage) -> Result<(), MainError> {
        self.calls.push(stage);
        if self.failure == Some(stage) {
            Err(MainError::Setup)
        } else {
            Ok(())
        }
    }
}
impl Platform for FakePlatform {
    fn client_preflight(&mut self, _: &InstallWindowsClient) -> Result<(), MainError> {
        self.call(Stage::Preflight)
    }
    fn server_preflight(&mut self, _: &InstallWindowsServer) -> Result<(), MainError> {
        self.call(Stage::Preflight)
    }
    fn elevate(
        &mut self,
        _: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<Option<eitmad_contracts::setup::SetupDesktopHandoff>, MainError> {
        self.call(Stage::Elevation)?;
        progress.begin(Stage::VerifyHttps)?;
        progress.record(Stage::VerifyHttps, Status::Succeeded)?;
        Ok(None)
    }
    async fn prepare(
        &mut self,
        _: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<(), MainError> {
        for stage in [
            Stage::PrepareWorkshop,
            Stage::ApplyMigrations,
            Stage::InitializeOwner,
            Stage::StopPreparedDatabase,
        ] {
            progress.begin(stage)?;
            self.call(stage)?;
        }
        Ok(())
    }
    fn host(&mut self, _: &InstallWindowsServer, action: &str) -> Result<(), MainError> {
        self.call(match action {
            "Install" => Stage::InstallServices,
            "Firewall" => Stage::ConfigureFirewall,
            "Start" => Stage::VerifyHttps,
            _ => panic!("Unexpected host action"),
        })
    }
    fn desktop(&mut self, _: &InstallWindowsClient) -> Result<(), MainError> {
        self.call(Stage::RegisterDesktop)
    }
}

#[tokio::test]
async fn client_registers_only_desktop_and_refuses_replacement() {
    let root = TestRoot::new();
    let request = InstallWindows::Client(root.request().client);
    let mut platform = FakePlatform::default();
    install(&request, &mut platform).await.unwrap();
    assert_eq!(platform.calls, [Stage::Preflight, Stage::RegisterDesktop]);
    let before = fs::read(root.0.join("client-progress/00000003.json")).unwrap();
    assert!(install(&request, &mut platform).await.is_err());
    assert_eq!(
        fs::read(root.0.join("client-progress/00000003.json")).unwrap(),
        before
    );
    assert_eq!(platform.calls.len(), 2);
    let state = read_state(&root.0.join("client-progress")).unwrap();
    assert!(!state.interrupted);
    assert_eq!(state.progress.status, Status::Succeeded);
}

#[tokio::test]
async fn server_completes_all_host_stages_without_registering_desktop_as_administrator() {
    let root = TestRoot::new();
    let request = root.request();
    let mut platform = FakePlatform::default();
    install_server(&request, &mut platform).await.unwrap();
    assert_eq!(
        platform.calls,
        [
            Stage::Preflight,
            Stage::PrepareWorkshop,
            Stage::ApplyMigrations,
            Stage::InitializeOwner,
            Stage::StopPreparedDatabase,
            Stage::InstallServices,
            Stage::ConfigureFirewall,
            Stage::VerifyHttps
        ]
    );
    let state = read_state(Path::new(&request.server_progress_directory)).unwrap();
    assert_eq!(state.progress.stage, Stage::Complete);
    assert_eq!(state.progress.status, Status::Succeeded);
    platform.calls.clear();
    install(&InstallWindows::Server(Box::new(request)), &mut platform)
        .await
        .unwrap();
    assert_eq!(
        platform.calls,
        [Stage::Preflight, Stage::Elevation, Stage::RegisterDesktop]
    );
}

#[tokio::test]
async fn each_host_failure_retains_the_exact_stage_and_blocks_reinitialization() {
    for stage in [
        Stage::PrepareWorkshop,
        Stage::ApplyMigrations,
        Stage::InitializeOwner,
        Stage::StopPreparedDatabase,
        Stage::InstallServices,
        Stage::ConfigureFirewall,
        Stage::VerifyHttps,
    ] {
        let root = TestRoot::new();
        let request = root.request();
        let mut platform = FakePlatform {
            failure: Some(stage),
            ..Default::default()
        };
        assert!(install_server(&request, &mut platform).await.is_err());
        assert_eq!(platform.calls.last(), Some(&stage));
        let state = read_state(Path::new(&request.server_progress_directory)).unwrap();
        assert_eq!(
            (state.progress.stage, state.progress.status),
            (stage, Status::Failed)
        );
        assert!(!state.interrupted);
        let count = platform.calls.len();
        assert!(install_server(&request, &mut platform).await.is_err());
        assert_eq!(platform.calls.len(), count + 1); // read-only preflight, no native stage repeated
        assert_eq!(
            read_state(Path::new(&request.server_progress_directory))
                .unwrap()
                .progress,
            state.progress
        );
    }
}

#[tokio::test]
async fn denied_elevation_or_desktop_failure_never_reports_success() {
    for failed in [Stage::Preflight, Stage::Elevation, Stage::RegisterDesktop] {
        let root = TestRoot::new();
        let request = root.request();
        let mut platform = FakePlatform {
            failure: Some(failed),
            ..Default::default()
        };
        assert!(
            install(
                &InstallWindows::Server(Box::new(request.clone())),
                &mut platform
            )
            .await
            .is_err()
        );
        let state = read_state(Path::new(&request.client.progress_directory)).unwrap();
        assert_eq!(
            (state.progress.stage, state.progress.status),
            (failed, Status::Failed)
        );
        if failed != Stage::RegisterDesktop {
            assert!(!platform.calls.contains(&Stage::RegisterDesktop));
        }
    }
}

#[test]
fn interrupted_and_torn_records_preserve_the_last_durable_stage() {
    let root = TestRoot::new();
    let path = root.path("progress");
    let mut progress = Progress::create(&path).unwrap();
    progress.begin(Stage::InstallServices).unwrap();
    assert!(!read_state(Path::new(&path)).unwrap().interrupted);
    drop(progress);
    fs::write(Path::new(&path).join("00000002.json"), b"{\"version\":").unwrap();
    let state = read_state(Path::new(&path)).unwrap();
    assert!(state.interrupted);
    assert_eq!(state.progress.stage, Stage::InstallServices);
    assert!(Progress::create(&path).is_err());

    // Server readiness is an intermediate success; desktop registration can still be interrupted.
    let path = root.path("ready-before-desktop");
    let mut progress = Progress::create(&path).unwrap();
    progress
        .record(Stage::VerifyHttps, Status::Succeeded)
        .unwrap();
    drop(progress);
    assert!(read_state(Path::new(&path)).unwrap().interrupted);
    fs::write(Path::new(&path).join("00000002.json"), b"{").unwrap();
    let state = read_state(Path::new(&path)).unwrap();
    assert!(state.interrupted);
    assert_eq!(state.progress.stage, Stage::VerifyHttps);
}

#[test]
fn private_requests_reject_unknown_fields_and_debug_redacts_credentials() {
    let root = TestRoot::new();
    let request = InstallWindows::Server(Box::new(root.request()));
    let mut json = serde_json::to_value(&request).unwrap();
    json["payload"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<InstallWindows>(json).is_err());
    assert!(!format!("{request:?}").contains("Synthetic-password-123!"));
    let encoded = serde_json::to_vec(&request).unwrap();
    assert!(matches!(
        serde_json::from_slice::<InstallWindows>(&encoded).unwrap(),
        InstallWindows::Server(_)
    ));
}

#[test]
fn installation_validation_refuses_versions_existing_state_and_aliased_paths() {
    let root = TestRoot::new();
    let mut request = root.request();
    fs::create_dir(&request.server.directory).unwrap();
    fs::create_dir(&request.preparation.postgres_bin).unwrap();
    for file in [
        &request.preparation.platform_script,
        &request.preparation.certificate_executable,
        &request.winsw_executable,
    ] {
        fs::write(file, b"synthetic tool sentinel").unwrap();
    }
    validate_server(&request).unwrap();
    validate_client(&request.client).unwrap();
    request.version = 2;
    assert!(validate_server(&request).is_err());
    request.version = 1;
    request.client.version = 2;
    assert!(validate_client(&request.client).is_err());
    assert!(validate_server(&request).is_err());
    request.client.version = 1;
    request.service_state_directory = root.path("package/../host");
    assert!(validate_server(&request).is_err());
    request.service_state_directory = request.server.directory.clone();
    assert!(validate_server(&request).is_err());
    request.service_state_directory = root.path("host");
    fs::create_dir(&request.preparation.state_directory).unwrap();
    let sentinel = Path::new(&request.preparation.state_directory).join("retained");
    fs::write(&sentinel, b"durable workshop sentinel").unwrap();
    assert!(validate_server(&request).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"durable workshop sentinel");
    assert!(!Path::new(&request.service_state_directory).exists());
    assert!(!Path::new(&request.client.progress_directory).exists());
}
