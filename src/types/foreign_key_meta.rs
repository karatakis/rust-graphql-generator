use proc_macro2::TokenStream;

#[derive(Clone)]
pub struct ForeignKeyMeta {

    pub source_table_name: String,   // CamelCase
    pub source_table_module: String, // snake_case
    pub source_columns: Vec<String>,     // Vec<CamelCase>
    pub source_column_types: Vec<TokenStream>,

    pub destination_table_name: String,   // CamelCase
    pub destination_table_module: String, // snake_case
    pub destination_columns: Vec<String>, // Vec<CamelCase>
    pub destination_column_types: Vec<TokenStream>,
}

impl ForeignKeyMeta {
    pub fn is_reverse(self: &Self, table_name: &String) -> bool {
        self.destination_table_name.eq(table_name)
    }

    pub fn is_optional(self: &Self, is_reverse: bool) -> bool {
        let column_types = if is_reverse {&self.destination_column_types} else {&self.source_column_types};

        column_types
            .iter()
            .any(|column_type: &TokenStream| column_type.to_string().starts_with("Option"))
    }

    pub fn is_source_optional(self: &Self) -> bool {
        self.is_optional(false)
    }

    pub fn is_destination_optional(self: &Self) -> bool {
        self.is_optional(true)
    }

    pub fn get_optional_columns(self: &Self, is_reverse: bool) -> Vec<bool> {
        let column_types = if is_reverse {&self.destination_column_types} else {&self.source_column_types};

        column_types
            .iter()
            .map(|column_type: &TokenStream| column_type.to_string().starts_with("Option"))
            .collect()
    }

    pub fn get_source_optional(self: &Self) -> Vec<bool> {
        self.get_optional_columns(false)
    }

    pub fn get_destination_optional(self: &Self) -> Vec<bool> {
        self.get_optional_columns(true)
    }
}

impl std::fmt::Debug for ForeignKeyMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForeignKeyMeta")
        .field("source_table_name", &self.source_table_name)
        .field("source_columns", &self.source_columns)
        .field("source_column_types", &self.source_column_types)
        .field("source_optional", &self.is_optional(false))

        .field("destination_table_name", &self.destination_table_name)
        .field("destination_columns", &self.destination_columns)
        .field("destination_column_types", &self.destination_column_types)
        .field("destination_optional", &self.is_optional(true))
        .finish()
    }
}
