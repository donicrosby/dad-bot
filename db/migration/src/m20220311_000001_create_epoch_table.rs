use super::util::create_table_statement;
use sea_schema::migration::{sea_query::*, *};

use entity::Epoch;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220311_000001_create_epoch_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_table_statement(
                manager.get_database_backend(),
                Epoch,
            ))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Epoch).to_owned())
            .await
    }
}
