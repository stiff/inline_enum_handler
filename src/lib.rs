//! A procedural macro library for implementing enum-based command dispatch in Rust.
//!
//! This crate provides two macros that work together to generate a unified dispatch
//! method from tagged impl block methods:
//! - `#[with_enum_handlers(EnumType)]` — generates a dispatch method that matches
//!   on the enum type and delegates to the appropriate handler
//! - `#[handle(VariantType)]` — marks a method as a handler for a specific enum variant
//!
//! # Features
//!
//! - **Automatic handler collection**: Methods tagged with `#[handle(...)]` are automatically
//!   collected and used to generate a `match` statement in the dispatch method
//! - **Custom dispatch names**: Use `dispatch = custom_name` to name the generated method
//! - **Multiple dispatch methods**: Multiple dispatch methods can be generated from the same
//!   impl block, each handling a different enum type
//! - **Async support**: Handlers can be async, and the generated dispatch method will
//!   automatically become async if any handler is async
//! - **Immutable detection**: The dispatch signature uses `&self` when all handlers are
//!   immutable, or `&mut self` if any handler requires mutation
//!
//! # Example
//!
//! ```ignore
//! use enum_dispatch_derive::{handle, with_enum_handlers};
//!
//! enum Message {
//!     Get(Sender<i32>),
//!     Set(i32),
//! }
//!
//! struct State {
//!     value: Cell<i32>,
//! }
//!
//! #[with_enum_handlers(Message)]
//! impl State {
//!     #[handle(Message::Get)]
//!     async fn get(&self, tx: Sender<i32>) {
//!         tx.send(self.value.get()).await;
//!     }
//!
//!     #[handle(Message::Set)]
//!     fn set(&mut self, value: i32) {
//!         self.value.set(value);
//!     }
//! }
//! // Generates: pub async fn dispatch(&mut self, cmd: Message) { match cmd { ... } }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ImplItemFn, ItemImpl, ImplItem, Type, parse_macro_input, parse::Parse, parse::ParseStream, token};

/// Parsed representation of the `#[with_enum_handlers(...)]` attribute.
///
/// Extracts the enum type and optional custom dispatch method name from the attribute syntax:
/// `EnumType` or `EnumType, dispatch = custom_name`
struct EnumHandlersAttr {
    /// The enum type to match on in the generated dispatch method
    enum_name: Type,
    /// The name of the generated dispatch method (defaults to `dispatch`)
    dispatch_name: Ident,
}

impl Parse for EnumHandlersAttr {
    /// Parse the attribute input in the form:
    /// `EnumType` or `EnumType, dispatch = custom_name`
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let enum_name: Type = input.parse()?;

        let mut dispatch_name = Ident::new("dispatch", input.span());

        if input.peek(token::Comma) {
            input.parse::<token::Comma>()?;
            let attr_ident: Ident = input.parse()?;
            if attr_ident == "dispatch" {
                input.parse::<syn::Token![=]>()?;
                dispatch_name = input.parse()?;
            } else {
                return Err(syn::Error::new(attr_ident.span(), "Expected 'dispatch'"));
            }
        }

        Ok(EnumHandlersAttr {
            enum_name,
            dispatch_name,
        })
    }
}

