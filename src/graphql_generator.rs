use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{TokenStream, Ident};
use quote::{format_ident, quote};
use sea_schema::sea_query::table::TableCreateStatement;
use std::{collections::HashMap, fs};

use sea_orm_codegen::Column;

fn generate_generic_input_type_filter() -> TokenStream {
    quote! {
        #[derive(async_graphql::InputObject, Debug)]
        #[graphql(concrete(name = "StringFilter", params(String)))]
        #[graphql(concrete(name = "TinyIntegerFilter", params(i8)))]
        #[graphql(concrete(name = "SmallIntegerFilter", params(i16)))]
        #[graphql(concrete(name = "IntegerFilter", params(i32)))]
        #[graphql(concrete(name = "BigIntegerFilter", params(i64)))]
        #[graphql(concrete(name = "TinyUnsignedFilter", params(u8)))]
        #[graphql(concrete(name = "SmallUnsignedFilter", params(u16)))]
        #[graphql(concrete(name = "UnsignedFilter", params(u32)))]
        #[graphql(concrete(name = "BigUnsignedFilter", params(u64)))]
        #[graphql(concrete(name = "FloatFilter", params(f32)))]
        #[graphql(concrete(name = "DoubleFilter", params(f64)))]
        // TODO #[graphql(concrete(name = "JsonFilter", params()))]
        // TODO #[graphql(concrete(name = "DateFilter", params()))]
        // TODO #[graphql(concrete(name = "TimeFilter", params()))]
        #[graphql(concrete(name = "DateTimeFilter", params(DateTime)))]
        // TODO #[graphql(concrete(name = "TimestampFilter", params()))]
        // TODO #[graphql(concrete(name = "TimestampWithTimeZoneFilter", params()))]
        #[graphql(concrete(name = "DecimalFilter", params(Decimal)))]
        // TODO #[graphql(concrete(name = "UuidFilter", params(uuid::Uuid)))]
        // TODO #[graphql(concrete(name = "BinaryFilter", params()))]
        #[graphql(concrete(name = "BooleanFilter", params(bool)))]
        // TODO #[graphql(concrete(name = "EnumFilter", params()))]
        pub struct TypeFilter<T: async_graphql::InputType> {
            pub eq: Option<T>,
            pub ne: Option<T>,
            pub gt: Option<T>,
            pub gte: Option<T>,
            pub lt: Option<T>,
            pub lte: Option<T>,
            pub is_in: Option<Vec<T>>,
            pub is_not_in: Option<Vec<T>>,
            pub is_null: Option<bool>,
        }
    }
}

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

            let getters: Vec<_> = table_meta
                .clone()
                .get_columns()
                .into_iter()
                .map(|column| {
                    // let specs = column.get_column_spec();
                    let column: Column = Column::from(column);

                    let column_name = column.get_name_snake_case();
                    let column_type: TokenStream = column.get_rs_type();

                    // used to convert Option<T> -> T
                    let filter_column_type: proc_macro2::TokenTree = column_type.clone().into_iter().find(|token: &proc_macro2::TokenTree| {
                        if let proc_macro2::TokenTree::Ident(ident) = token {
                            if ident.eq("Option") {
                                false
                            } else {
                                true
                            }
                        } else {
                            false
                        }
                    }).unwrap();

                    filters.push(quote!{
                        pub #column_name: Option<TypeFilter<#filter_column_type>>
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

                #[derive(async_graphql::InputObject, Debug)]
                pub struct #filter_name {
                    pub or: Option<Vec<Box<#filter_name>>>,
                    pub and: Option<Vec<Box<#filter_name>>>,
                    #(#filters),*
                }
            }
        })
        .collect();

    let generic_input_type_filter = generate_generic_input_type_filter();

    let tokens = quote! {
        use sea_orm::prelude::{DateTime, Decimal};

        #generic_input_type_filter

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
