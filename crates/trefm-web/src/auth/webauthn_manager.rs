use std::collections::HashMap;
use std::sync::Mutex;

use url::Url;
use webauthn_rs::prelude::*;
use webauthn_rs::Webauthn;

pub struct WebAuthnManager {
    webauthn: Webauthn,
    credentials: Mutex<HashMap<String, Vec<Passkey>>>,
    user_id: Uuid,
}

impl WebAuthnManager {
    pub fn new(rp_id: &str, rp_origin: &str) -> Result<Self, anyhow::Error> {
        let rp_origin = Url::parse(rp_origin)?;
        let builder = WebauthnBuilder::new(rp_id, &rp_origin)?;
        let webauthn = builder.build()?;

        Ok(Self {
            webauthn,
            credentials: Mutex::new(HashMap::new()),
            user_id: Uuid::new_v4(),
        })
    }

    pub fn has_credentials_for(&self, username: &str) -> bool {
        let creds = self.credentials.lock().unwrap();
        creds.get(username).is_some_and(|v| !v.is_empty())
    }

    pub fn start_registration(&self, username: &str) -> Result<(CreationChallengeResponse, PasskeyRegistration), anyhow::Error> {
        let creds = self.credentials.lock().unwrap();
        let exclude = creds
            .get(username)
            .map(|v| v.iter().map(|c| c.cred_id().clone()).collect::<Vec<_>>())
            .unwrap_or_default();
        drop(creds);

        let (challenge, reg_state) = self.webauthn.start_passkey_registration(
            self.user_id,
            username,
            username,
            Some(exclude),
        )?;

        Ok((challenge, reg_state))
    }

    pub fn finish_registration(
        &self,
        username: &str,
        response: &RegisterPublicKeyCredential,
        state: &PasskeyRegistration,
    ) -> Result<(), anyhow::Error> {
        let passkey = self.webauthn.finish_passkey_registration(response, state)?;
        let mut creds = self.credentials.lock().unwrap();
        creds.entry(username.to_string()).or_default().push(passkey);
        Ok(())
    }

    pub fn start_authentication(&self, username: &str) -> Result<(RequestChallengeResponse, PasskeyAuthentication), anyhow::Error> {
        let creds = self.credentials.lock().unwrap();
        let user_creds = creds.get(username).ok_or_else(|| {
            anyhow::anyhow!("No passkeys registered for user '{}'", username)
        })?;
        if user_creds.is_empty() {
            return Err(anyhow::anyhow!("No passkeys registered for user '{}'", username));
        }
        let (challenge, auth_state) = self.webauthn.start_passkey_authentication(user_creds)?;
        Ok((challenge, auth_state))
    }

    pub fn finish_authentication(
        &self,
        username: &str,
        response: &PublicKeyCredential,
        state: &PasskeyAuthentication,
    ) -> Result<(), anyhow::Error> {
        let auth_result = self.webauthn.finish_passkey_authentication(response, state)?;
        let mut creds = self.credentials.lock().unwrap();
        if let Some(user_creds) = creds.get_mut(username) {
            for cred in user_creds.iter_mut() {
                cred.update_credential(&auth_result);
            }
        }
        Ok(())
    }
}
