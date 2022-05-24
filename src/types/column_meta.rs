use proc_macro2::TokenStream;

#[derive(Clone, Debug)]
pub struct ColumnMeta {
    pub column_name: String,      // snake_case
    pub column_enum_name: String, // CamelCase
    pub column_type: TokenStream,
    pub column_filter_type: TokenStream,
    pub not_null: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
}