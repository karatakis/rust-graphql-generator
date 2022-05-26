use proc_macro2::TokenStream;

#[derive(Clone)]
pub struct ColumnMeta {
    pub column_name: String,      // snake_case
    pub column_enum_name: String, // CamelCase
    pub column_type: TokenStream,
    pub column_filter_type: TokenStream,
    pub not_null: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
}

impl std::fmt::Debug for ColumnMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnMeta")
        .field("column_name", &self.column_name)
        .field("not_null", &self.not_null)
        .field("column_type", &self.column_type.to_string())
        .finish()
    }
}
