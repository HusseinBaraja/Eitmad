//! Explicit, synthetic LAN dataset attached to an already enrolled installation.
use super::{Client, Result, checked, mutation, scope, sign_in};
use eitmad_contracts::{
    accounts::{AccountPassword, CreateDesktopAccount, DesktopAccountRole},
    commands::Command,
    ipc::DesktopSignInRequest,
    transport::{RequestId, SchemaId},
};
use eitmad_engine_runtime::DesktopAccountAuthority as _;
use eitmad_pricing::CatalogReplication as _;
use eitmad_secret_storage::SecretStore;
use eitmad_server_connection::{
    DesktopConnectionSettings, DesktopEnrollment, DirectAccountClient, DirectCatalogSyncClient,
    DirectCustomerSyncClient, DirectDiscountApprovalClient, DirectPriceClient, DirectServerConfig,
};
use eitmad_storage::AuthorityStore;
use serde::Deserialize;
use std::{
    io::{Read as _, Write as _},
    path::PathBuf,
    sync::Arc,
};
use uuid::Uuid;
use zeroize::Zeroize as _;

#[path = "hadramout/catalog.rs"]
mod catalog;
#[path = "hadramout/sales.rs"]
mod sales;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    directory: PathBuf,
    expected_server_id: eitmad_contracts::discovery::ServerInstallationId,
    owner_username: String,
    owner_password: String,
    receptionist_password: String,
    acknowledge_synthetic_data: bool,
}

impl Drop for Request {
    fn drop(&mut self) {
        self.owner_password.zeroize();
        self.receptionist_password.zeroize();
    }
}

pub(super) fn run() -> Result<()> {
    let mut input = zeroize::Zeroizing::new(Vec::new());
    checked(
        std::io::stdin().take(16_385).read_to_end(&mut input),
        "demo input",
    )?;
    if input.len() > 16_384 {
        return Err("demo input limit");
    }
    let mut request: Request = checked(serde_json::from_slice(&input), "demo request")?;
    if !request.acknowledge_synthetic_data {
        return Err("synthetic data consent");
    }
    let settings = checked(
        DesktopConnectionSettings::load(&request.directory),
        "enrolled connection",
    )?
    .ok_or("enrolled connection required")?;
    if settings.server_id != Some(request.expected_server_id) {
        return Err("demo server identity mismatch");
    }
    let manager = connected(&request, &request.owner_username, &request.owner_password)?;
    receptionist(&request, &settings, &manager)?;
    let reception = connected(&request, "receptionist", &request.receptionist_password)?;
    request.owner_password.zeroize();
    request.receptionist_password.zeroize();
    admit(&request, &manager, &reception)?;
    let targets = catalog::run(&manager)?;
    checked(
        reception
            .catalog
            .synchronize(&reception.actor, super::deadline()),
        "demo catalog download",
    )?;
    sales::run(&manager, &reception, &targets)?;
    super::seed::verify(&manager, &reception)?;
    sales::verify(&manager, &reception)?;
    println!(
        "Synthetic Hadramout workshop synchronized: 96 customers, 24 materials, 16 parts, 16 furniture, 12 products, 72 quotations."
    );
    Ok(())
}

fn admit(request: &Request, manager: &Client, reception: &Client) -> Result<()> {
    use eitmad_contracts::{pricing::ListPrices, quotation_lifecycle::ListQuotations};
    use eitmad_pricing::{PricingService, QuotationServer as _};
    let marker = request.directory.join("hadramout-demo-v1");
    let identity = request.expected_server_id.value().to_string();
    if marker.exists() {
        return if checked(std::fs::read_to_string(marker), "demo marker")? == identity {
            Ok(())
        } else {
            Err("demo marker identity mismatch")
        };
    }
    checked(
        manager
            .catalog
            .synchronize(&manager.actor, super::deadline()),
        "demo existing catalog check",
    )?;
    let prices = checked(
        PricingService::new(
            manager.store.clone(),
            eitmad_authorization::AuthorizationService::new(manager.store.clone()),
        )
        .list(
            &manager.actor,
            &ListPrices {
                term: String::new(),
                after: None,
                limit: 1,
            },
        ),
        "demo empty catalog check",
    )?;
    let quotes = checked(
        reception.workflow.quotations(
            &reception.branch,
            &ListQuotations {
                after: None,
                limit: 1,
            },
            super::deadline(),
        ),
        "demo empty workflow check",
    )?;
    if !prices.items.is_empty()
        || !quotes.items.is_empty()
        || sales::customer_count(reception)? != 0
    {
        return Err("demo requires an empty test workshop");
    }
    let mut file = checked(
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(marker),
        "demo admission marker",
    )?;
    checked(file.write_all(identity.as_bytes()), "demo admission marker")?;
    checked(file.sync_all(), "demo admission marker")
}

