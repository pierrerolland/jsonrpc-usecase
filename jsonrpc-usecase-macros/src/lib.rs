use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, FnArg, GenericArgument, ImplItem, ImplItemFn, ItemImpl, Lit, LitStr, Meta, PatType,
    PathArguments, ReturnType, Type, parse_macro_input, punctuated::Punctuated,
};

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn UseCase(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let item_impl = parse_macro_input!(input as ItemImpl);

    match expand_use_case(args, item_impl) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_use_case(
    args: Punctuated<Meta, syn::Token![,]>,
    item_impl: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    if item_impl.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            item_impl.impl_token,
            "UseCase must be applied to an inherent impl block",
        ));
    }

    if !item_impl.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_impl.generics,
            "macro-registered use cases cannot be generic",
        ));
    }

    let self_ty = item_impl.self_ty.as_ref();
    let default_method = pascal_case_method_name(self_ty)?;
    let method = method_name_from_args(args)?.unwrap_or(default_method);
    let method = LitStr::new(&method, proc_macro2::Span::call_site());

    let execute = execute_method(&item_impl)?;
    let input_ty = input_type(execute)?;
    let (output_ty, error_ty) = result_output_types(execute)?;

    Ok(quote! {
        #item_impl

        impl ::jsonrpc_usecase::__private::UseCaseDefinition for #self_ty {
            type Input = #input_ty;
            type Output = #output_ty;
            type Error = #error_ty;

            fn execute(
                &self,
                input: Self::Input,
            ) -> impl ::std::future::Future<Output = ::std::result::Result<Self::Output, Self::Error>> + Send {
                #self_ty::execute(self, input)
            }
        }

        ::jsonrpc_usecase::__private::inventory::submit! {
            ::jsonrpc_usecase::__private::UseCaseRegistration {
                method: #method,
                factory: || {
                    ::std::sync::Arc::new(
                        ::jsonrpc_usecase::__private::UseCaseMethod::from(
                            <#self_ty as ::std::default::Default>::default()
                        )
                    )
                },
            }
        }
    })
}

fn method_name_from_args(args: Punctuated<Meta, syn::Token![,]>) -> syn::Result<Option<String>> {
    let mut method = None;

    for arg in args {
        match arg {
            Meta::NameValue(name_value) if name_value.path.is_ident("method") => {
                let Expr::Lit(expr_lit) = name_value.value else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "expected `method = \"MethodName\"`",
                    ));
                };
                let Lit::Str(literal) = expr_lit.lit else {
                    return Err(syn::Error::new_spanned(
                        expr_lit,
                        "expected `method = \"MethodName\"`",
                    ));
                };

                let value = literal.value();
                if value.is_empty() {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "method name must not be empty",
                    ));
                }
                method = Some(value);
            }
            meta => {
                return Err(syn::Error::new_spanned(
                    meta,
                    "expected `method = \"MethodName\"`",
                ));
            }
        }
    }

    Ok(method)
}

fn execute_method(item_impl: &ItemImpl) -> syn::Result<&ImplItemFn> {
    item_impl
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(method) if method.sig.ident == "execute" => Some(method),
            _ => None,
        })
        .next()
        .ok_or_else(|| {
            syn::Error::new_spanned(
                item_impl,
                "UseCase requires an async `execute(&self, input) -> Result<Output, Error>` method",
            )
        })
}

fn input_type(method: &ImplItemFn) -> syn::Result<Type> {
    if method.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            method.sig.fn_token,
            "execute must be async",
        ));
    }

    if method.sig.inputs.len() != 2 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "execute must take exactly `&self` and one input argument",
        ));
    }

    let mut inputs = method.sig.inputs.iter();
    let receiver = inputs.next().expect("length checked above");
    match receiver {
        FnArg::Receiver(receiver)
            if receiver.reference.is_some() && receiver.mutability.is_none() => {}
        _ => {
            return Err(syn::Error::new_spanned(
                receiver,
                "execute must take `&self` as its first argument",
            ));
        }
    }

    let input = inputs.next().expect("length checked above");
    match input {
        FnArg::Typed(PatType { ty, .. }) => Ok((**ty).clone()),
        FnArg::Receiver(_) => Err(syn::Error::new_spanned(
            input,
            "execute must take one typed input argument after `&self`",
        )),
    }
}

fn result_output_types(method: &ImplItemFn) -> syn::Result<(Type, Type)> {
    let ReturnType::Type(_, return_ty) = &method.sig.output else {
        return Err(syn::Error::new_spanned(
            &method.sig.output,
            "execute must return `Result<Output, Error>`",
        ));
    };

    let Type::Path(type_path) = return_ty.as_ref() else {
        return Err(syn::Error::new_spanned(
            return_ty,
            "execute must return `Result<Output, Error>`",
        ));
    };

    let Some(segment) = type_path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            return_ty,
            "execute must return `Result<Output, Error>`",
        ));
    };

    if segment.ident != "Result" {
        return Err(syn::Error::new_spanned(
            return_ty,
            "execute must return `Result<Output, Error>`",
        ));
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            return_ty,
            "execute must return `Result<Output, Error>`",
        ));
    };

    let mut types = arguments.args.iter().filter_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    });

    let output = types.next().ok_or_else(|| {
        syn::Error::new_spanned(return_ty, "execute must return `Result<Output, Error>`")
    })?;
    let error = types.next().ok_or_else(|| {
        syn::Error::new_spanned(return_ty, "execute must return `Result<Output, Error>`")
    })?;

    Ok((output, error))
}

fn pascal_case_method_name(self_ty: &Type) -> syn::Result<String> {
    let Type::Path(type_path) = self_ty else {
        return Err(syn::Error::new_spanned(
            self_ty,
            "UseCase method names can only be inferred for named structs",
        ));
    };

    let Some(segment) = type_path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            self_ty,
            "UseCase method names can only be inferred for named structs",
        ));
    };

    if !matches!(segment.arguments, PathArguments::None) {
        return Err(syn::Error::new_spanned(
            self_ty,
            "macro-registered use cases cannot be generic",
        ));
    }

    Ok(segment.ident.to_string())
}
