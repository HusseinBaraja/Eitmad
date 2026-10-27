use super::*;
use eitmad_contracts::{
    accounts::AccountPassword,
    setup::{InitializeWorkshop, PrepareWindowsWorkshop},
};
mod enrollment;

#[test]
fn native_failure_details_survive_reopening_the_journal() {
    use eitmad_contracts::setup::WindowsSetupFailure;
    let root = TestRoot::new();
    let request = InstallWindows::Client(root.request().client);
    let journal = root.path("diagnostic-progress");
    let mut progress = Progress::open(&journal, &request).unwrap();
    progress.begin(Stage::Preflight).unwrap();
    let details = "Synthetic native check: access denied. ".repeat(80);
    assert!(
        progress
            .finish(Err(MainError::SetupDiagnostic(
                Some(WindowsSetupFailure::DesktopUser),
                details.clone()
            )))
            .is_err()
    );
    drop(progress);
    let state = read_state(Path::new(&journal)).unwrap();
    assert_eq!(state.progress.status, Status::Failed);
    assert_eq!(state.progress.stage, Stage::Preflight);
    assert_eq!(
        state.progress.failure,
        Some(WindowsSetupFailure::DesktopUser)
    );
    assert_eq!(state.progress.details, Some(details));
    assert!(!state.interrupted);
}

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
        }
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn elevated_bundle_rebinds_only_authenticated_tools_and_release_pins() {
    let root = TestRoot::new();
    let original = root.0.join("source");
    let protected = root.0.join("protected");
    fs::create_dir(&protected).unwrap();
    let original_path = |name: &str| original.join(name).to_string_lossy().into_owned();
    let mut request = root.request();
    request.server.directory = original_path("server/application");
    request.client.desktop.directory = original_path("client/desktop");
    request.preparation.postgres_bin = original_path("server/postgresql/bin");
    request.preparation.platform_script =
        original_path("server/application/deploy/windows/prepare-workshop.ps1");
    request.preparation.certificate_executable = original_path(
        "server/application/tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe",
    );
    request.winsw_executable = original_path("server/winsw/WinSW.NET461.exe");
    let mut metadata = eitmad_contracts::setup::WindowsSetupBundle {
        version: 2,
        role: eitmad_contracts::setup::WindowsSetupRole::Server,
        publisher: "CN=Synthetic".into(),
        desktop_release_sha256: "a".repeat(64),
        server_release_sha256: "a".repeat(64),
        winsw_sha256: "b".repeat(64),
    };
    fs::write(
        protected.join("setup-bundle.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let mut changed = request.clone();
    changed.preparation.postgres_bin = root.path("unreviewed/bin");
    assert!(super::super::payload::bind_server(&mut changed, &protected).is_err());
    changed = request.clone();
    changed.server.release_sha256 = "c".repeat(64);
    assert!(super::super::payload::bind_server(&mut changed, &protected).is_err());
    let mut windows_paths = request.clone();
    for path in [
        &mut windows_paths.server.directory,
        &mut windows_paths.client.desktop.directory,
        &mut windows_paths.preparation.postgres_bin,
        &mut windows_paths.preparation.platform_script,
        &mut windows_paths.preparation.certificate_executable,
        &mut windows_paths.winsw_executable,
    ] {
        *path = path.replace('/', "\\");
    }
    super::super::payload::bind_server(&mut windows_paths, &protected).unwrap();
    super::super::payload::bind_server(&mut request, &protected).unwrap();
    assert_eq!(
        Path::new(&request.server.directory),
        protected.join("server/application")
    );
    assert_eq!(
        Path::new(&request.preparation.postgres_bin),
        protected.join("server/postgresql/bin")
    );
    assert_eq!(
        request.client.intended_user_sid,
        "S-1-5-21-123-456-789-1001"
    );
    assert_eq!(request.service_state_directory, root.path("host"));
    metadata.role = eitmad_contracts::setup::WindowsSetupRole::Client;
    fs::write(
        protected.join("setup-bundle.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    assert!(super::super::payload::bind_server(&mut request, &protected).is_err());
}

#[derive(Default)]
struct FakePlatform {
    calls: Vec<Stage>,
    failure: Option<Stage>,
    ready: std::rc::Rc<std::cell::Cell<bool>>,
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
        request: &InstallWindowsServer,
        progress: &mut Progress,
    ) -> Result<(), MainError> {
        let root = Path::new(&request.preparation.state_directory);
        if root.exists() {
            return if root.join("prepared-workshop.json").exists() {
                Ok(())
            } else {
                Err(MainError::Setup)
            };
        }
        fs::create_dir(root).unwrap();
        for stage in [
            Stage::PrepareWorkshop,
            Stage::ApplyMigrations,
            Stage::InitializeOwner,
            Stage::StopPreparedDatabase,
        ] {
            progress.begin(stage)?;
            self.call(stage)?;
        }
        fs::write(root.join("prepared-workshop.json"), b"synthetic completion").unwrap();
        Ok(())
    }
    fn host(&mut self, request: &InstallWindowsServer, action: &str) -> Result<(), MainError> {
        if action == "Install" {
            fs::create_dir(&request.service_state_directory).unwrap();
        }
        self.call(match action {
            "Install" | "ResumeInstall" => Stage::InstallServices,
            "Firewall" => Stage::ConfigureFirewall,
            "Start" => Stage::VerifyHttps,
            _ => panic!("Unexpected host action"),
        })?;
        if action == "Start" {
            self.ready.set(true);
        }
        Ok(())
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
    install_server(&request, &mut platform, &mut std::io::empty())
        .await
        .unwrap();
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
async fn each_host_failure_retains_the_stage_and_never_reinitializes_the_owner() {
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
        assert!(
            install_server(&request, &mut platform, &mut std::io::empty())
                .await
                .is_err()
        );
        assert_eq!(platform.calls.last(), Some(&stage));
        let state = read_state(Path::new(&request.server_progress_directory)).unwrap();
        assert_eq!(
            (state.progress.stage, state.progress.status),
            (stage, Status::Failed)
        );
        assert!(!state.interrupted);
        let count = platform.calls.len();
        let previous = fs::read(
            Path::new(&request.server_progress_directory)
                .join(format!("{:08}.json", state.progress.sequence)),
        )
        .unwrap();
        assert!(
            install_server(&request, &mut platform, &mut std::io::empty())
                .await
                .is_err()
        );
        assert!(platform.calls.len() > count);
        assert!(!platform.calls[count..].contains(&Stage::InitializeOwner));
        assert_eq!(
            fs::read(
                Path::new(&request.server_progress_directory)
                    .join(format!("{:08}.json", state.progress.sequence))
            )
            .unwrap(),
            previous
        );
        if matches!(
            stage,
            Stage::InstallServices | Stage::ConfigureFirewall | Stage::VerifyHttps
        ) {
            platform.failure = None;
            install_server(&request, &mut platform, &mut std::io::empty())
                .await
                .unwrap();
            assert_eq!(
                platform
                    .calls
                    .iter()
                    .filter(|s| **s == Stage::InitializeOwner)
                    .count(),
                1
            );
            assert_eq!(
                read_state(Path::new(&request.server_progress_directory))
                    .unwrap()
                    .progress
                    .stage,
                Stage::Complete
            );
        }
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

#[tokio::test]
async fn cancelled_uac_and_registration_failure_resume_with_original_inputs() {
    for failed in [Stage::Elevation, Stage::RegisterDesktop] {
        let root = TestRoot::new();
        let request = InstallWindows::Server(Box::new(root.request()));
        let mut platform = FakePlatform {
            failure: Some(failed),
            ..Default::default()
        };
        assert!(install(&request, &mut platform).await.is_err());
        let saved = recovery::load(&root.0.join("client-progress")).unwrap();
        let json = serde_json::to_string(&saved).unwrap();
        assert!(!json.contains("Synthetic-password-123!"));
        assert_eq!(
            recovery::binding(&saved).unwrap(),
            recovery::binding(&request).unwrap()
        );
        platform.failure = None;
        install(&request, &mut platform).await.unwrap();
        assert_eq!(
            read_state(&root.0.join("client-progress"))
                .unwrap()
                .progress
                .stage,
            Stage::Complete
        );
    }
}

#[test]
fn retry_binds_user_release_and_device_and_preserves_torn_records() {
    let root = TestRoot::new();
    let request = InstallWindows::Server(Box::new(root.request()));
    let path = root.path("client-progress");
    let mut progress = Progress::open(&path, &request).unwrap();
    progress.begin(Stage::Elevation).unwrap();
    assert!(Progress::open(&path, &request).is_err());
    drop(progress);
    let torn = Path::new(&path).join("00000002.json");
    fs::write(&torn, b"{").unwrap();
    for change in [0, 1] {
        let mut changed = request.clone();
        let InstallWindows::Server(server) = &mut changed else {
            unreachable!()
        };
        if change == 0 {
            server.client.intended_user_sid.push('1');
        } else {
            server.server.release_sha256 = "c".repeat(64);
        }
        assert!(Progress::open(&path, &changed).is_err());
    }
    let mut resumed = Progress::open(&path, &request).unwrap();
    resumed.begin(Stage::Preflight).unwrap();
    assert_eq!(fs::read(torn).unwrap(), b"{");
    assert!(Path::new(&path).join("00000003.json").exists());
    let ca = Path::new(&path).join("ca.pem");
    recovery::retain_exact(&ca, b"original public CA").unwrap();
    recovery::retain_exact(&ca, b"original public CA").unwrap();
    assert!(recovery::retain_exact(&ca, b"replacement CA").is_err());
    assert_eq!(fs::read(ca).unwrap(), b"original public CA");
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

#[tokio::test]
async fn installation_validation_refuses_versions_uncertain_state_and_aliased_paths() {
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
    validate_server(&request).unwrap();
    assert!(
        install_server(
            &request,
            &mut FakePlatform::default(),
            &mut std::io::empty()
        )
        .await
        .is_err()
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"durable workshop sentinel");
    assert!(!Path::new(&request.service_state_directory).exists());
    assert!(!Path::new(&request.client.progress_directory).exists());
}
