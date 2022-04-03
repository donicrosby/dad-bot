#[cfg(test)]
mod utils {
    use crate::errors::Error;
    use db::migration::*;
    use db::sea_orm::*;

    pub async fn create_inmemory_db() -> Result<DbConn, Error> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;
        Ok(db)
    }
}

pub use self::utils::create_inmemory_db;
