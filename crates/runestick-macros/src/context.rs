use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned as _;
use syn::Lit;
use syn::Meta::*;
use syn::NestedMeta::*;

#[derive(Clone, Copy)]
struct Generate<'a> {
    tokens: &'a Tokens,
    attrs: &'a FieldAttrs,
    protocol: &'a FieldProtocol,
    ident: &'a syn::Ident,
    field: &'a syn::Field,
    field_ident: &'a syn::Ident,
    ty: &'a syn::Type,
    name: &'a syn::LitStr,
}

pub(crate) struct FieldProtocol {
    generate: fn(Generate<'_>) -> TokenStream,
    custom: Option<syn::Path>,
}

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    /// `#[rune(..)]` to generate a protocol function.
    pub(crate) protocols: Vec<FieldProtocol>,
    /// `#[rune(copy)]` to indicate that a field is copy and does not need to be
    /// cloned.
    pub(crate) copy: bool,
}

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct DeriveAttrs {
    /// `#[rune(name = "TypeName")]` to override the default type name.
    pub(crate) name: Option<syn::LitStr>,
    /// `#[rune(module = "...")]`.
    pub(crate) module: Option<syn::Path>,
    /// `#[rune(install_with = "...")]`.
    pub(crate) install_with: Option<syn::Path>,
}

pub(crate) struct Tokens {
    pub(crate) protocol: TokenStream,
    pub(crate) any: TokenStream,
    pub(crate) context_error: TokenStream,
    pub(crate) from_value: TokenStream,
    pub(crate) variant_data: TokenStream,
    pub(crate) hash: TokenStream,
    pub(crate) module: TokenStream,
    pub(crate) named: TokenStream,
    pub(crate) object: TokenStream,
    pub(crate) pointer_guard: TokenStream,
    pub(crate) raw_into_mut: TokenStream,
    pub(crate) raw_into_ref: TokenStream,
    pub(crate) raw_str: TokenStream,
    pub(crate) shared: TokenStream,
    pub(crate) to_value: TokenStream,
    pub(crate) tuple: TokenStream,
    pub(crate) type_info: TokenStream,
    pub(crate) type_of: TokenStream,
    pub(crate) unit_struct: TokenStream,
    pub(crate) unsafe_from_value: TokenStream,
    pub(crate) unsafe_to_value: TokenStream,
    pub(crate) value: TokenStream,
    pub(crate) vm_error_kind: TokenStream,
    pub(crate) vm_error: TokenStream,
    pub(crate) install_with: TokenStream,
}

impl Tokens {
    /// Define a tokenstream for the specified protocol
    pub(crate) fn protocol(&self, sym: Symbol) -> TokenStream {
        let protocol = &self.protocol;
        quote!(#protocol::#sym)
    }
}

#[derive(Default)]
pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) module: Option<TokenStream>,
}

