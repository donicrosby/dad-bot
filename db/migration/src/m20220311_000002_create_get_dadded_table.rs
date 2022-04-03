use crate::util::create_table_statement;
use sea_schema::migration::{sea_query::*, *};

use entity::Dadded;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220311_000002_create_get_dadded_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_table_statement(
                manager.get_database_backend(),
                Dadded,
            ))
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Dadded).to_owned())
            .await
    }
}
