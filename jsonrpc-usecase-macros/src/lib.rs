use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, FnArg, GenericArgument, ImplItem, ImplItemFn, Item, ItemFn, ItemImpl, ItemStruct, Lit,
    LitStr, Meta, PatType, PathArguments, ReturnType, Type, TypePath, parse_macro_input,
    punctuated::Punctuated,
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

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn UseCaseEventConsumer(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let item = parse_macro_input!(input as Item);

    match expand_event_consumer(args, item) {
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
    let use_case_name = use_case_name(self_ty)?;
    let args = use_case_args_from_args(args)?;
    let method = args.method.unwrap_or_else(|| use_case_name.clone());
    let method = LitStr::new(&method, proc_macro2::Span::call_site());
    let guards = args.guards;
    let will_event = LitStr::new(
        &format!("Will{use_case_name}"),
        proc_macro2::Span::call_site(),
    );
    let did_event = LitStr::new(
        &format!("Did{use_case_name}"),
        proc_macro2::Span::call_site(),
    );

    let execute = execute_method(&item_impl)?;
    let input_ty = input_type(execute)?;
    let (output_ty, error_ty) = result_output_types(execute)?;

    Ok(quote! {
        #item_impl

        impl ::jsonrpc_usecase::__private::UseCaseDefinition for #self_ty {
            type Input = #input_ty;
            type Output = #output_ty;
            type Error = #error_ty;

            const WILL_EVENT: &'static str = #will_event;
            const DID_EVENT: &'static str = #did_event;

            fn can_proceed(context: &::jsonrpc_usecase::__private::GuardContext) -> bool {
                true #(&& <#guards as ::jsonrpc_usecase::__private::Guard>::can_proceed(
                    &<#guards as ::std::default::Default>::default(),
                    context,
                ))*
            }

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

fn expand_event_consumer(
    args: Punctuated<Meta, syn::Token![,]>,
    item: Item,
) -> syn::Result<proc_macro2::TokenStream> {
    let event = event_name_from_args(args)?;

    match item {
        Item::Fn(item_fn) => expand_event_consumer_fn(event, item_fn),
        Item::Struct(item_struct) => expand_event_consumer_struct(event, item_struct),
        Item::Impl(item_impl) => expand_event_consumer_impl(event, item_impl),
        item => Err(syn::Error::new_spanned(
            item,
            "UseCaseEventConsumer must be applied to a struct, function, or inherent impl block",
        )),
    }
}

fn expand_event_consumer_fn(
    event: LitStr,
    item_fn: ItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    validate_event_consumer_fn_signature(&item_fn)?;
    let consumer = &item_fn.sig.ident;
    let trampoline = format_ident!("__jsonrpc_usecase_event_consumer_{}", consumer);
    let item = quote! {
        #item_fn

        #[allow(non_snake_case)]
        fn #trampoline<'a>(
            event: &'a ::jsonrpc_usecase::UseCaseEvent,
        ) -> ::jsonrpc_usecase::__private::UseCaseEventConsumerFuture<'a> {
            ::std::boxed::Box::pin(async move {
                #consumer(event).await
            })
        }
    };

    Ok(event_consumer_registration(
        event,
        item,
        quote!(#trampoline),
    ))
}

fn expand_event_consumer_struct(
    event: LitStr,
    item_struct: ItemStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    if !item_struct.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_struct.generics,
            "event consumers cannot be generic",
        ));
    }

    let consumer = &item_struct.ident;
    let trampoline = format_ident!("__jsonrpc_usecase_event_consumer_{}_consume", consumer);
    let item = quote! {
        #item_struct

        #[allow(non_snake_case)]
        fn #trampoline<'a>(
            event: &'a ::jsonrpc_usecase::UseCaseEvent,
        ) -> ::jsonrpc_usecase::__private::UseCaseEventConsumerFuture<'a> {
            ::std::boxed::Box::pin(async move {
                <#consumer as ::std::default::Default>::default().consume(event).await
            })
        }
    };

    Ok(event_consumer_registration(
        event,
        item,
        quote!(#trampoline),
    ))
}

