use proc_macro2::TokenStream;

#[derive(Clone)]
pub struct ForeignKeyMeta {
    pub source_columns: Vec<String>,     // Vec<CamelCase>
    pub destination_columns: Vec<String>, // Vec<CamelCase>

    pub column_types: Vec<TokenStream>,

    pub source_table_name: String,   // CamelCase
    pub source_table_module: String, // snake_case

    pub destination_table_name: String,   // CamelCase
    pub destination_table_module: String, // snake_case

    pub optional_relation: bool,
}

impl std::fmt::Debug for ForeignKeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForeignKeyMeta")
        .field("source_table_name", &self.source_table_name)
        .field("source_columns", &self.source_columns)
        .field("destination_table_name", &self.destination_table_name)
        .field("destination_columns", &self.destination_columns)
        .field("optional_relation", &self.optional_relation)
        .finish()
    }
}
