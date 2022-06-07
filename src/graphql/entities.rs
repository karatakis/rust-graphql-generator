use crate::types::{ColumnMeta, ForeignKeyMeta, TableMeta};
use heck::{ToSnakeCase};
use proc_macro2::{Ident, TokenStream, Literal};
use quote::{format_ident, quote};
use std::collections::HashMap;

pub fn generate_graphql_entities(tables_meta: &Vec<TableMeta>) -> HashMap<String, TokenStream> {
    let entities: HashMap<String, TokenStream> = tables_meta
        .iter()
        .map(|table: &TableMeta| {
            let entity_module: Ident = format_ident!("{}", table.entity_module);
            let entity_name = format!("{}", table.entity_name);
            let entity_filter = format!("{}Filter", table.entity_name);

            let filters: Vec<TokenStream> = generate_entity_filters(table);
            let getters: Vec<TokenStream> = generate_entity_getters(table);
            let relations: Vec<TokenStream> = generate_entity_relations(table);
            // let foreign_keys: Vec<TokenStream> = generate_foreign_keys_and_loaders(table);

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

                // WIP
                // struct Dataloader {
                //     db: DatabaseConnection
                // }

                // #(#foreign_keys)*
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
            let reverse = fk.destination_table_name.eq(&table.entity_name);

            let source_columns = if reverse { &fk.destination_columns } else { &fk.source_columns};
            let destination_columns = if reverse { &fk.source_columns } else { &fk.destination_columns};

            // TODO deduplicate code
            let source_name = source_columns
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

            let destination_table_module = if reverse { &fk.source_table_module } else { &fk.destination_table_module };
            let relation_name = format_ident!("{}_{}", source_name, destination_table_module);
            let destination_table_module = format_ident!("{}", destination_table_module);

            // TODO support multiple keys
            let dest_column = &destination_columns[0];
            let dest_column = format_ident!("{}", dest_column);
            let source_column = &source_columns[0];
            let source_column = format_ident!("{}", source_column.to_snake_case());

            let return_type: TokenStream = if reverse {
                quote! {
                    Vec<crate::orm::#destination_table_module::Model>
                }
            } else if fk.optional_relation {
                quote! {
                    Option<crate::orm::#destination_table_module::Model>
                }
            } else {
                quote! {
                    crate::orm::#destination_table_module::Model
                }
            };

            let data_query: TokenStream = if reverse {
                quote! {
                    let data: #return_type = crate::orm::#destination_table_module::Entity::find()
                        .filter(filter)
                        .all(db)
                        .await
                        .unwrap();
                }
            } else if fk.optional_relation {
                quote! {
                    let data: #return_type = crate::orm::#destination_table_module::Entity::find()
                        .filter(filter)
                        .one(db)
                        .await
                        .unwrap();
                }
            } else {
                quote! {
                    let data: #return_type = crate::orm::#destination_table_module::Entity::find()
                        .filter(filter)
                        .one(db)
                        .await
                        .unwrap()
                        .unwrap();
                }
            };

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
                        .add(crate::orm::#destination_table_module::Column::#dest_column.eq(self.#source_column));

                    #data_query

                    data
                }
            }
        })
        .collect()
}

pub fn generate_foreign_keys_and_loaders(table: &TableMeta) -> Vec<TokenStream> {
    table
        .foreign_keys
        .iter()
        .map(|fk: &ForeignKeyMeta| {
            let field_indexes: Vec<Literal> = (0..fk.column_types.clone().len()).map(|n| Literal::usize_unsuffixed(n)).collect();

            let column_names: Vec<Ident> = fk.destination_columns.iter().map(|name| format_ident!("{}", name)).collect();
            let column_names_snake: Vec<Ident> = fk.destination_columns.iter().map(|name| format_ident!("{}", name.to_snake_case())).collect();

            let fk_name = format_ident!("{}{}FK", fk.source_table_name, fk.destination_table_name);

            let reverse = fk.destination_table_name.eq(&table.entity_name);


            let field_types = if reverse {&fk.column_types} else {&fk.column_types};

            let return_type: TokenStream = if reverse {
                quote! {
                    Vec<Model>
                }
            } else {
                quote! {
                    Model
                }
            };

            let return_result: TokenStream = if reverse {
                quote! {
                    let hashmap: std::collections::HashMap<#fk_name, #return_type> = std::collections::HashMap::new();

                    Ok(data.fold(
                        hashmap,
                        |mut acc: std::collections::HashMap<#fk_name, #return_type>, (key, model)| {
                            if !acc.contains_key(&key) {
                                acc.insert(key.clone(), Vec::new()).unwrap();
                            }
                            acc.get_mut(&key).unwrap().push(model);
                            acc
                        }
                    ))
                }
            } else {
                quote! {
                    Ok(data.collect())
                }
            };

            quote! {

                #[derive(Clone, Eq, PartialEq, Hash)]
                pub struct #fk_name(#(#field_types),*);

                #[async_trait::async_trait]
                impl async_graphql::dataloader::Loader<#fk_name> for Dataloader {
                    type Value = #return_type;
                    type Error = std::sync::Arc<sea_orm::error::DbErr>;

                    async fn load(&self, keys: &[#fk_name]) -> Result<std::collections::HashMap<#fk_name, Self::Value>, Self::Error> {
                        let filter = sea_orm::Condition::all()
                            .add(
                                sea_orm::sea_query::SimpleExpr::Binary(
                                    Box::new(
                                        sea_orm::sea_query::SimpleExpr::Tuple(vec![
                                            #(sea_orm::sea_query::Expr::col(Column::#column_names.as_column_ref()).into_simple_expr()),*
                                        ])
                                    ),
                                    sea_orm::sea_query::BinOper::In,
                                    Box::new(
                                        sea_orm::sea_query::SimpleExpr::Tuple(
                                            keys
                                                .iter()
                                                .map(|tuple|
                                                    sea_orm::sea_query::SimpleExpr::Values(vec![#(tuple.#field_indexes.into()),*])
                                                )
                                                .collect()
                                        )
                                    )
                                )
                            );

                        let data =  Entity::find()
                            .filter(filter)
                            .all(&self.db)
                            .await?
                            .into_iter()
                            .map(|model| {
                                let key = #fk_name(#(model.#column_names_snake),*);

                                (key, model)
                            });

                        #return_result
                    }
                }
            }
        })
        .collect()
}
