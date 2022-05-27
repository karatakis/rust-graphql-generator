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

    let reverse_foreign_keys: Vec<(String, ForeignKeyMeta)> = schema
        .tables
        .clone()
        .iter()
        .map(|table: &TableDef| {
            let table_create_stmt = table.write();

            let foreign_keys: Vec<(String, ForeignKeyMeta)> = table_create_stmt
                .get_foreign_key_create_stmts()
                .iter()
                .map(|fk: &ForeignKeyCreateStatement| fk.get_foreign_key())
                .map(|fk: &TableForeignKey| {
                    let foreign_key = parse_table_fk(&table.name, fk, &table_create_stmt, true);

                    (
                        fk.get_ref_table().unwrap().to_upper_camel_case(),
                        foreign_key,
                    )
                })
                .collect();

            foreign_keys
        })
        .fold(
            Vec::<(String, ForeignKeyMeta)>::new(),
            |acc: Vec<(String, ForeignKeyMeta)>, cur: Vec<(String, ForeignKeyMeta)>| {
                [acc, cur].concat()
            },
        );

    let tables_meta = schema
        .tables
        .iter()
        .map(|table: &TableDef| {
            let table_create_stmt = table.write();

            let entity_name = table.name.to_upper_camel_case();

            let foreign_keys: Vec<ForeignKeyMeta> = table_create_stmt
                .get_foreign_key_create_stmts()
                .iter()
                .map(|fk: &ForeignKeyCreateStatement| fk.get_foreign_key())
                .map(|fk: &TableForeignKey| {
                    parse_table_fk(&table.name, fk, &table_create_stmt, false)
                })
                .collect();

            let reverse_fks: Vec<ForeignKeyMeta> = reverse_foreign_keys
                .iter()
                .filter(|fk: &&(String, ForeignKeyMeta)| fk.0.eq(&entity_name))
                .map(|fk: &(String, ForeignKeyMeta)| fk.1.clone())
                .collect();

            let foreign_keys = [foreign_keys, reverse_fks].concat();

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
    table_name: &String,
    fk: &TableForeignKey,
    table_create_stmt: &sea_query::TableCreateStatement,
    reverse: bool,
) -> ForeignKeyMeta {
    let table_name = if reverse {
        table_name.clone()
    } else {
        fk.get_ref_table().unwrap().to_upper_camel_case()
    };
    let table_module = table_name.to_snake_case();

    // TODO if column_types not needed remove
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
        ref_columns: if reverse { fk.get_columns() } else { fk.get_ref_columns()},
        columns: if reverse { fk.get_ref_columns() } else { fk.get_columns()},
        table_name,
        table_module,
        column_types: column_types,
        many_relation: reverse,
        optional_relation,
    }
}
