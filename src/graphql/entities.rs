use crate::types::{ColumnMeta, ForeignKeyMeta, TableMeta};
use heck::ToSnakeCase;
use proc_macro2::{Ident, Literal, TokenStream};
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
            let foreign_keys: Vec<TokenStream> = generate_foreign_keys_and_loaders(table);

            let entity_tokens: TokenStream = quote! {
                use async_graphql::Context;
                use sea_orm::prelude::*;
                use itertools::Itertools;

                // TODO generate filter parser function

                pub use crate::orm::#entity_module::*;
                use crate::graphql::*;

                #[async_graphql::Object(name=#entity_name)]
                impl Model {
                    #(#getters)*
                    #(#relations)*
                }

                // TODO export to common file
                #[derive(derivative::Derivative, Clone, Eq)]
                #[derivative(Hash, PartialEq, Debug)]
                pub struct FkWithFilter<T, Y> {
                    pub foreign_key: T,
                    #[derivative(PartialEq="ignore")]
                    #[derivative(Hash="ignore")]
                    #[derivative(Debug="ignore")]
                    pub filter: Option<Y>,
                }

                #[derive(async_graphql::InputObject, Debug, Eq, PartialEq, Clone)]
                #[graphql(name=#entity_filter)]
                pub struct Filter {
                    pub or: Option<Vec<Box<Filter>>>,
                    pub and: Option<Vec<Box<Filter>>>,
                    #(#filters),*
                }

                #(#foreign_keys)*
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

pub fn generate_entity_relations(table: &TableMeta) -> Vec<TokenStream> {
    table
        .foreign_keys
        .iter()
        .map(|fk: &ForeignKeyMeta| {
            let reverse = fk.destination_table_name.eq(&table.entity_name);


            let source_entity = if reverse { &fk.destination_table_name } else { &fk.source_table_name };
            let destination_entity = if reverse { &fk.source_table_name } else { &fk.destination_table_name };

            let fk_name = format_ident!("{}{}FK", source_entity, destination_entity);

            let source_columns = if reverse { &fk.destination_columns } else { &fk.source_columns };

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

            let return_type: TokenStream = if reverse {
                quote! {
                    Vec<crate::orm::#destination_table_module::Model>
                }
            } else if fk.is_optional(reverse) {
                quote! {
                    Option<crate::orm::#destination_table_module::Model>
                }
            } else {
                quote! {
                    crate::orm::#destination_table_module::Model
                }
            };

            // TODO add filter on relation
            // filters: Option<entities::#table_filter>,

            let key_items: Vec<Ident> = source_columns
                .iter()
                .map(|name: &String| {
                    format_ident!("{}", name.to_snake_case())
                })
                .collect();

            let return_value: TokenStream = if reverse {
                quote! {
                    data.unwrap_or(vec![])
                }
            } else if fk.is_optional(reverse) {
                quote! {
                    data
                }
            } else {
                quote! {
                    data.unwrap()
                }
            };

            quote! {
                pub async fn #relation_name<'a>(
                    &self,
                    ctx: &Context<'a>,
                    filter: Option<crate::graphql::entities::#destination_table_module::Filter>,
                ) -> #return_type {
                    let data_loader = ctx.data::<async_graphql::dataloader::DataLoader<OrmDataLoader>>().unwrap();

                    let key = FkWithFilter {
                        foreign_key: #fk_name(#(self.#key_items),*),
                        filter,
                    };

                    let data: Option<_> = data_loader.load_one(key).await.unwrap();

                    #return_value
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
            let reverse = fk.is_reverse(&table.entity_name);

            let field_indexes: Vec<Literal> = (0..fk.source_column_types.clone().len()).map(|n| Literal::usize_unsuffixed(n)).collect();

            let source_entity = if reverse { &fk.destination_table_name } else { &fk.source_table_name };
            // let source_table_module = if reverse { &fk.destination_table_module } else { &fk.source_table_module };
            // let source_table_module = format_ident!("{}", source_table_module);
            // let source_column_names = if reverse { &fk.destination_columns } else { &fk.source_columns };
            // let source_column_names: Vec<Ident> = source_column_names.iter().map(|name| format_ident!("{}", name.to_snake_case())).collect();

            let destination_entity = if reverse { &fk.source_table_name } else { &fk.destination_table_name };
            let destination_table_module = if reverse { &fk.source_table_module } else { &fk.destination_table_module };
            let destination_table_module = format_ident!("{}", destination_table_module);
            let destination_column_names = if reverse { &fk.source_columns } else { &fk.destination_columns };
            let destination_columns: Vec<Ident> = destination_column_names.iter().map(|name| format_ident!("{}", name)).collect();
            let destination_column_names: Vec<Ident> = destination_column_names.iter().map(|name| format_ident!("{}", name.to_snake_case())).collect();

            let fk_name = format_ident!("{}{}FK", source_entity, destination_entity);


            let return_type: TokenStream = if reverse {
                quote! {
                    Vec<crate::orm::#destination_table_module::Model>
                }
            } else {
                quote! {
                    crate::orm::#destination_table_module::Model
                }
            };

            let source_field_types = if reverse { &fk.destination_column_types } else { &fk.source_column_types };
            let destination_field_types = if reverse { &fk.source_column_types } else { &fk.destination_column_types };

            let destination_fields: Vec<TokenStream> = destination_column_names
                .iter()
                .enumerate()
                .map(|(index, name)|{
                    let source_type = &destination_field_types[index];
                    let destination_type = &source_field_types[index];
                    let source_optional = source_type.to_string().starts_with("Option");
                    let destination_optional = destination_type.to_string().starts_with("Option");

                    if source_optional && !destination_optional {
                        quote! {
                            model.#name.unwrap()
                        }
                    } else if !source_optional && destination_optional {
                        quote! {
                            Some(model.#name)
                        }
                    } else {
                        quote! {
                            model.#name
                        }
                    }
                })
                .collect();

            let prepare_step = if reverse {
                quote! {
                    .into_group_map()
                }
            } else {
                quote!{
                    .collect()
                }
            };

            quote! {
                #[derive(Clone, Eq, PartialEq, Hash, Debug)]
                pub struct #fk_name(#(#source_field_types),*);

                #[async_trait::async_trait]
                impl async_graphql::dataloader::Loader<FkWithFilter<#fk_name, crate::graphql::entities::#destination_table_module::Filter>> for OrmDataLoader {
                    type Value = #return_type;
                    type Error = std::sync::Arc<sea_orm::error::DbErr>;

                    async fn load(&self, keys: &[FkWithFilter<#fk_name, crate::graphql::entities::#destination_table_module::Filter>]) -> Result<std::collections::HashMap<FkWithFilter<#fk_name, crate::graphql::entities::#destination_table_module::Filter>, Self::Value>, Self::Error> {
                        let external_filter: Option<crate::graphql::entities::#destination_table_module::Filter> = keys[0].clone().filter;

                        let filter = sea_orm::Condition::all()
                            .add(
                                sea_orm::sea_query::SimpleExpr::Binary(
                                    Box::new(
                                        sea_orm::sea_query::SimpleExpr::Tuple(vec![
                                            #(sea_orm::sea_query::Expr::col(crate::orm::#destination_table_module::Column::#destination_columns.as_column_ref()).into_simple_expr()),*
                                        ])
                                    ),
                                    sea_orm::sea_query::BinOper::In,
                                    Box::new(
                                        sea_orm::sea_query::SimpleExpr::Tuple(
                                            keys
                                                .iter()
                                                .map(|key: &FkWithFilter<#fk_name, crate::graphql::entities::#destination_table_module::Filter>|
                                                    sea_orm::sea_query::SimpleExpr::Values(vec![#(key.foreign_key.#field_indexes.into()),*])
                                                )
                                                .collect()
                                        )
                                    )
                                )
                            );

                        Ok(
                            crate::orm::#destination_table_module::Entity::find()
                                .filter(filter)
                                .all(&self.db)
                                .await?
                                .into_iter()
                                .map(|model| {
                                    let key = FkWithFilter {
                                        foreign_key: #fk_name(#(#destination_fields),*),
                                        filter: None,
                                    };

                                    (key, model)
                                })
                                #prepare_step
                        )
                    }
                }
            }
        })
        .collect()
}
