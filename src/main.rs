// use proc_macro2::TokenStream;
use heck::{ToUpperCamelCase};
use quote::{format_ident, quote};
use rust_graphql_generator_demo::{
    database_schema::get_database_schema, toml_generator::write_toml, column_mapping::column_mapping,
};
use sqlx::SqlitePool;
use std::{env, fs, process};
use sea_schema::sea_query::table::TableCreateStatement;

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = env::args().collect();

    let default_path: String = "generated".into();
    let path: &String = arguments.get(1).unwrap_or(&default_path);

    fs::create_dir(path).unwrap();

    write_toml(path, &"generated".into());

    // sea generate entity -u sqlite://chinook.db -o ./generated/src/orm --expanded-format
    process::Command::new("sea")
        .arg("generate")
        .arg("entity")
        .arg("-u")
        .arg("sqlite://chinook.db")
        .arg("-o")
        .arg(format!("{}/src/orm", path))
        .arg("--expanded-format")
        .output()
        .unwrap();

    let connection = SqlitePool::connect("sqlite://chinook.db").await.unwrap();

    let database_schema = get_database_schema(connection).await.unwrap();

    let table_definitions: Vec<_> = database_schema
        .tables
        .clone()
        .into_iter()
        .map(|table| {
            let table_meta: TableCreateStatement = table.write();

            let ident = format_ident!("{}", table.name);
            let name = format_ident!("{}", table.name.to_upper_camel_case());

            let getters: Vec<_> = table_meta
                .get_columns()
                .into_iter()
                .map(|column| {
                    column_mapping(column)
                })
                .collect();

            quote! {
                #[derive(Debug)]
                struct #name (generated::orm::#ident::Model);

                #[async_graphql::Object]
                impl #name {
                    #(#getters)*
                }
            }
        })
        .collect();

    let single_queries: Vec<_> = database_schema
        .tables
        .into_iter()
        .map(|table| {
            let table_name = format_ident!("{}", table.name);

            let struct_name = format_ident!("{}", table.name.to_upper_camel_case());

            quote!{
                async fn #table_name(&self) -> Vec<#struct_name> {
                    vec![]
                }
            }
        })
        .collect();

    let tokens = quote! {
        use async_graphql::{
            http::{playground_source, GraphQLPlaygroundConfig},
            EmptyMutation, EmptySubscription, Schema,
        };
        use async_graphql_poem::GraphQL;
        use poem::{get, handler, listener::TcpListener, web::Html, IntoResponse, Route, Server};
        use sea_orm::prelude::{DateTime, Decimal};

        #(#table_definitions)*

        struct QueryRoot;

        #[async_graphql::Object]
        impl QueryRoot {
            #(#single_queries)*
        }

        #[handler]
        async fn graphql_playground() -> impl IntoResponse {
            Html(playground_source(GraphQLPlaygroundConfig::new("/")))
        }

        #[tokio::main]
        async fn main() {
            let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
                // .data(StarWars::new())
                .finish();
            let app = Route::new().at("/", get(graphql_playground).post(GraphQL::new(schema)));
            println!("Playground: http://localhost:8000");
            Server::new(TcpListener::bind("0.0.0.0:8000"))
                .run(app)
                .await
                .unwrap();
        }
    };

    fs::write(format!("{}/src/main.rs", path), tokens.to_string()).unwrap();

    let tokens = quote! {
        pub mod orm;
    };

    fs::write(format!("{}/src/lib.rs", path), tokens.to_string()).unwrap();

    env::set_current_dir(path).unwrap();

    process::Command::new("cargo").arg("fmt").output().unwrap();
}
