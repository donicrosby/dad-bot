use crate::sea_orm::*;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error(transparent)]
    EpochBoundsCalc(#[from] chrono::RoundingError),
    #[error("Too many epochs returned from db")]
    TooManyEpochs,
    #[error("Epoch with id [{id}] doesn't exist")]
    EpochNotFound { id: u32 },
    #[error("Dadded with id [{id}] doesn't exist")]
    DaddedNotFound { id: u32 },
}
