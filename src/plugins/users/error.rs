use thiserror::Error;

#[derive(Debug, Error)]
pub enum UsersError {
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("authentication failed")]
    AuthFailed,
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl UsersError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}
