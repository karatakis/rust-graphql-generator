use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
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
                        struct #column_filter_name {
                            equals: Option<#column_type>,
                            not_equals: Option<#column_type>,
                            greater_than: Option<#column_type>,
                            greater_than_equals: Option<#column_type>,
                            less_than: Option<#column_type>,
                            less_than_equals: Option<#column_type>,
                            r#in: Option<Vec<#column_type>>,
                            not_in: Option<Vec<#column_type>>,
                            is_null: Option<bool>,
                        }
                    });

                    filter_names.push(quote!{
                        #column_name: Option<#column_filter_name>
                    });

                    quote! {
                        async fn #column_name(&self) -> &#column_type {
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
                    or: Option<Vec<Box<#filter_name>>>,
                    and: Option<Vec<Box<#filter_name>>>,
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

fn generate_root(dir: &std::path::Path, crate_stmts_map: HashMap<String, TableCreateStatement>) {
    let use_tokens: Vec<TokenStream> = crate_stmts_map
        .iter()
        .map(|(name, _)| {
            let table_name = format_ident!("{}", name.to_snake_case());
            let struct_name = format_ident!("{}", name.to_upper_camel_case());

            quote! {
                use crate::orm::#table_name::Model as #struct_name;
            }
        })
        .collect();

    let single_queries: Vec<TokenStream> = crate_stmts_map
        .iter()
        .map(|(name, _)| {
            let table_name = format_ident!("{}", name.to_snake_case());
            let struct_name = format_ident!("{}", name.to_upper_camel_case());
            let filter_name = format_ident!("{}Filter", struct_name);

            quote! {
                async fn #table_name<'a>(&self, ctx: &Context<'a>, filters: Option<#filter_name>) -> Vec<#struct_name> {
                    println!("filters: {:?}", filters);
                    use crate::orm::#table_name::Entity;
                    let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();
                    let data: Vec<#struct_name> = Entity::find().all(db).await.unwrap();
                    data
                }
            }
        })
        .collect();

    let mod_tokens = quote! {
        pub mod entities;

        pub mod query_root;

        pub use query_root::QueryRoot;
    };
    fs::write(dir.join("mod.rs"), mod_tokens.to_string()).unwrap();

    let query_root_tokens = quote! {
        use super::entities::*;

        #(#use_tokens)*

        use async_graphql::Context;
        use sea_orm::{DatabaseConnection, EntityTrait};


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
