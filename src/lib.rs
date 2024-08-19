extern crate proc_macro;
use proc_macro::TokenStream;

mod expand;
mod parser;

#[allow(clippy::needless_doctest_main)]
/// Due to limitations in rusts proc-macro API there is currently no
/// way to signal that a specific proc macro should be rerun if some
/// external file changes/is added. This implies that `embed_migrations!`
/// cannot regenerate the list of embedded migrations if **only** the
/// migrations are changed. This limitation can be solved by adding a
/// custom `build.rs` file to your crate, such that the crate is rebuild
/// if the migration directory changes.
///
/// Add the following `build.rs` file to your project to fix the problem
///
/// ```
/// fn main() {
///    println!("cargo:rerun-if-changed=path/to/your/migration/dir/relative/to/your/Cargo.toml");
/// }
/// ```

#[proc_macro]
pub fn create_component(item: TokenStream) -> TokenStream {
    expand::expand_template(item.to_string())
}

/// Creates an entrypoint for the application using the specified `.vue` template file
/// 
/// ```
/// use villain::create_entypoint;
/// 
/// fn main() {
///    println!("{:?}", create_entypoint!("src/App.vue"));
/// }
/// ```
#[proc_macro]
pub fn create_entypoint(item: TokenStream) -> TokenStream {
    expand::expand_template(item.to_string())
}
