use crate::types::{ColumnMeta, ForeignKeyMeta, TableMeta};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use sea_orm_codegen::Column;
use sea_query::{
    ColumnDef, ColumnSpec, ForeignKeyCreateStatement, TableCreateStatement, TableForeignKey,
};
use sea_schema::sqlite::def::{Schema, TableDef};
use sea_schema::sqlite::discovery::SchemaDiscovery;
use sqlx::{Pool, Sqlite};

pub async fn get_database_schema(
    connection: Pool<Sqlite>,
) -> (Vec<TableMeta>, Vec<TableCreateStatement>) {
    let schema_discovery = SchemaDiscovery::new(connection);

    let schema: Schema = schema_discovery.discover().await.unwrap();

    let foreign_keys: Vec<ForeignKeyMeta> = schema
        .tables
        .clone()
        .iter()
        .map(|table: &TableDef| {
            let table_create_stmt = table.write();

            let foreign_keys: Vec<ForeignKeyMeta> = table_create_stmt
                .get_foreign_key_create_stmts()
                .iter()
                .map(|fk: &ForeignKeyCreateStatement| fk.get_foreign_key())
                .map(|fk: &TableForeignKey| parse_table_fk(&table, &table_create_stmt, fk))
                .collect();

            foreign_keys
        })
        .fold(
            Vec::<ForeignKeyMeta>::new(),
            |acc: Vec<ForeignKeyMeta>, cur: Vec<ForeignKeyMeta>| {
                [acc, cur].concat()
            },
        );

    let tables_meta = schema
        .tables
        .iter()
        .map(|table: &TableDef| {
            let table_create_stmt = table.write();

            let entity_name = table.name.to_upper_camel_case();

            let foreign_keys: Vec<ForeignKeyMeta> = foreign_keys
                .clone()
                .into_iter()
                .filter(|fk: &ForeignKeyMeta| fk.destination_table_name.eq(&entity_name) || fk.source_table_name.eq(&entity_name))
                .collect();

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
                        column_filter_type: column_info
                            .get_rs_type() // TODO common function
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
                entity_name,
                entity_module: table.name.to_snake_case(),
                columns,
                foreign_keys,
            }
        })
        .collect();

    let tables_create_stmts: Vec<TableCreateStatement> =
        schema.tables.iter().map(|table| table.write()).collect();

    (tables_meta, tables_create_stmts)
}

fn parse_table_fk(
    table: &TableDef,
    table_create_stmt: &sea_query::TableCreateStatement,
    fk: &TableForeignKey,
) -> ForeignKeyMeta {
    let source_table_name = table.name.clone().to_upper_camel_case();
    let source_table_module = source_table_name.to_snake_case();

    let destination_table_name = fk.get_ref_table().unwrap().to_upper_camel_case();
    let destination_table_module = destination_table_name.to_snake_case();

    let column_types: Vec<_> = fk
        .get_columns()
        .iter()
        .map(|name| {
            Column::from(
                table_create_stmt
                    .get_columns()
                    .into_iter()
                    .find(|column: &&ColumnDef| column.get_column_name().eq(name))
                    .unwrap(),
            )
        })
        .map(|column_info: Column| column_info.get_rs_type())
        .collect();

    let optional_relation = column_types
        .iter()
        .any(|column_type: &TokenStream| column_type.to_string().starts_with("Option"));

    ForeignKeyMeta {
        source_columns: fk.get_columns(),
        destination_columns: fk.get_ref_columns(),
        column_types,

        source_table_name,
        source_table_module,

        destination_table_name,
        destination_table_module,

        optional_relation,
    }
}
