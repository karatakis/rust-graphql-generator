use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_orm_data_loader() -> TokenStream {
    quote! {
        use sea_orm::prelude::*;

        pub struct OrmDataLoader {
            pub db: DatabaseConnection,
        }
    }
}