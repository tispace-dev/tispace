use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
};
use google_signin;
use google_signin::{CachedCerts, Client};
use headers::{authorization::Bearer, Authorization};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;

use crate::env::GOOGLE_CLIENT_ID;
use crate::error::AuthError;
use crate::storage::Storage;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    let mut client = Client::new();
    client.audiences.push(GOOGLE_CLIENT_ID.clone());
    client
});

static CACHEDCERTS: Lazy<RwLock<CachedCerts>> = Lazy::new(|| RwLock::new(CachedCerts::new()));

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserClaims {
    crate username: String,
    crate email: String,
}

#[async_trait]
impl<B> FromRequest<B> for UserClaims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(req)
                .await
                .map_err(|_| AuthError::InvalidToken)?;

        let mut certs = CACHEDCERTS.read().await.clone();
        match certs.refresh_if_needed().await {
            Ok(true) => {
                *CACHEDCERTS.write().await = certs.clone();
            }
            Ok(false) => {}
            Err(e) => {
                warn!("refresh certs err {:?}", e);
                return Err(AuthError::InvalidToken);
            }
        }

        let id_info = CLIENT.verify(bearer.token(), &certs).await.map_err(|e| {
            warn!("verify token err {:?}", e);
            AuthError::InvalidToken
        })?;
        let email = id_info.email.ok_or(AuthError::InvalidToken)?;
        let username = email
            .replace(
                format!("@{}", id_info.hd.ok_or(AuthError::InvalidToken)?).as_str(),
                "",
            )
            // Ignore the `. `
            .replace('.', "");

        let Extension(storage) = Extension::<Storage>::from_request(req)
            .await
            .expect("`Storage` extension is missing");

        let mut found = false;
        storage
            .read_only(|state| found = state.find_user(&username).is_some())
            .await;
        if found {
            Ok(UserClaims { username, email })
        } else {
            warn!("unauthorized user {}", username);
            Err(AuthError::UnauthorizedUser)
        }
    }
}
