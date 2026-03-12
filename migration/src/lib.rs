pub use sea_orm_migration::prelude::*;

mod m20260312_211347_users;
mod m20260312_211355_rooms;
mod m20260312_211431_images;
mod m20260312_211454_image_votes;
mod m20260312_211519_comments;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260312_211347_users::Migration),
            Box::new(m20260312_211355_rooms::Migration),
            Box::new(m20260312_211431_images::Migration),
            Box::new(m20260312_211454_image_votes::Migration),
            Box::new(m20260312_211519_comments::Migration),
        ]
    }
}
