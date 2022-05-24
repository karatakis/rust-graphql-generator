use super::column_meta::ColumnMeta;
use super::foreign_key_meta::ForeignKeyMeta;

#[derive(Clone, Debug)]
pub struct TableMeta {
    pub entity_name: String, // CamelCase
    pub entity_module: String, // snake_case
    pub entity_filter: String, // CameCase + 'Filter'
    pub columns: Vec<ColumnMeta>,
    pub foreign_keys: Vec<ForeignKeyMeta>,
}