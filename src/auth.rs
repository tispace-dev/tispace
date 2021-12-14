use axum::{
    async_trait,
    extract::{FromRequest, RequestParts, TypedHeader},
};
use google_signin;
use google_signin::{CachedCerts, Client};
use headers::{authorization::Bearer, Authorization};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::error::AuthError;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    let mut client = Client::new();
    client.audiences.push(std::env::var("CLIENT_ID").unwrap());

    client
});

static CACHEDCERTS: Lazy<CachedCerts> = Lazy::new(|| {
    let mut certs = CachedCerts::new();
    Runtime::new().unwrap().block_on(async {
        certs.refresh_if_needed().await.unwrap();
    });

    certs
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct UserClaims {
    crate sub: String,
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

        let id_info = CLIENT
            .verify(bearer.token(), &CACHEDCERTS)
            .await
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(UserClaims { sub: id_info.sub })
    }
}
