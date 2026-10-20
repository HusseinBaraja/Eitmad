//! Pairing v1 uses opaque-ke 3.0 (OPAQUE draft-16), Ristretto255/TripleDH/Argon2.
//! The complete public binding is authenticated by OPAQUE's context, never by discovery.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use eitmad_contracts::pairing::{
    CompletePairing, PairingBinding, PairingChallenge, PairingReceipt, StartPairing,
};
use hmac::{Hmac, Mac as _};
use opaque_ke::{
    CipherSuite, ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, ServerLogin, ServerLoginStartParameters, ServerRegistration, ServerSetup,
    rand::{RngCore as _, rngs::OsRng},
};
use sha2::Sha256;
use zeroize::Zeroizing;

struct Suite;
impl CipherSuite for Suite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::key_exchange::tripledh::TripleDh;
    type Ksf = argon2::Argon2<'static>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("pairing authentication failed")]
pub struct PairingError;

pub struct PairingRegistration {
    pub setup: Zeroizing<Vec<u8>>,
    pub password_file: Zeroizing<Vec<u8>>,
}

/// Generates an unbiased eight-digit code using the operating system RNG.
#[must_use]
pub fn generate_code() -> Zeroizing<String> {
    let mut rng = OsRng;
    let value = loop {
        let value = rng.next_u32();
        if value < u32::MAX - u32::MAX % 100_000_000 {
            break value % 100_000_000;
        }
    };
    Zeroizing::new(format!("{value:08}"))
}

/// Registers a code entirely on the Owner's server. No registration message crosses the network.
/// # Errors
/// Returns an error for invalid code input or a PAKE failure.
pub fn register(code: &str, session: &[u8]) -> Result<PairingRegistration, PairingError> {
    validate_code(code)?;
    let mut rng = OsRng;
    let setup = ServerSetup::<Suite>::new(&mut rng);
    let client =
        ClientRegistration::<Suite>::start(&mut rng, code.as_bytes()).map_err(|_| PairingError)?;
    let response =
        ServerRegistration::start(&setup, client.message, session).map_err(|_| PairingError)?;
    let upload = client
        .state
        .finish(
            &mut rng,
            code.as_bytes(),
            response.message,
            ClientRegistrationFinishParameters::default(),
        )
        .map_err(|_| PairingError)?;
    let file = ServerRegistration::<Suite>::finish(upload.message);
    Ok(PairingRegistration {
        setup: Zeroizing::new(setup.serialize().to_vec()),
        password_file: Zeroizing::new(file.serialize().to_vec()),
    })
}

pub struct PairingClient {
    state: ClientLogin<Suite>,
    binding: PairingBinding,
}

impl PairingClient {
    /// Starts an exchange without transmitting the code.
    /// # Errors
    /// Rejects invalid codes and bindings.
    pub fn start(
        code: &str,
        binding: PairingBinding,
    ) -> Result<(Self, StartPairing), PairingError> {
        validate_code(code)?;
        context(&binding)?;
        let start =
            ClientLogin::<Suite>::start(&mut OsRng, code.as_bytes()).map_err(|_| PairingError)?;
        let request = StartPairing {
            binding: binding.clone(),
            credential_request: B64.encode(start.message.serialize()),
        };
        Ok((
            Self {
                state: start.state,
                binding,
            },
            request,
        ))
    }

    /// Authenticates the complete server offer before any CA is trusted.
    /// # Errors
    /// Rejects incorrect codes, interception, or a changed identity/device binding.
    pub fn finish(
        self,
        code: &str,
        challenge: &PairingChallenge,
        key: &SigningKey,
    ) -> Result<(CompletePairing, Zeroizing<Vec<u8>>), PairingError> {
        if challenge.binding != self.binding || public_key(&self.binding)? != key.verifying_key() {
            return Err(PairingError);
        }
        let context = context(&self.binding)?;
        let response =
            CredentialResponse::<Suite>::deserialize(&decode(&challenge.credential_response)?)
                .map_err(|_| PairingError)?;
        let result = self
            .state
            .finish(
                code.as_bytes(),
                response,
                ClientLoginFinishParameters {
                    context: Some(&context),
                    ..Default::default()
                },
            )
            .map_err(|_| PairingError)?;
        let mut request = CompletePairing {
            binding: self.binding,
            credential_finalization: B64.encode(result.message.serialize()),
            device_signature: String::new(),
        };
        request.device_signature = B64.encode(key.sign(&completion_bytes(&request)?).to_bytes());
        Ok((request, Zeroizing::new(result.session_key.to_vec())))
    }
}

