use crate::types::{ColumnMeta, TableMeta};
use proc_macro2::{TokenStream};
use quote::{format_ident, quote};

fn generate_recursive_filter_fn(table: &TableMeta) -> TokenStream {
    let entity_module = format_ident!("{}", table.entity_module);

    let columns_filters: Vec<TokenStream> = table
        .columns
        .iter()
        .map(|column: &ColumnMeta| {
            let column_name = format_ident!("{}", column.column_name);
            let column_enum_name = format_ident!("{}", column.column_enum_name);

            quote! {
                if let Some(#column_name) = current_filter.#column_name {
                    if let Some(eq_value) = #column_name.eq {
                        condition = condition.add(entities::#entity_module::Column::#column_enum_name.eq(eq_value))
                    }

                    if let Some(ne_value) = #column_name.ne {
                        condition = condition.add(entities::#entity_module::Column::#column_enum_name.ne(ne_value))
                    }
                }
            }
        })
        .collect();

    quote! {
        fn filter_recursive(root_filter: Option<entities::#entity_module::Filter>) -> sea_orm::Condition {
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

pub fn generate_root(tables_meta: &Vec<TableMeta>) -> TokenStream {
    let single_queries: Vec<TokenStream> = tables_meta
        .iter()
        .map(|table: &TableMeta| {
            let entity_module = format_ident!("{}", table.entity_module);

            let filter_recursive = generate_recursive_filter_fn(table);

            quote! {
                async fn #entity_module<'a>(&self, ctx: &Context<'a>, filters: Option<entities::#entity_module::Filter>) -> Vec<entities::#entity_module::Model> {
                    println!("filters: {:?}", filters);

                    #filter_recursive

                    let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();

                    let data: Vec<entities::#entity_module::Model> = entities::#entity_module::Entity::find()
                        .filter(filter_recursive(filters))
                        .all(db).await.unwrap();

                    data
                }
            }
        })
        .collect();

    quote! {
        use super::entities;

        use async_graphql::Context;
        use sea_orm::prelude::*;


        pub struct QueryRoot;

        #[async_graphql::Object]
        impl QueryRoot {
            #(#single_queries)*
        }
    }
}