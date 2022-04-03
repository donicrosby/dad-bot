use db::sea_orm::*;
use db::Error as DbError;
use matrix_sdk::ruma::events::AnyMessageEventContent;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<AnyMessageEventContent>),
    #[error(transparent)]
    DbError(#[from] DbError),
}

impl From<db::sea_orm::DbErr> for Error {
    fn from(item: DbErr) -> Self {
        Self::DbError(DbError::Db(item))
    }
}
