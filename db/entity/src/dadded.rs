use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "got_dadded")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub epoch_id: u32,
    pub count: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::epochs::Entity",
        from = "Column::EpochId",
        to = "super::epochs::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Epoch,
}

impl Related<super::epochs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Epoch.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
