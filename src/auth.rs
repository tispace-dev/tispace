use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
};
use google_signin;
use google_signin::{CachedCerts, Client};
use headers::{authorization::Bearer, Authorization};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing::warn;

use crate::error::AuthError;
use crate::storage::Storage;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    let mut client = Client::new();
    client
        .audiences
        .push(std::env::var("GOOGLE_CLIENT_ID").unwrap());

    client
});

static CACHEDCERTS: OnceCell<CachedCerts> = OnceCell::const_new();

pub async fn authorized(
    user: UserClaims,
    Extension(storage): Extension<Storage>,
) -> Result<(), AuthError> {
    let mut found = false;
    storage
        .read_only(|state| found = state.users.iter().any(|u| u.username == user.username))
        .await;
    if found {
        Ok(())
    } else {
        warn!("unauthorized user {}", user.username);
        Err(AuthError::UnauthorizedUser)
    }
}

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

        let certs = CACHEDCERTS
            .get_or_init(|| async {
                let mut certs = CachedCerts::new();
                certs.refresh_if_needed().await.unwrap();

                certs
            })
            .await;
        let id_info = CLIENT.verify(bearer.token(), certs).await.map_err(|e| {
            warn!("verify token err {:?}", e);
            AuthError::InvalidToken
        })?;
        let email = id_info.email.ok_or(AuthError::InvalidToken)?;
        let username = email.replace(
            format!("@{}", id_info.hd.ok_or(AuthError::InvalidToken)?).as_str(),
            "",
        );

        Ok(UserClaims { username, email })
    }
}
