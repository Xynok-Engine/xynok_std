#![allow(unused)]
use proc_macro::TokenStream;
mod enum_dispatcher;
mod enum_extensions;
mod getter;
mod singleton;

#[proc_macro_attribute]
pub fn enum_dispatcher(args: TokenStream, input: TokenStream) -> TokenStream
{
    enum_dispatcher::enum_dispatcher(args, input)
}

#[proc_macro_attribute]
pub fn enum_extensions(args: TokenStream, input: TokenStream) -> TokenStream
{
    enum_extensions::enum_extensions(args, input)
}

///  A procedural macro to create a singleton struct. The struct must implement `Default` to
///  provide an initial value for the singleton instance. The macro generates a static instance of
///  the struct and an `instance()` method to access it.
/// ```rust
/// #[singleton]
/// #[derive(Default)]
/// struct AppState
/// {
///     counter: u32,
///     running: bool,
/// }
///
/// fn main()
/// {
///     AppState::instance().counter += 1;
///     println!("{}", AppState::instance().counter); // 1
/// }
/// ```
#[proc_macro_attribute]
pub fn global_static(args: TokenStream, input: TokenStream) -> TokenStream
{
    singleton::singleton_gen(args, input)
}

///  A procedural macro to create a readonly singleton struct. The struct must implement `Default`
///  to provide an initial value. The instance is initialized once on first access and afterwards
///  exposed as an immutable shared reference, so no locking is required.
/// ```rust
/// #[global_static_readonly]
/// #[derive(Default)]
/// struct AppConfig
/// {
///     name:    &'static str,
///     version: u32,
/// }
///
/// fn main()
/// {
///     println!(
///         "{} v{}",
///         AppConfig::instance().name,
///         AppConfig::instance().version
///     );
/// }
/// ```
#[proc_macro_attribute]
pub fn global_static_readonly(args: TokenStream, input: TokenStream) -> TokenStream
{
    singleton::singleton_readonly_gen(args, input)
}

/// ! Ý tưởng:
/// ```rust
/// #[getter]
/// pub struct FeatureChain<'a>
/// {
///     #[get(root)]
///     f10:     vk::PhysicalDeviceFeature2<'a>,
///     #[get]
///     linked:  bool,
///     #[get_mut]
///     broken:  bool,
///     #[get_mut(custom_name_here)]
///     broken2: bool,
/// }
/// ```
/// Sẽ tự động gen:
///
/// ```rust
/// pub fn root(&self) -> &vk::PhysicalDeviceFeature2<'a>
/// {
///     &self.f10
/// }
/// pub fn linked(&self) -> &bool
/// {
///     &self.linked
/// }
/// pub fn broken(&mut self) -> &mut bool
/// {
///     &mut self.broken
/// }
/// pub fn custom_name_here(&mut self) -> &mut bool
/// {
///     &mut self.broken2
/// }
/// ```
#[proc_macro_attribute]
pub fn getter(args: TokenStream, input: TokenStream) -> TokenStream
{
    getter::getter(args, input)
}