/// Marks a method as a handler for a specific enum variant.
///
/// The argument should be the full variant path (e.g., `MyEnum::Variant`).
/// The macro itself does nothing — it's metadata for `#[with_enum_handlers]`.
///
/// # Syntax
///
/// ```ignore
/// #[handle(MyEnum::VariantName)]
/// fn handle_method(&mut self, ...params...) -> ReturnType { ... }
/// ```
#[proc_macro_attribute]
pub fn handle(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Generates a dispatch method that routes enum variants to their handlers.
///
/// This macro scans the impl block for methods tagged with `#[handle(VariantType)]`,
/// collects them, and generates a public dispatch method that matches on the enum
/// and calls the appropriate handler.
///
/// # Syntax
///
/// ```ignore
/// #[with_enum_handlers(EnumType, dispatch = method_name)]
/// impl MyStruct {
///     #[handle(EnumType::Variant1)]
///     fn handler1(&self, ...) -> ReturnType { ... }
///
///     #[handle(EnumType::Variant2)]
///     fn handler2(&mut self, ...) -> ReturnType { ... }
/// }
/// ```
///
/// # Attributes
///
/// | Argument | Description | Required |
/// |----------|-------------|----------|
/// | `EnumType` | The enum type to match on | Yes |
/// | `dispatch = name` | Custom name for the generated method | No (defaults to `dispatch`) |
#[proc_macro_attribute]
pub fn with_enum_handlers(attr: TokenStream, input: TokenStream) -> TokenStream {
    // 1. Parse the attribute to extract enum name and optional dispatch name
    let EnumHandlersAttr { enum_name, dispatch_name } = parse_macro_input!(attr with EnumHandlersAttr::parse);

    // 2. Parse the impl block into a structured AST
    let mut input_impl = parse_macro_input!(input as ItemImpl);

    // Collect handler metadata: (variant_type, method_ident, params, is_async, is_immutable)
    let mut handlers = Vec::new();
    let mut return_type = syn::ReturnType::Default;
    let mut any_async = false;
    let mut all_immutable = true;

    // 2. Iterate over items and extract #[handle(Type)] annotations,
    //    filtering to only handlers that belong to this enum
    for item in input_impl.items.iter() {
        if let ImplItem::Fn(method) = item {
            for attr in &method.attrs {
                if attr.path().is_ident("handle") {
                    // Parse the argument inside #[handle(...)]
                    if let Ok(variant_type) = attr.parse_args::<Type>() {
                        // Only include handlers that belong to this enum
                        if !handler_belongs_to_enum(&variant_type, &enum_name) {
                            continue;
                        }

                        // Extract method parameters (ignoring self)
                        let params: Vec<_> = method
                            .sig
                            .inputs
                            .iter()
                            .filter_map(|input| {
                                if let FnArg::Typed(pat) = input {
                                    Some(pat.pat.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let is_async = method.sig.asyncness.is_some();
                        let is_immutable = is_immutable_handler(method);
                        any_async |= is_async;
                        all_immutable &= is_immutable;
                        handlers.push((variant_type, method.sig.ident.clone(), params, is_async));
                        return_type = method.sig.output.clone();
                    } else {
                        return syn::Error::new_spanned(attr, "Invalid #[handle(Type)] syntax")
                            .into_compile_error()
                            .into();
                    }
                }
            }
        }
    }

    // 3. Generate the dispatch method with unified return type
    let receiver = if all_immutable { quote! { &self } } else { quote! { &mut self } };
    let match_arms = handlers.iter().map(|(variant, method, params, is_async)| {
        if is_unit_path_variant(variant) && params.is_empty() {
            // Unit variant: Message::Get (no parentheses)
            if *is_async {
                quote! {
                    #variant => return self.#method().await,
                }
            } else {
                quote! {
                    #variant => return self.#method(),
                }
            }
        } else {
            // Tuple variant: Message::Inc(value)
            if *is_async {
                quote! {
                    #variant(#(#params),*) => return self.#method(#(#params),*).await,
                }
            } else {
                quote! {
                    #variant(#(#params),*) => return self.#method(#(#params),*),
                }
            }
        }
    });

    let async_keyword = if any_async { quote! { async } } else { quote! {} };

    let dispatch_fn = quote! {
        pub #async_keyword fn #dispatch_name(#receiver, cmd: #enum_name) #return_type {
            match cmd {
                #(#match_arms)*
            }
        }
    };

    // 4. Inject the generated method into the impl block
    let dispatch_item: ImplItem =
        syn::parse2(dispatch_fn).expect("Generated dispatch is valid syntax");
    input_impl.items.push(dispatch_item);

    // 5. Emit the complete impl block as a TokenStream
    quote! { #input_impl }.into()
}

/// Check if a type is a path variant (e.g., `Message::Get`) with no arguments.
///
/// Used to determine whether to generate a match arm with or without parentheses
/// around the variant arguments.
fn is_unit_path_variant(ty: &Type) -> bool {
    matches!(ty, Type::Path(_))
}

/// Extract the root identifier from a type path (e.g., `ApiMessage` from `ApiMessage::Inc`).
///
/// Returns `None` if the type is not a path type or has no segments.
fn get_type_root_ident(ty: &Type) -> Option<&Ident> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first() {
            return Some(&segment.ident);
        }
    None
}

/// Check if a handler's variant type belongs to the specified enum.
///
/// Compares the root identifier of the variant type (e.g., `ApiMessage` from `ApiMessage::Inc`)
/// with the enum type from the attribute to filter handlers for the correct enum.
fn handler_belongs_to_enum(variant_type: &Type, enum_type: &Type) -> bool {
    let variant_root = get_type_root_ident(variant_type);
    let enum_root = get_type_root_ident(enum_type);
    match (variant_root, enum_root) {
        (Some(v), Some(e)) => v == e,
        _ => false,
    }
}

/// Check if a handler uses immutable self (`&self`) vs mutable self (`&mut self`).
///
/// Returns `true` if the receiver argument is a reference without `mut`, indicating
/// an immutable borrow. Used to determine whether the dispatch method should use
/// `&self` or `&mut self` as its receiver.
fn is_immutable_handler(method: &ImplItemFn) -> bool {
    if let Some(FnArg::Receiver(receiver)) = method.sig.inputs.first() {
        // &self has reference with mutability=None, &mut self has reference with mutability=Some(_)
        receiver.reference.is_some() && receiver.mutability.is_none()
    } else {
        false
    }
}
