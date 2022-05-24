use crate::types::{ColumnMeta, TableMeta};
use heck::{ToSnakeCase, ToUpperCamelCase};
use sea_query::{ColumnDef, ColumnSpec, TableCreateStatement};
use sea_schema::sqlite::def::{Schema, TableDef};
use sea_schema::sqlite::discovery::SchemaDiscovery;
use sea_orm_codegen::Column;
use sqlx::{Pool, Sqlite};

pub async fn get_database_schema(
    connection: Pool<Sqlite>,
) -> (Vec<TableMeta>, Vec<TableCreateStatement>) {
    let schema_discovery = SchemaDiscovery::new(connection);

    let schema: Schema = schema_discovery.discover().await.unwrap();

    let tables_meta = schema
        .tables
        .iter()
        .map(|table: &TableDef| {
            let table_create_stmt = table.write();

            let columns: Vec<ColumnMeta> = table_create_stmt
                .get_columns()
                .into_iter()
                .map(|column: &ColumnDef| {
                    let column_name = column.get_column_name();
                    let column_spec: &Vec<ColumnSpec> = column.get_column_spec();
                    let column_info: Column = Column::from(column);

                    let not_null = column_spec
                        .iter()
                        .any(|spec| matches!(spec, ColumnSpec::NotNull));
                    let is_primary_key = column_spec
                        .iter()
                        .any(|spec| matches!(spec, ColumnSpec::PrimaryKey));

                    ColumnMeta {
                        column_name: column_name.to_snake_case(),
                        column_enum_name: column_name.to_upper_camel_case(),
                        not_null,
                        column_type: column_info.get_rs_type(),
                        column_filter_type: column_info.get_rs_type()
                            .into_iter()
                            .find(|token: &proc_macro2::TokenTree| {
                                if let proc_macro2::TokenTree::Ident(ident) = token {
                                    if ident.eq("Option") {
                                        false
                                    } else {
                                        true
                                    }
                                } else {
                                    false
                                }
                            })
                            .unwrap()
                            .into(),
                        is_primary_key,
                        is_foreign_key: false, // TODO
                    }
                })
                .collect();

            TableMeta {
                entity_name: table.name.to_upper_camel_case(),
                entity_filter: format!("{}Filter", table.name.to_upper_camel_case()),
                entity_module: table.name.to_snake_case(),
                columns,
                foreign_keys: vec![], // TODO
            }
        })
        .collect();

    let tables_create_stmts: Vec<TableCreateStatement> =
        schema.tables.iter().map(|table| table.write()).collect();

    (tables_meta, tables_create_stmts)
}
