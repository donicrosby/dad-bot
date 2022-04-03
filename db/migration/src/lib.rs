pub use sea_schema::migration::*;

mod m20220311_000001_create_epoch_table;
mod m20220311_000002_create_get_dadded_table;
mod util;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220311_000001_create_epoch_table::Migration),
            Box::new(m20220311_000002_create_get_dadded_table::Migration),
        ]
    }
}
