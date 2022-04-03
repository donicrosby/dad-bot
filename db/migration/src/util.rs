use entity::sea_orm::{sea_query::table::TableCreateStatement, DbBackend, EntityTrait, Schema};

pub fn create_table_statement<E>(db: DbBackend, entity: E) -> TableCreateStatement
where
    E: EntityTrait,
{
    Schema::new(db)
        .create_table_from_entity(entity)
        .if_not_exists()
        .to_owned()
}
