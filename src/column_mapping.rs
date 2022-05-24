use proc_macro2::TokenStream;
use quote::quote;
use sea_orm_codegen::Column;
use sea_query::table::ColumnDef;

pub fn column_mapping(column: &ColumnDef) -> TokenStream {
    let column: Column = Column::from(column);

    let column_name = column.get_name_snake_case();
    let column_type = column.get_rs_type();

    quote! {
      async fn #column_name(&self) -> &#column_type {
        &self.#column_name
      }
    }
}
