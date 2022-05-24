use proc_macro2::{TokenStream};
use quote::{format_ident, quote};
use std::{fs};
use crate::types::{TableMeta, ColumnMeta};

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
    tables_meta: &Vec<TableMeta>,
) {
    let graphql_entities: Vec<TokenStream> = tables_meta
        .iter()
        .map(|table: &TableMeta| {
            let entity_module = format_ident!("{}", table.entity_module);
            let entity_name = format_ident!("{}", table.entity_name);
            let entity_filter = format_ident!("{}", table.entity_filter);

            let filters: Vec<TokenStream> = table
                .columns
                .iter()
                .map(|column: &ColumnMeta| {
                    let column_name = format_ident!("{}", column.column_name);
                    let column_filter_type = column.column_filter_type.clone();

                    quote!{
                        pub #column_name: Option<TypeFilter<#column_filter_type>>
                    }
                })
                .collect();

            let getters: Vec<TokenStream> = table
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
                .collect();

            quote! {
                use crate::orm::#entity_module::Model as #entity_name;

                #[async_graphql::Object]
                impl #entity_name {
                    #(#getters)*
                }

                #[derive(async_graphql::InputObject, Debug)]
                pub struct #entity_filter {
                    pub or: Option<Vec<Box<#entity_filter>>>,
                    pub and: Option<Vec<Box<#entity_filter>>>,
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

fn generate_filter_recursive(table: &TableMeta) -> TokenStream {
    let entity_module = format_ident!("{}", table.entity_module);
    let entity_filter = format_ident!("{}", table.entity_filter);

    let columns_filters: Vec<TokenStream> = table
        .columns
        .iter()
        .map(|column: &ColumnMeta| {
            let column_name = format_ident!("{}", column.column_name);
            let column_enum_name = format_ident!("{}", column.column_enum_name);

            quote! {
                if let Some(#column_name) = current_filter.#column_name {
                    if let Some(eq_value) = #column_name.eq {
                        condition = condition.add(orm::#entity_module::Column::#column_enum_name.eq(eq_value))
                    }

                    if let Some(ne_value) = #column_name.ne {
                        condition = condition.add(orm::#entity_module::Column::#column_enum_name.ne(ne_value))
                    }
                }
            }
        })
        .collect();

    quote! {
        fn filter_recursive(root_filter: Option<entities::#entity_filter>) -> sea_orm::Condition {
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

fn generate_root(
    dir: &std::path::Path,
    tables_meta: &Vec<TableMeta>,
) {
    let mod_tokens = quote! {
        pub mod entities;

        pub mod query_root;

        pub use query_root::QueryRoot;
    };
    fs::write(dir.join("mod.rs"), mod_tokens.to_string()).unwrap();

    let single_queries: Vec<TokenStream> = tables_meta
        .iter()
        .map(|table: &TableMeta| {
            let entity_module = format_ident!("{}", table.entity_module);
            let entity_filter = format_ident!("{}", table.entity_filter);

            let filter_recursive = generate_filter_recursive(table);

            quote! {
                async fn #entity_module<'a>(&self, ctx: &Context<'a>, filters: Option<entities::#entity_filter>) -> Vec<orm::#entity_module::Model> {
                    println!("filters: {:?}", filters);

                    #filter_recursive

                    let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();

                    let data: Vec<orm::#entity_module::Model> = orm::#entity_module::Entity::find()
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
    tables_meta: Vec<TableMeta>,
) {
    generate_graphql_entities(dir, &tables_meta);
    generate_root(dir, &tables_meta);
}
