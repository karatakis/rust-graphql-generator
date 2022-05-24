use proc_macro2::TokenStream;

#[derive(Clone, Debug)]
pub struct ForeignKeyMeta {
    pub columns: Vec<String>,
    pub types: Vec<TokenStream>,
    pub table_name: String, // CamelCase
    pub table_module: String, // snake_case
}