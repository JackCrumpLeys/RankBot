pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20231013_004433_snowflake_primary;
mod m20231015_012152_float_score;
mod m20231016_192446_time;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20231013_004433_snowflake_primary::Migration),
            Box::new(m20231015_012152_float_score::Migration),
            Box::new(m20231016_192446_time::Migration),
        ]
    }
}
