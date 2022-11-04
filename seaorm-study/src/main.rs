use anyhow::Result;
use sea_orm::{entity::prelude::*, Database};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cake")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");
    let db: DatabaseConnection = Database::connect("mysql://root:root@localhost/test").await?;
    let a = Model { id: 1, name: "1".to_owned() };
    let s = Entity::<Model>::insert(a);
    debug_query!();
    //Entity::insert(a)
    Ok(())
}
