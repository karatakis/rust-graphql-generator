use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{TokenStream, Ident};
use quote::{format_ident, quote};
use sea_schema::sea_query::table::TableCreateStatement;
use std::{collections::HashMap, fs};

use sea_orm_codegen::Column;

fn generate_graphql_entities(
    dir: &std::path::Path,
    crate_stmts_map: HashMap<String, TableCreateStatement>,
) {
    let graphql_entities: Vec<TokenStream> = crate_stmts_map
        .iter()
        .map(|(name, table_meta)| {
            let table_name = format_ident!("{}", name.to_snake_case());
            let struct_name = format_ident!("{}", name.to_upper_camel_case());
            let filter_name = format_ident!("{}Filter", name.to_upper_camel_case());

            let mut filters: Vec<TokenStream> = Vec::new();
            let mut filter_names: Vec<TokenStream> = Vec::new();

            let getters: Vec<_> = table_meta
                .clone()
                .get_columns()
                .into_iter()
                .map(|column| {
                    // let specs = column.get_column_spec();
                    let column: Column = Column::from(column);

                    let column_name = column.get_name_snake_case();
                    let column_type = column.get_rs_type();

                    let column_filter_name = format_ident!("{}{}Filter", struct_name, column.get_name_camel_case());

                    filters.push(quote!{
                        #[derive(async_graphql::InputObject, Debug)]
                        pub struct #column_filter_name {
                            pub eq: Option<#column_type>,
                            pub ne: Option<#column_type>,
                            pub gt: Option<#column_type>,
                            pub gte: Option<#column_type>,
                            pub lt: Option<#column_type>,
                            pub lte: Option<#column_type>,
                            pub is_in: Option<Vec<#column_type>>,
                            pub is_not_in: Option<Vec<#column_type>>,
                            pub is_null: Option<bool>,
                        }
                    });

                    filter_names.push(quote!{
                        pub #column_name: Option<#column_filter_name>
                    });

                    quote! {
                        pub async fn #column_name(&self) -> &#column_type {
                            &self.#column_name
                        }
                    }
                })
                .collect();

            quote! {
                use crate::orm::#table_name::Model as #struct_name;

                #[async_graphql::Object]
                impl #struct_name {
                    #(#getters)*
                }

                #(#filters)*

                #[derive(async_graphql::InputObject, Debug)]
                pub struct #filter_name {
                    pub or: Option<Vec<Box<#filter_name>>>,
                    pub and: Option<Vec<Box<#filter_name>>>,
                    #(#filter_names),*
                }
            }
        })
        .collect();

    let tokens = quote! {
        use sea_orm::prelude::{DateTime, Decimal};

        #(#graphql_entities)*
    };

    fs::write(dir.join("entities.rs"), tokens.to_string()).unwrap();
}

fn generate_filter_recursive(table_name: &Ident, filter_name: &Ident, meta: &TableCreateStatement) -> TokenStream {

    let columns_filters: Vec<TokenStream> = meta
        .get_columns()
        .into_iter()
        .map(|column| {
            let column_name = format_ident!("{}", column.get_column_name().to_snake_case());
            let column_upper_name = format_ident!("{}", column.get_column_name().to_upper_camel_case());

            quote! {
                if let Some(#column_name) = current_filter.#column_name {
                    if let Some(eq_value) = #column_name.eq {
                        condition = condition.add(orm::#table_name::Column::#column_upper_name.eq(eq_value))
                    }

                    if let Some(ne_value) = #column_name.ne {
                        condition = condition.add(orm::#table_name::Column::#column_upper_name.ne(ne_value))
                    }
                }
            }
        })
        .collect();

    quote! {
        fn filter_recursive(root_filter: Option<entities::#filter_name>) -> sea_orm::Condition {
            let mut condition = sea_orm::Condition::all();

            if let Some(current_filter) = root_filter {
                if let Some(or_filters) = current_filter.or {
                    let or_condition = or_filters
                        .into_iter()
                        .fold(
                            sea_orm::Condition::any(),
                            |fold_condition, filter| fold_condition.add(filter_recursive(Some(*filter)))
                        );
                    condition = condition.add(or_condition);
                }

                if let Some(and_filters) = current_filter.and {
                    let and_condition = and_filters
                        .into_iter()
                        .fold(
                            sea_orm::Condition::all(),
                            |fold_condition, filter| fold_condition.add(filter_recursive(Some(*filter)))
                        );
                    condition = condition.add(and_condition);
                }

                #(#columns_filters)*
            }

            condition
        }
    }
}

fn generate_root(dir: &std::path::Path, crate_stmts_map: HashMap<String, TableCreateStatement>) {
    let mod_tokens = quote! {
        pub mod entities;

        pub mod query_root;

        pub use query_root::QueryRoot;
    };
    fs::write(dir.join("mod.rs"), mod_tokens.to_string()).unwrap();

    let single_queries: Vec<TokenStream> = crate_stmts_map
        .iter()
        .map(|(name, table_meta)| {
            let table_name = format_ident!("{}", name.to_snake_case());
            let filter_name = format_ident!("{}Filter", name.to_upper_camel_case());

            let filter_recursive = generate_filter_recursive(&table_name, &filter_name, &table_meta);

            quote! {
                async fn #table_name<'a>(&self, ctx: &Context<'a>, filters: Option<entities::#filter_name>) -> Vec<orm::#table_name::Model> {
                    println!("filters: {:?}", filters);

                    #filter_recursive

                    let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();

                    let data: Vec<orm::#table_name::Model> = orm::#table_name::Entity::find()
                        .filter(filter_recursive(filters))
                        .all(db).await.unwrap();

                    data
                }
            }
        })
        .collect();

    let query_root_tokens = quote! {
        use super::entities;
        use crate::orm;

        use async_graphql::Context;
        use sea_orm::prelude::*;


        pub struct QueryRoot;

        #[async_graphql::Object]
        impl QueryRoot {
            #(#single_queries)*
        }
    };

    fs::write(dir.join("query_root.rs"), query_root_tokens.to_string()).unwrap();
}

pub fn generate_graphql(
    dir: &std::path::Path,
    crate_stmts_map: HashMap<String, TableCreateStatement>,
) {
    generate_graphql_entities(dir, crate_stmts_map.clone());
    generate_root(dir, crate_stmts_map);
}
