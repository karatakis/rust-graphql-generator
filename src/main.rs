use quote::{format_ident, quote};
use rust_graphql_generator_demo::{
    database_schema::get_database_schema, entities_generator::generate_entities,
    graphql_generator::generate_graphql, toml_generator::write_toml,
};
use sea_schema::sea_query::table::TableCreateStatement;
use sqlx::SqlitePool;
use std::{collections::HashMap, env, fs, path, process};

#[tokio::main]
async fn main() {
    // TODO proper CLI application
    let arguments: Vec<String> = env::args().collect();

    let default_project_name: String = "generated".into();
    let project_name: &String = arguments.get(1).unwrap_or(&default_project_name);
    let project_dir = path::Path::new(project_name);

    let connection = SqlitePool::connect("sqlite://chinook.db").await.unwrap();

    let database_schema = get_database_schema(connection).await.unwrap();

    let mut crate_stmts_map: HashMap<String, TableCreateStatement> = HashMap::new();

    for table in database_schema.tables.into_iter() {
        crate_stmts_map.insert(table.name.clone(), table.write());
    }

    {
        let table_create_stmts: Vec<TableCreateStatement> =
            crate_stmts_map.values().cloned().collect();

        let folder: String = format!("{}/src/orm", project_name).into();
        let dir = path::Path::new(&folder);
        fs::create_dir_all(dir).unwrap();
        generate_entities(dir, table_create_stmts).unwrap();
    }

    write_toml(project_dir, &"generated".into()).unwrap();

    {
        let folder: String = format!("{}/src/graphql", project_name).into();
        let dir = path::Path::new(&folder);
        fs::create_dir_all(dir).unwrap();
        generate_graphql(dir, crate_stmts_map);
    }

    let lib_tokens = quote! {
        pub mod orm;
        pub mod graphql;

        pub use graphql::QueryRoot;
    };
    fs::write(
        format!("{}/src/lib.rs", project_name),
        lib_tokens.to_string(),
    )
    .unwrap();

    let crate_name = format_ident!("{}", project_name);
    let main_tokens = quote! {
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
    fs::write(
        format!("{}/src/main.rs", project_name),
        main_tokens.to_string(),
    )
    .unwrap();

    env::set_current_dir(project_dir).unwrap();

    process::Command::new("cargo").arg("fmt").output().unwrap();
}
