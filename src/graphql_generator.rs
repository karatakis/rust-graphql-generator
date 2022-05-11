use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sea_schema::sea_query::table::TableCreateStatement;
use std::{collections::HashMap, fs};

use crate::column_mapping::column_mapping;

fn generate_graphql_entities(
    dir: &std::path::Path,
    crate_stmts_map: HashMap<String, TableCreateStatement>,
) {
    let graphql_entities: Vec<TokenStream> = crate_stmts_map
        .iter()
        .map(|(name, table_meta)| {
            let table_name = format_ident!("{}", name.to_snake_case());
            let struct_name = format_ident!("{}", name.to_upper_camel_case());

            let getters: Vec<_> = table_meta
                .get_columns()
                .into_iter()
                .map(|column| column_mapping(column))
                .collect();

            quote! {
                use crate::orm::#table_name::Model as #struct_name;

                #[async_graphql::Object]
                impl #struct_name {
                    #(#getters)*
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

            quote! {
                async fn #table_name<'a>(&self, ctx: &Context<'a>) -> Vec<#struct_name> {
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
        use super::entities;

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
