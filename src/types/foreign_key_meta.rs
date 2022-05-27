use proc_macro2::TokenStream;

#[derive(Clone)]
pub struct ForeignKeyMeta {
    pub columns: Vec<String>,     // Vec<CamelCase>
    pub ref_columns: Vec<String>, // Vec<CamelCase>
    pub column_types: Vec<TokenStream>,
    pub table_name: String,   // CamelCase
    pub table_module: String, // snake_case
    pub many_relation: bool,
    pub optional_relation: bool,
}

impl std::fmt::Debug for ForeignKeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForeignKeyMeta")
        .field("table_name", &self.table_name)
        .field("columns", &self.columns)
        .field("many_relation", &self.many_relation)
        .field("optional_relation", &self.optional_relation)
        .finish()
    }
}
