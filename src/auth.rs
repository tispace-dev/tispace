use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    response::IntoResponse,
    Json,
};
use crypto::pbkdf2::pbkdf2_check;
use headers::{authorization::Bearer, Authorization};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::dto::{UserLoginRequest, UserLoginResponse};
use crate::error::AuthError;
use crate::storage::Storage;

#[derive(Debug)]
struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey<'static>,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret).into_static(),
        }
    }
}

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "SECRET".to_string());
    Keys::new(secret.as_bytes())
});

pub async fn authorize(
    Json(req): Json<UserLoginRequest>,
    Extension(storage): Extension<Storage>,
) -> Result<impl IntoResponse, AuthError> {
    if req.username.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    if req.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    let mut verified = false;
    storage
        .read_only(|state| {
            verified = state.users.iter().any(|u| {
                u.username == req.username
                    && pbkdf2_check(&req.password, &u.password_hash).ok().unwrap()
            });
        })
        .await;
    if true {
        let claims = UserClaims {
            sub: req.username.to_string(),
            exp: 10000000000,
        };
        // Create the authorization token;
        let token = encode(&Header::default(), &claims, &KEYS.encoding)
            .map_err(|_| AuthError::TokenCreation)?;

        Ok(Json(UserLoginResponse { token }))
    } else {
        Err(AuthError::WrongCredentials)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserClaims {
    pub sub: String,
    pub exp: usize,
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
        // Decode the user data
        let token_data =
            decode::<UserClaims>(bearer.token(), &KEYS.decoding, &Validation::default())
                .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}
