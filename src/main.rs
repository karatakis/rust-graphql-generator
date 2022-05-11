use heck::{ToUpperCamelCase, ToSnakeCase};
use proc_macro2::TokenStream;
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

    let default_project_name: String = "generated".into();
    let project_name: &String = arguments.get(1).unwrap_or(&default_project_name);
    let crate_name = format_ident!("{}", project_name);

    fs::create_dir(project_name).unwrap();

    write_toml(project_name, &"generated".into());

    // sea generate entity -u sqlite://chinook.db -o ./generated/src/orm --expanded-format
    process::Command::new("sea")
        .arg("generate")
        .arg("entity")
        .arg("-u")
        .arg("sqlite://chinook.db")
        .arg("-o")
        .arg(format!("{}/src/orm", project_name))
        .arg("--expanded-format")
        .output()
        .unwrap();


    // TODO receive it as parameter value
    let connection = SqlitePool::connect("sqlite://chinook.db").await.unwrap();

    let database_schema = get_database_schema(connection).await.unwrap();

    // TODO utilize sea_orm_codegen::EntityWriter <3

    let mut single_queries: Vec<TokenStream> = Vec::new();
    let mut table_definitions: Vec<TokenStream> = Vec::new();

    for table in database_schema.tables.into_iter() {
        let table_meta: TableCreateStatement = table.write();

        let table_name = format_ident!("{}", table.name.to_snake_case());
        let struct_name = format_ident!("{}", table.name.to_upper_camel_case());

        let getters: Vec<_> = table_meta
            .get_columns()
            .into_iter()
            .map(|column| {
                column_mapping(column)
            })
            .collect();

        single_queries.push(quote!{
            async fn #table_name<'a>(&self, ctx: &Context<'a>) -> Vec<#struct_name> {
                use crate::orm::#table_name::Entity;
                let db: &DatabaseConnection = ctx.data::<DatabaseConnection>().unwrap();
                let data: Vec<#struct_name> = Entity::find().all(db).await.unwrap();
                data
            }
        });

        table_definitions.push(quote! {
            use crate::orm::#table_name::Model as #struct_name;

            #[async_graphql::Object]
            impl #struct_name {
                #(#getters)*
            }
        });
    }

    let tokens = quote! {
        use async_graphql::Context;
        use sea_orm::{
            prelude::{DateTime, Decimal},
            DatabaseConnection, EntityTrait,
        };

        pub mod orm;

        #(#table_definitions)*

        pub struct QueryRoot;

        #[async_graphql::Object]
        impl QueryRoot {
            #(#single_queries)*
        }
    };

    fs::write(format!("{}/src/lib.rs", project_name), tokens.to_string()).unwrap();

    let tokens = quote! {
        use async_graphql::{
            http::{playground_source, GraphQLPlaygroundConfig},
            EmptyMutation, EmptySubscription, Schema,
        };
        use async_graphql_poem::GraphQL;
        use poem::{get, handler, listener::TcpListener, web::Html, IntoResponse, Route, Server};
        use sea_orm::Database;

        use #crate_name::QueryRoot;

        #[handler]
        async fn graphql_playground() -> impl IntoResponse {
            Html(playground_source(GraphQLPlaygroundConfig::new("/")))
        }

        #[tokio::main]
        async fn main() {
            let database = Database::connect("sqlite://../chinook.db").await.unwrap();
            let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
                .data(database)
                .finish();
            let app = Route::new().at("/", get(graphql_playground).post(GraphQL::new(schema)));
            println!("Playground: http://localhost:8000");
            Server::new(TcpListener::bind("0.0.0.0:8000"))
                .run(app)
                .await
                .unwrap();
        }

    };

    fs::write(format!("{}/src/main.rs", project_name), tokens.to_string()).unwrap();

    env::set_current_dir(project_name).unwrap();

    process::Command::new("cargo").arg("fmt").output().unwrap();
}
