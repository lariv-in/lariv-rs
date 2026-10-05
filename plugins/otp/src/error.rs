use thiserror::Error;

#[derive(Debug, Error)]
pub enum OtpError {
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),

    #[error("{0}")]
    Message(String),

    #[error("OTP send failed")]
    SendFailed,

    #[error("not found")]
    NotFound,
}
