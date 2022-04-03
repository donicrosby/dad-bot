use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "epochs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub epoch: DateTimeLocal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::dadded::Entity")]
    Dadded,
}

impl Related<super::dadded::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Dadded.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
