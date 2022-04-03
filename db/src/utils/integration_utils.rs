use crate::errors::Error;
use crate::migration::*;
use crate::sea_orm::*;

pub async fn create_inmemory_db() -> Result<DbConn, Error> {
    let db = Database::connect("sqlite::memory:").await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}
