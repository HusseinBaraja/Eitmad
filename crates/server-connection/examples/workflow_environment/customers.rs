use super::{Client, Result, checked};

pub(super) fn synchronize(client: &Client) -> Result<()> {
    checked(
        client.customers.synchronize(&client.branch),
        "customer TLS synchronization",
    )
    .map(|_| ())
}