fn connected(request: &Request, username: &str, password: &str) -> Result<Client> {
    let directory = &request.directory;
    let store = checked(AuthorityStore::open(directory), "demo authority store")?;
    let secrets = checked(SecretStore::open(directory, None), "demo secret storage")?;
    checked(
        DesktopEnrollment::new(directory.clone(), store.clone(), secrets.clone()).sign_in(
            &DesktopSignInRequest {
                request_id: RequestId::new(Uuid::new_v4()),
                correlation_id: super::correlation(),
                username: username.into(),
                password: password.into(),
                server_login: true,
                connection: None,
            },
        ),
        "demo online sign in",
    )?;
    let installer = checked(
        store.existing_local_authorization_context(),
        "demo installation",
    )?;
    let (actor, branch) = sign_in(&store, installer, username, password)?;
    let settings = checked(DesktopConnectionSettings::load(directory), "demo settings")?
        .ok_or("demo settings")?;
    let config = configuration(&settings, false)?;
    Ok(Client {
        store: store.clone(),
        actor,
        branch,
        catalog: Arc::new(DirectCatalogSyncClient::from_config(
            config.clone(),
            secrets.clone(),
            settings.credential.clone(),
            store.clone(),
        )),
        prices: Arc::new(DirectPriceClient::from_config(
            config.clone(),
            secrets.clone(),
            settings.credential.clone(),
        )),
        workflow: DirectDiscountApprovalClient::from_config(
            config,
            secrets.clone(),
            settings.credential.clone(),
            scope("branch", settings.branch.value())?,
        ),
        customers: DirectCustomerSyncClient::from_config(
            configuration(&settings, true)?,
            secrets,
            settings.credential,
            store,
        ),
    })
}

fn configuration(settings: &DesktopConnectionSettings, branch: bool) -> Result<DirectServerConfig> {
    checked(
        DirectServerConfig::new(
            &settings.endpoint,
            if branch {
                scope("branch", settings.branch.value())?
            } else {
                scope("organization", settings.organization.value())?
            },
            checked(
                SchemaId::parse(if branch {
                    "eitmad.schema.customer.v1"
                } else {
                    "eitmad.schema.pricing.v1"
                }),
                "demo schema",
            )?,
            1,
            &settings.trusted_certificate,
        ),
        "demo verified TLS route",
    )
}

fn receptionist(
    request: &Request,
    settings: &DesktopConnectionSettings,
    manager: &Client,
) -> Result<()> {
    let client = DirectAccountClient::from_config(
        configuration(settings, false)?,
        checked(
            SecretStore::open(&request.directory, None),
            "demo account secrets",
        )?,
        settings.credential.clone(),
        settings.organization,
        settings.branch,
    );
    let accounts = checked(client.list(&manager.actor), "demo account list")?;
    if accounts
        .accounts
        .iter()
        .any(|a| a.username == "receptionist")
    {
        return Ok(());
    }
    checked(
        client.change(
            &mutation(&manager.actor, 0x4844_0001),
            &Command::CreateDesktopAccount(CreateDesktopAccount {
                display_name: "سالم باوزير ـ استقبال المعرض".into(),
                username: "receptionist".into(),
                password: AccountPassword::new(request.receptionist_password.clone()),
                role: DesktopAccountRole::Receptionist,
            }),
        ),
        "demo receptionist creation",
    )?;
    Ok(())
}