impl Context {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Construct a context with a different default module.
    pub(crate) fn with_module<M>(module: M) -> Self
    where
        M: ToTokens,
    {
        Self {
            errors: Vec::new(),
            module: Some(quote!(#module)),
        }
    }

    pub(crate) fn tokens_with_module(&self, attrs: &DeriveAttrs) -> Tokens {
        let module = &match &attrs.module {
            Some(module) => quote!(#module),
            None => match &self.module {
                Some(module) => module.clone(),
                None => {
                    let runestick = RUNESTICK;
                    quote!(#runestick)
                }
            },
        };

        Tokens {
            protocol: quote!(#module::Protocol),
            any: quote!(#module::Any),
            context_error: quote!(#module::ContextError),
            from_value: quote!(#module::FromValue),
            variant_data: quote!(#module::VariantData),
            hash: quote!(#module::Hash),
            module: quote!(#module::Module),
            named: quote!(#module::Named),
            object: quote!(#module::Object),
            pointer_guard: quote!(#module::SharedPointerGuard),
            raw_into_mut: quote!(#module::RawMut),
            raw_into_ref: quote!(#module::RawRef),
            raw_str: quote!(#module::RawStr),
            shared: quote!(#module::Shared),
            to_value: quote!(#module::ToValue),
            tuple: quote!(#module::Tuple),
            type_info: quote!(#module::TypeInfo),
            type_of: quote!(#module::TypeOf),
            unit_struct: quote!(#module::UnitStruct),
            unsafe_from_value: quote!(#module::UnsafeFromValue),
            unsafe_to_value: quote!(#module::UnsafeToValue),
            value: quote!(#module::Value),
            vm_error_kind: quote!(#module::VmErrorKind),
            vm_error: quote!(#module::VmError),
            install_with: quote!(#module::InstallWith),
        }
    }

    /// Parse the toplevel component of the attribute, which must be `#[rune(..)]`.
    fn get_rune_meta_items(&mut self, attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
        if attr.path != RUNE {
            return Some(Vec::new());
        }

        match attr.parse_meta() {
            Ok(List(meta)) => Some(meta.nested.into_iter().collect()),
            Ok(other) => {
                self.errors
                    .push(syn::Error::new_spanned(other, "expected #[rune(...)]"));
                None
            }
            Err(error) => {
                self.errors.push(syn::Error::new(Span::call_site(), error));
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn parse_field_attrs(&mut self, attrs: &[syn::Attribute]) -> Option<FieldAttrs> {
        macro_rules! generate_op {
            ($proto:ident, $op:tt) => {
                |g| {
                    let Generate {
                        ident,
                        field_ident,
                        ty,
                        name,
                        ..
                    } = g;

                    let protocol = g.tokens.protocol($proto);

                    if let Some(custom) = &g.protocol.custom {
                        quote_spanned! { g.field.span() =>
                            module.field_fn(#protocol, #name, #custom)?;
                        }
                    } else {
                        quote_spanned! { g.field.span() =>
                            module.field_fn(#protocol, #name, |s: &mut #ident, value: #ty| {
                                s.#field_ident $op value;
                            })?;
                        }
                    }
                }
            };
        }

        let mut output = FieldAttrs::default();

        for attr in attrs {
            for meta in self.get_rune_meta_items(attr)? {
                match meta {
                    Meta(Path(path)) if path == COPY => {
                        output.copy = true;
                    }
                    Meta(meta) if meta.path() == GET => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: |g| {
                                let Generate {
                                    ident,
                                    field_ident,
                                    name,
                                    ..
                                } = g;

                                let access = if g.attrs.copy {
                                    quote!(s.#field_ident)
                                } else {
                                    quote!(Clone::clone(&s.#field_ident))
                                };

                                let protocol = g.tokens.protocol(PROTOCOL_GET);

                                quote_spanned! { g.field.span() =>
                                    module.field_fn(#protocol, #name, |s: &#ident| #access)?;
                                }
                            },
                        });
                    }
                    Meta(meta) if meta.path() == SET => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: |g| {
                                let Generate {
                                    ident,
                                    field_ident,
                                    ty,
                                    name,
                                    ..
                                } = g;

                                let protocol = g.tokens.protocol(PROTOCOL_SET);

                                quote_spanned! { g.field.span() =>
                                    module.field_fn(#protocol, #name, |s: &mut #ident, value: #ty| {
                                        s.#field_ident = value;
                                    })?;
                                }
                            },
                        });
                    }
                    Meta(meta) if meta.path() == ADD_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_ADD_ASSIGN, +=),
                        });
                    }
                    Meta(meta) if meta.path() == SUB_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SUB_ASSIGN, -=),
                        });
                    }
                    Meta(meta) if meta.path() == DIV_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_DIV_ASSIGN, /=),
                        });
                    }
                    Meta(meta) if meta.path() == MUL_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_MUL_ASSIGN, *=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_AND_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_AND_ASSIGN, &=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_OR_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_OR_ASSIGN, |=),
                        });
                    }
                    Meta(meta) if meta.path() == BIT_XOR_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_BIT_XOR_ASSIGN, ^=),
                        });
                    }
                    Meta(meta) if meta.path() == SHL_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SHL_ASSIGN, <<=),
                        });
                    }
                    Meta(meta) if meta.path() == SHR_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_SHR_ASSIGN, >>=),
                        });
                    }
                    Meta(meta) if meta.path() == REM_ASSIGN => {
                        output.protocols.push(FieldProtocol {
                            custom: self.parse_field_custom(meta)?,
                            generate: generate_op!(PROTOCOL_REM_ASSIGN, %=),
                        });
                    }
                    _ => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    /// Parse path to custom field function.
    fn parse_field_custom(&mut self, meta: syn::Meta) -> Option<Option<syn::Path>> {
        let s = match meta {
            Path(..) => return Some(None),
            NameValue(syn::MetaNameValue {
                lit: syn::Lit::Str(s),
                ..
            }) => s,
            _ => {
                self.errors
                    .push(syn::Error::new(meta.span(), "unsupported meta"));
                return None;
            }
        };

        match s.parse_with(syn::Path::parse_mod_style) {
            Ok(path) => Some(Some(path)),
            Err(error) => {
                self.errors.push(error);
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn parse_derive_attrs(&mut self, attrs: &[syn::Attribute]) -> Option<DeriveAttrs> {
        let mut output = DeriveAttrs::default();

        for attr in attrs {
            for meta in self.get_rune_meta_items(attr)? {
                match meta {
                    // Parse `#[rune(name = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(name),
                        ..
                    })) if path == NAME => {
                        output.name = Some(name);
                    }
                    // Parse `#[rune(module = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) if path == MODULE => {
                        let module = match s.parse_with(syn::Path::parse_mod_style) {
                            Ok(module) => module,
                            Err(e) => {
                                self.errors.push(e);
                                return None;
                            }
                        };

                        output.module = Some(module);
                    }
                    // Parse `#[rune(install_with = "..")]`.
                    Meta(NameValue(syn::MetaNameValue {
                        path,
                        lit: Lit::Str(s),
                        ..
                    })) if path == INSTALL_WITH => {
                        let install_with = match s.parse_with(syn::Path::parse_mod_style) {
                            Ok(install_with) => install_with,
                            Err(e) => {
                                self.errors.push(e);
                                return None;
                            }
                        };

                        output.install_with = Some(install_with);
                    }
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(output)
    }

    /// Expannd the install into impl.
    pub(crate) fn expand_install_with(
        &mut self,
        input: &syn::DeriveInput,
        tokens: &Tokens,
        attrs: &DeriveAttrs,
    ) -> Option<TokenStream> {
        let mut installers = Vec::new();

        if let Some(install_with) = &attrs.install_with {
            installers.push(quote_spanned! { input.span() =>
                #install_with(module)?;
            });
        }

        let ident = &input.ident;

        match &input.data {
            syn::Data::Struct(st) => {
                for field in &st.fields {
                    let attrs = self.parse_field_attrs(&field.attrs)?;

                    let field_ident = match &field.ident {
                        Some(ident) => ident,
                        None => {
                            if !attrs.protocols.is_empty() {
                                self.errors.push(syn::Error::new_spanned(
                                    field,
                                    "only named fields can be used with protocol generators like `#[rune(get)]`",
                                ));
                                return None;
                            }

                            continue;
                        }
                    };

                    let ty = &field.ty;
                    let name = &syn::LitStr::new(&field_ident.to_string(), field_ident.span());

                    for protocol in &attrs.protocols {
                        installers.push((protocol.generate)(Generate {
                            tokens,
                            protocol,
                            attrs: &attrs,
                            ident,
                            field,
                            field_ident,
                            ty,
                            name,
                        }));
                    }
                }
            }
            syn::Data::Enum(..) => {
                self.errors.push(syn::Error::new_spanned(
                    input,
                    "`Any` not supported on enums",
                ));
                return None;
            }
            syn::Data::Union(..) => {
                self.errors.push(syn::Error::new_spanned(
                    input,
                    "`Any` not supported on unions",
                ));
                return None;
            }
        }

        Some(quote! {
            #(#installers)*
            Ok(())
        })
    }

    /// Expand the necessary implementation details for `Any`.
    pub(super) fn expand_any<T>(
        &self,
        ident: T,
        name: &TokenStream,
        install_with: &TokenStream,
        tokens: &Tokens,
    ) -> Result<TokenStream, Vec<syn::Error>>
    where
        T: Copy + ToTokens,
    {
        let any = &tokens.any;
        let context_error = &tokens.context_error;
        let hash = &tokens.hash;
        let module = &tokens.module;
        let named = &tokens.named;
        let pointer_guard = &tokens.pointer_guard;
        let raw_into_mut = &tokens.raw_into_mut;
        let raw_into_ref = &tokens.raw_into_ref;
        let raw_str = &tokens.raw_str;
        let shared = &tokens.shared;
        let type_info = &tokens.type_info;
        let type_of = &tokens.type_of;
        let unsafe_from_value = &tokens.unsafe_from_value;
        let unsafe_to_value = &tokens.unsafe_to_value;
        let value = &tokens.value;
        let vm_error = &tokens.vm_error;
        let install_into_trait = &tokens.install_with;

        Ok(quote! {
            impl #any for #ident {
                fn type_hash() -> #hash {
                    // Safety: `Hash` asserts that it is layout compatible with `TypeId`.
                    // TODO: remove this once we can have transmute-like functionality in a const fn.
                    #hash::from_type_id(std::any::TypeId::of::<#ident>())
                }
            }

            impl #install_into_trait for #ident {
                fn install_with(module: &mut #module) -> ::std::result::Result<(), #context_error> {
                    #install_with
                }
            }

            impl #named for #ident {
                const NAME: #raw_str = #raw_str::from_str(#name);
            }

            impl #type_of for #ident {
                fn type_hash() -> #hash {
                    <Self as #any>::type_hash()
                }

                fn type_info() -> #type_info {
                    #type_info::Any(<Self as #named>::NAME)
                }
            }

            impl #unsafe_from_value for &#ident {
                type Output = *const #ident;
                type Guard = #raw_into_ref;

                fn from_value(
                    value: #value,
                ) -> ::std::result::Result<(Self::Output, Self::Guard), #vm_error> {
                    value.into_any_ptr()
                }

                unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                    &*output
                }
            }

            impl #unsafe_from_value for &mut #ident {
                type Output = *mut #ident;
                type Guard = #raw_into_mut;

                fn from_value(
                    value: #value,
                ) -> ::std::result::Result<(Self::Output, Self::Guard), #vm_error> {
                    value.into_any_mut()
                }

                unsafe fn unsafe_coerce(output: Self::Output) -> Self {
                    &mut *output
                }
            }

            impl #unsafe_to_value for &#ident {
                type Guard = #pointer_guard;

                unsafe fn unsafe_to_value(self) -> ::std::result::Result<(#value, Self::Guard), #vm_error> {
                    let (shared, guard) = #shared::from_ref(self);
                    Ok((#value::from(shared), guard))
                }
            }

            impl #unsafe_to_value for &mut #ident {
                type Guard = #pointer_guard;

                unsafe fn unsafe_to_value(self) -> ::std::result::Result<(#value, Self::Guard), #vm_error> {
                    let (shared, guard) = #shared::from_mut(self);
                    Ok((#value::from(shared), guard))
                }
            }
        })
    }
}