fn expand_event_consumer_impl(
    event: LitStr,
    item_impl: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    if item_impl.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            item_impl.impl_token,
            "UseCaseEventConsumer must be applied to an inherent impl block",
        ));
    }

    if !item_impl.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_impl.generics,
            "event consumers cannot be generic",
        ));
    }

    let self_ty = item_impl.self_ty.as_ref();
    let consumer_name = use_case_name(self_ty)?;
    let consume = consume_method(&item_impl)?;
    validate_event_consumer_method_signature(consume)?;
    let consume_ident = &consume.sig.ident;
    let trampoline = format_ident!(
        "__jsonrpc_usecase_event_consumer_{}_{}",
        consumer_name,
        consume_ident
    );
    let item = quote! {
        #item_impl

        #[allow(non_snake_case)]
        fn #trampoline<'a>(
            event: &'a ::jsonrpc_usecase::UseCaseEvent,
        ) -> ::jsonrpc_usecase::__private::UseCaseEventConsumerFuture<'a> {
            ::std::boxed::Box::pin(async move {
                <#self_ty as ::std::default::Default>::default().#consume_ident(event).await
            })
        }
    };

    Ok(event_consumer_registration(
        event,
        item,
        quote!(#trampoline),
    ))
}

fn event_consumer_registration(
    event: LitStr,
    item: proc_macro2::TokenStream,
    consumer: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #item

        ::jsonrpc_usecase::__private::inventory::submit! {
            ::jsonrpc_usecase::__private::UseCaseEventConsumerRegistration {
                event: #event,
                consumer: #consumer,
            }
        }
    }
}

struct UseCaseArgs {
    method: Option<String>,
    guards: Vec<Type>,
}

fn use_case_args_from_args(args: Punctuated<Meta, syn::Token![,]>) -> syn::Result<UseCaseArgs> {
    let mut method = None;
    let mut guards = Vec::new();

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
            Meta::NameValue(name_value) if name_value.path.is_ident("guards") => {
                let Expr::Array(array) = name_value.value else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "expected `guards = [GuardType, ...]`",
                    ));
                };

                for guard in array.elems {
                    let Expr::Path(expr_path) = guard else {
                        return Err(syn::Error::new_spanned(guard, "guards must be type paths"));
                    };

                    guards.push(Type::Path(TypePath {
                        qself: expr_path.qself,
                        path: expr_path.path,
                    }));
                }
            }
            meta => {
                return Err(syn::Error::new_spanned(
                    meta,
                    "expected `method = \"MethodName\"` or `guards = [GuardType, ...]`",
                ));
            }
        }
    }

    Ok(UseCaseArgs { method, guards })
}

fn event_name_from_args(args: Punctuated<Meta, syn::Token![,]>) -> syn::Result<LitStr> {
    let mut event = None;

    for arg in args {
        match arg {
            Meta::NameValue(name_value) if name_value.path.is_ident("event") => {
                let Expr::Lit(expr_lit) = name_value.value else {
                    return Err(syn::Error::new_spanned(
                        name_value,
                        "expected `event = \"EventName\"`",
                    ));
                };
                let Lit::Str(literal) = expr_lit.lit else {
                    return Err(syn::Error::new_spanned(
                        expr_lit,
                        "expected `event = \"EventName\"`",
                    ));
                };

                if literal.value().is_empty() {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "event name must not be empty",
                    ));
                }
                event = Some(literal);
            }
            meta => {
                return Err(syn::Error::new_spanned(
                    meta,
                    "expected `event = \"EventName\"`",
                ));
            }
        }
    }

    event.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected `event = \"EventName\"`",
        )
    })
}

fn validate_event_consumer_fn_signature(item_fn: &ItemFn) -> syn::Result<()> {
    if !item_fn.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.generics,
            "event consumers cannot be generic",
        ));
    }

    if item_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            item_fn.sig.fn_token,
            "event consumers must be async functions",
        ));
    }

    if item_fn.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.inputs,
            "event consumers must take exactly one `&UseCaseEvent` argument",
        ));
    }

    match &item_fn.sig.output {
        ReturnType::Default => Ok(()),
        ReturnType::Type(..) => Err(syn::Error::new_spanned(
            &item_fn.sig.output,
            "event consumers must not return a value",
        )),
    }
}

fn consume_method(item_impl: &ItemImpl) -> syn::Result<&ImplItemFn> {
    item_impl
        .items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Fn(method) if method.sig.ident == "consume" => Some(method),
            _ => None,
        })
        .next()
        .ok_or_else(|| {
            syn::Error::new_spanned(
                item_impl,
                "UseCaseEventConsumer requires a `consume(&self, event: &UseCaseEvent)` method",
            )
        })
}

fn validate_event_consumer_method_signature(method: &ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            method.sig.fn_token,
            "event consumers must be async functions",
        ));
    }

    if method.sig.inputs.len() != 2 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "consume must take exactly `&self` and one `&UseCaseEvent` argument",
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
                "consume must take `&self` as its first argument",
            ));
        }
    }

    match &method.sig.output {
        ReturnType::Default => Ok(()),
        ReturnType::Type(..) => Err(syn::Error::new_spanned(
            &method.sig.output,
            "event consumers must not return a value",
        )),
    }
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

fn use_case_name(self_ty: &Type) -> syn::Result<String> {
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
