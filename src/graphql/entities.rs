use crate::types::{ColumnMeta, ForeignKeyMeta, TableMeta};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use heck::{ToSnakeCase};
use std::collections::HashMap;

pub fn generate_graphql_entities(tables_meta: &Vec<TableMeta>) -> HashMap<String, TokenStream> {
    let entities: HashMap<String, TokenStream> = tables_meta
        .iter()
        .map(|table: &TableMeta| {
            let entity_module: Ident = format_ident!("{}", table.entity_module);
            let entity_name = format!("{}", table.entity_name);
            let entity_filter = format!("{}Filter", table.entity_name);

            println!("{} - {:?}", table.entity_name, table.foreign_keys);

            let filters: Vec<TokenStream> = generate_entity_filters(table);
            let getters: Vec<TokenStream> = generate_entity_getters(table);
            let relations: Vec<TokenStream> = generate_entity_relations(table);

            let entity_tokens: TokenStream = quote! {
                use async_graphql::Context;
                use sea_orm::prelude::*;

                // TODO generic filter name Filter
                // TODO dataloader
                pub use crate::orm::#entity_module::*;
                use crate::graphql::TypeFilter;

                #[async_graphql::Object(name=#entity_name)]
                impl Model {
                    #(#getters)*
                    #(#relations)*
                }

                #[derive(async_graphql::InputObject, Debug)]
                #[graphql(name=#entity_filter)]
                pub struct Filter {
                    pub or: Option<Vec<Box<Filter>>>,
                    pub and: Option<Vec<Box<Filter>>>,
                    #(#filters),*
                }
            };

            (table.entity_module.clone(), entity_tokens)
        })
        .collect();

    entities
}

pub fn generate_entity_filters(table: &TableMeta) -> Vec<TokenStream> {
    table
        .columns
        .iter()
        .map(|column: &ColumnMeta| {
            let column_name = format_ident!("{}", column.column_name);
            let column_filter_type = column.column_filter_type.clone();

            quote! {
                pub #column_name: Option<TypeFilter<#column_filter_type>>
            }
        })
        .collect()
}

pub fn generate_entity_getters(table: &TableMeta) -> Vec<TokenStream> {
    table
        .columns
        .iter()
        .map(|column: &ColumnMeta| {
            let column_name = format_ident!("{}", column.column_name);
            let column_type = column.column_type.clone();

            quote! {
                pub async fn #column_name(&self) -> &#column_type {
                    &self.#column_name
                }
            }
        })
        .collect()
}

// TODO refactor this
pub fn generate_entity_relations(table: &TableMeta) -> Vec<TokenStream> {


    table
        .foreign_keys
        .iter()
        .map(|fk: &ForeignKeyMeta| {
            let relation_name = fk
                .columns
                .clone()
                .into_iter()
                .map(|s: String| s.to_snake_case())
                .map(|s: String| {
                    if s.ends_with("_id") {
                        String::from(s.split_at(s.len() - 3).0)
                    } else {
                        s
                    }
                })
                .collect::<Vec<String>>()
                .join("_");
            let relation_name = format_ident!("{}_{}", relation_name, fk.table_module);
            let table_module = format_ident!("{}", fk.table_module);
            // let table_name = format_ident!("{}", fk.table_name);

            let return_type: TokenStream = if fk.many_relation {
                quote! {
                    Vec<crate::orm::#table_module::Model>
                }
            } else if fk.optional_relation {
                quote! {
                    Option<crate::orm::#table_module::Model>
                }
            } else {
                quote! {
                    crate::orm::#table_module::Model
                }
            };

            let data_query: TokenStream = if fk.many_relation {
                quote! {
                    let data: #return_type = crate::orm::#table_module::Entity::find()
                        .filter(filter)
                        .all(db)
                        .await
                        .unwrap();
                }
            } else if fk.optional_relation {
                quote! {
                    let data: #return_type = crate::orm::#table_module::Entity::find()
                        .filter(filter)
                        .one(db)
                        .await
                        .unwrap();
                }
            } else {
                quote! {
                    let data: #return_type = crate::orm::#table_module::Entity::find()
                        .filter(filter)
                        .one(db)
                        .await
                        .unwrap()
                        .unwrap();
                }
            };

            let column = format_ident!("{}", fk.columns[0].to_snake_case());
            let ref_column = format_ident!("{}", fk.ref_columns[0]);

            // TODO custom realtion filter (with id removed)
            // TODO add filters
            // filters: Option<entities::#table_filter>,

            quote! {
                pub async fn #relation_name<'a>(
                    &self,
                    ctx: &Context<'a>
                ) -> #return_type {
                    let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();

                    let filter = sea_orm::Condition::all()
                        .add(crate::orm::#table_module::Column::#ref_column.eq(self.#column));

                    #data_query

                    data
                }
            }
        })
        .collect()
}

pub fn generate_primary_key_struct(table: &TableMeta) -> TokenStream {
    let types: Vec<TokenStream> = table
        .columns
        .iter()
        .map(|column: &ColumnMeta| {
            column.column_type.clone()
        })
        .collect();

    quote! {
        struct PrimaryKey(#(#types),*);
    }
}

pub fn generate_dataloader(table: &TableMeta) -> TokenStream {
    quote! {

    }
}