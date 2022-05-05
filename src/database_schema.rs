use sea_schema::sqlite::{DiscoveryResult, Schema, SchemaDiscovery};
use sqlx::{Pool, Sqlite};

pub async fn get_database_schema(connection: Pool<Sqlite>) -> DiscoveryResult<Schema> {
    let schema_discovery = SchemaDiscovery::new(connection);

    let schema: Schema = schema_discovery.discover().await?;

    Ok(schema)
}