/// Generates one server challenge. The caller must durably charge an attempt before returning it.
/// # Errors
/// Rejects malformed PAKE input or stored registration.
pub fn challenge(
    registration: &PairingRegistration,
    request: &StartPairing,
) -> Result<(PairingChallenge, Zeroizing<Vec<u8>>), PairingError> {
    let context = context(&request.binding)?;
    let setup = ServerSetup::<Suite>::deserialize(&registration.setup).map_err(|_| PairingError)?;
    let file = ServerRegistration::<Suite>::deserialize(&registration.password_file)
        .map_err(|_| PairingError)?;
    let message = CredentialRequest::<Suite>::deserialize(&decode(&request.credential_request)?)
        .map_err(|_| PairingError)?;
    let result = ServerLogin::start(
        &mut OsRng,
        &setup,
        Some(file),
        message,
        request.binding.offer.session_id.as_bytes(),
        ServerLoginStartParameters {
            context: Some(&context),
            ..Default::default()
        },
    )
    .map_err(|_| PairingError)?;
    Ok((
        PairingChallenge {
            binding: request.binding.clone(),
            credential_response: B64.encode(result.message.serialize()),
        },
        Zeroizing::new(
            serde_json::to_vec(&(&request.binding, B64.encode(result.state.serialize())))
                .map_err(|_| PairingError)?,
        ),
    ))
}

/// Checks OPAQUE's client confirmation and possession of the bound device key.
/// # Errors
/// Rejects incorrect confirmation, changed context, and changed or unowned device keys.
pub fn complete(state: &[u8], request: &CompletePairing) -> Result<PairingReceipt, PairingError> {
    verify_device(request)?;
    let (binding, state): (PairingBinding, String) =
        serde_json::from_slice(state).map_err(|_| PairingError)?;
    let state = Zeroizing::new(state);
    if binding != request.binding {
        return Err(PairingError);
    }
    let state = ServerLogin::<Suite>::deserialize(&Zeroizing::new(decode(&state)?))
        .map_err(|_| PairingError)?;
    let finalization =
        CredentialFinalization::<Suite>::deserialize(&decode(&request.credential_finalization)?)
            .map_err(|_| PairingError)?;
    let result = state.finish(finalization).map_err(|_| PairingError)?;
    let confirmation = receipt_mac(&result.session_key, &request.binding)?
        .finalize()
        .into_bytes();
    Ok(PairingReceipt {
        binding: request.binding.clone(),
        confirmation: B64.encode(confirmation),
    })
}

/// Checks the durable-approval receipt before the client saves trust.
/// # Errors
/// Rejects forged receipts, reflection, and changed bindings.
pub fn verify_receipt(
    key: &[u8],
    binding: &PairingBinding,
    receipt: &PairingReceipt,
) -> Result<(), PairingError> {
    if binding != &receipt.binding {
        return Err(PairingError);
    }
    receipt_mac(key, binding)?
        .verify_slice(&decode(&receipt.confirmation)?)
        .map_err(|_| PairingError)
}

/// Verifies the device signature, including for an exact retry after interrupted completion.
/// # Errors
/// Rejects malformed keys or signatures.
pub fn verify_device(request: &CompletePairing) -> Result<(), PairingError> {
    let signature =
        Signature::from_slice(&decode(&request.device_signature)?).map_err(|_| PairingError)?;
    public_key(&request.binding)?
        .verify_strict(&completion_bytes(request)?, &signature)
        .map_err(|_| PairingError)
}

fn completion_bytes(request: &CompletePairing) -> Result<Vec<u8>, PairingError> {
    serde_json::to_vec(&(
        "eitmad.pairing.complete.v1",
        &request.binding,
        &request.credential_finalization,
    ))
    .map_err(|_| PairingError)
}
fn receipt_mac(key: &[u8], binding: &PairingBinding) -> Result<Hmac<Sha256>, PairingError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| PairingError)?;
    mac.update(b"eitmad.pairing.committed.v1\0");
    mac.update(&context(binding)?);
    Ok(mac)
}
fn public_key(binding: &PairingBinding) -> Result<VerifyingKey, PairingError> {
    if binding.device_public_key.algorithm != "ed25519" {
        return Err(PairingError);
    }
    let bytes: [u8; 32] = decode(&binding.device_public_key.base64)?
        .try_into()
        .map_err(|_| PairingError)?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| PairingError)?;
    if key.is_weak() {
        return Err(PairingError);
    }
    Ok(key)
}
fn context(binding: &PairingBinding) -> Result<Vec<u8>, PairingError> {
    if binding.offer.version != 1
        || binding.offer.identity.public_ca_pem.len() > 16_384
        || !binding.offer.identity.endpoint.starts_with("https://")
        || binding.offer.identity.endpoint.len() > 2048
        || binding.offer.identity.public_ca_pem.is_empty()
        || binding.device_id.value().is_nil()
        || binding.attempt_id.is_nil()
        || binding.offer.session_id.is_nil()
        || binding.offer.tenant_id.value().is_nil()
        || binding.offer.identity.server_id.value().is_nil()
        || binding.offer.organization_id.value().is_nil()
        || binding.offer.branch_id.value().is_nil()
    {
        return Err(PairingError);
    }
    public_key(binding)?;
    serde_json::to_vec(&(
        "eitmad.pairing.opaque3.ristretto255.tripledh.argon2.v1",
        binding,
    ))
    .map_err(|_| PairingError)
}
fn validate_code(code: &str) -> Result<(), PairingError> {
    if code.len() == 8 && code.bytes().all(|v| v.is_ascii_digit()) {
        Ok(())
    } else {
        Err(PairingError)
    }
}
fn decode(value: &str) -> Result<Vec<u8>, PairingError> {
    if value.len() > 4096 {
        return Err(PairingError);
    }
    B64.decode(value).map_err(|_| PairingError)
}

#[cfg(test)]
mod tests;
