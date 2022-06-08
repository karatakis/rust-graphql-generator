pub mod entities;
pub mod type_filter;
pub mod root_node;
pub mod orm_data_loader;

use quote::{quote, format_ident};
use crate::types::TableMeta;

use entities::generate_graphql_entities;
use type_filter::generate_type_filter;
use root_node::generate_root;
use proc_macro2::Ident;

pub fn write_type_filter(dir: &String)
{
    let dir = std::path::Path::new(dir);

    let type_filter = generate_type_filter();

    std::fs::write(dir.join("type_filter.rs"), type_filter.to_string()).unwrap();
}

pub fn write_entities(dir: &String, tables_meta: &Vec<TableMeta>) {
    let dir = std::path::Path::new(dir);

    std::fs::create_dir_all(dir).unwrap();

    let entities = generate_graphql_entities(tables_meta);

    for (name, entity) in entities.iter() {
        std::fs::write(dir.join(format!("{}.rs", name)), entity.to_string()).unwrap();
    }

    let entity_names: Vec<Ident> = entities
        .keys()
        .map(|name: &String| {
            format_ident!("{}", name)
        })
        .collect();

    let mod_tokens = quote!{
        #(pub mod #entity_names;)*
    };

    std::fs::write(dir.join("mod.rs"), mod_tokens.to_string()).unwrap();
}

pub fn write_root_node(dir: &String, tables_meta: &Vec<TableMeta>) {
    let tokens = generate_root(tables_meta);

    let dir = std::path::Path::new(dir);
    std::fs::write(dir.join("query_root.rs"), tokens.to_string()).unwrap();
}

pub fn write_orm_data_loader(dir: &String) {
    let tokens = orm_data_loader::generate_orm_data_loader();

    let dir = std::path::Path::new(dir);
    std::fs::write(dir.join("orm_data_loader.rs"), tokens.to_string()).unwrap();
}

pub fn write_graphql(dir: &String, tables_meta: &Vec<TableMeta>) {
    write_entities(&format!("{}/entities", dir), tables_meta);

    write_type_filter(dir);


    write_root_node(dir, tables_meta);

    write_orm_data_loader(dir);

    let mod_tokens = quote!{
        pub mod entities;
        pub mod query_root;
        pub mod type_filter;
        pub mod orm_data_loader;
        pub use query_root::QueryRoot;
        pub use type_filter::TypeFilter;
        pub use orm_data_loader::OrmDataLoader;
    };

    std::fs::write(std::path::Path::new(dir).join("mod.rs"), mod_tokens.to_string()).unwrap();
}