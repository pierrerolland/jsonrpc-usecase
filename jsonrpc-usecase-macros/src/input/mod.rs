use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, LitStr, Meta, PathArguments,
    Type, TypeArray, TypePath, TypeSlice, parse::Parser, punctuated::Punctuated,
};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "UseCaseInput cannot be derived for a generic type",
        ));
    }

    let name = input.ident;
    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "UseCaseInput can only be derived for a struct with named fields",
        ));
    };
    let Fields::Named(fields) = data.fields else {
        return Err(syn::Error::new_spanned(
            name,
            "UseCaseInput requires a struct with named fields",
        ));
    };

    let mut transforms = Vec::new();
    let mut validations = Vec::new();

    for field in fields.named {
        let ident = field.ident.expect("named fields have identifiers");
        let field_name = LitStr::new(&snake_to_camel(unraw(&ident.to_string())), ident.span());
        let transform_rules = attribute_rules(&field.attrs, "transform")?;
        let validation_rules = attribute_rules(&field.attrs, "validate")?;
        let field_access = quote!(&mut self.#ident);

        for rule in &transform_rules {
            transforms.push(expand_transform(rule, field_access.clone(), &field.ty)?);
        }
        for rule in &validation_rules {
            transforms.push(expand_nested_transform(
                rule,
                field_access.clone(),
                &field.ty,
            )?);
        }

        let field_access = quote!(&self.#ident);
        for rule in &validation_rules {
            validations.push(expand_validation(
                rule,
                field_access.clone(),
                &field.ty,
                quote!(#field_name),
            )?);
        }
    }

    Ok(quote! {
        impl ::jsonrpc_usecase::UseCaseInput for #name {
            fn transform(&mut self) {
                #(#transforms)*
            }

            fn validate(
                &self,
            ) -> ::std::result::Result<(), ::jsonrpc_usecase::InputValidationErrors> {
                let mut __jsonrpc_usecase_errors =
                    ::jsonrpc_usecase::InputValidationErrors::new();
                #(#validations)*
                __jsonrpc_usecase_errors.into_result()
            }
        }

        const _: () = {
            fn input_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<#name>()
            }

            fn process_input(
                input: &mut dyn ::std::any::Any,
            ) -> ::std::result::Result<(), ::jsonrpc_usecase::InputValidationErrors> {
                let input = input
                    .downcast_mut::<#name>()
                    .expect("registered use-case input must match its processor");
                <#name as ::jsonrpc_usecase::UseCaseInput>::process(input)
            }

            ::jsonrpc_usecase::__private::inventory::submit! {
                ::jsonrpc_usecase::__private::InputProcessorRegistration::new(
                    input_type_id,
                    process_input,
                )
            }
        };
    })
}

fn attribute_rules(attributes: &[syn::Attribute], name: &str) -> syn::Result<Vec<Meta>> {
    let mut rules = Vec::new();
    for attribute in attributes {
        if attribute.path().is_ident(name) {
            rules.extend(
                attribute.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)?,
            );
        }
    }
    Ok(rules)
}

fn expand_transform(rule: &Meta, value: TokenStream, ty: &Type) -> syn::Result<TokenStream> {
    if let Some(inner) = wrapper_inner(ty, "Option") {
        let body = expand_transform_non_optional(rule, quote!(__jsonrpc_usecase_value), inner)?;
        return Ok(quote! {
            {
                let __jsonrpc_usecase_option = #value;
                if let ::std::option::Option::Some(__jsonrpc_usecase_value) =
                    __jsonrpc_usecase_option.as_mut()
                {
                    #body
                }
            }
        });
    }

    expand_transform_non_optional(rule, value, ty)
}

fn expand_transform_non_optional(
    rule: &Meta,
    value: TokenStream,
    ty: &Type,
) -> syn::Result<TokenStream> {
    let (name, arguments) = meta_parts(rule)?;
    let name_string = name.to_string();

    if name_string == "each" {
        let rules = nested_rules(arguments, "each")?;
        let inner = collection_inner(ty)
            .ok_or_else(|| syn::Error::new_spanned(ty, "`each` requires Vec<T>, [T; N], or [T]"))?;
        let mut bodies = Vec::new();
        for rule in &rules {
            bodies.push(expand_transform(
                rule,
                quote!(__jsonrpc_usecase_item),
                inner,
            )?);
        }
        return Ok(quote! {
            {
                let __jsonrpc_usecase_collection = #value;
                for __jsonrpc_usecase_item in (__jsonrpc_usecase_collection).iter_mut() {
                    #(#bodies)*
                }
            }
        });
    }

    let module = format_ident!("{}", name_string);
    let call = match name_string.as_str() {
        "trim"
        | "trim_start"
        | "trim_end"
        | "lowercase"
        | "uppercase"
        | "capitalize"
        | "titlecase"
        | "collapse_whitespace"
        | "remove_whitespace"
        | "normalize_nfc"
        | "normalize_nfkc"
        | "abs"
        | "floor"
        | "ceil"
        | "sort"
        | "dedup" => {
            require_no_arguments(rule, arguments, &name_string)?;
            quote!(::jsonrpc_usecase::__private::transformers::#module::transform(#value);)
        }
        "replace" => {
            let arguments = named_arguments(arguments, "replace")?;
            let from = required_argument(&arguments, "from", rule)?;
            let to = required_argument(&arguments, "to", rule)?;
            reject_unknown_arguments(&arguments, &["from", "to"], rule)?;
            quote!(::jsonrpc_usecase::__private::transformers::replace::transform(
                #value, #from, #to,
            );)
        }
        "clamp" => {
            let arguments = named_arguments(arguments, "clamp")?;
            let minimum = required_argument(&arguments, "min", rule)?;
            let maximum = required_argument(&arguments, "max", rule)?;
            reject_unknown_arguments(&arguments, &["min", "max"], rule)?;
            quote!(::jsonrpc_usecase::__private::transformers::clamp::transform(
                #value, #minimum, #maximum,
            );)
        }
        "round" => {
            if matches!(rule, Meta::NameValue(_)) {
                return Err(syn::Error::new_spanned(
                    rule,
                    "use `round(precision = n)` to configure precision",
                ));
            }
            let arguments = named_arguments(arguments, "round")?;
            let precision = optional_argument(&arguments, "precision")
                .map(|value| quote!(#value))
                .unwrap_or_else(|| quote!(0));
            reject_unknown_arguments(&arguments, &["precision"], rule)?;
            quote!(::jsonrpc_usecase::__private::transformers::round::transform(
                #value, #precision,
            );)
        }
        "custom" => {
            let function = if let Meta::NameValue(name_value) = rule {
                name_value.value.clone()
            } else {
                let arguments = named_arguments(arguments, "custom")?;
                let function = required_argument(&arguments, "function", rule)?.clone();
                reject_unknown_arguments(&arguments, &["function"], rule)?;
                function
            };
            quote!(::jsonrpc_usecase::__private::transformers::custom::transform(
                #value, #function,
            );)
        }
        "truncate" | "strip_prefix" | "strip_suffix" => {
            let argument = single_value_argument(rule, arguments, &name_string)?;
            quote!(::jsonrpc_usecase::__private::transformers::#module::transform(
                #value, #argument,
            );)
        }
        unknown => {
            return Err(syn::Error::new_spanned(
                rule,
                format!("unknown input transformer `{unknown}`"),
            ));
        }
    };

    Ok(call)
}

fn expand_nested_transform(rule: &Meta, value: TokenStream, ty: &Type) -> syn::Result<TokenStream> {
    let (name, arguments) = meta_parts(rule)?;
    if name == "nested" {
        require_no_arguments(rule, arguments, "nested")?;
        return wrap_optional_transform(value, ty, |value, _| {
            Ok(quote!(
                ::jsonrpc_usecase::__private::transformers::nested::transform(#value);
            ))
        });
    }
    if name != "each" {
        return Ok(TokenStream::new());
    }

    let rules = nested_rules(arguments, "each")?;
    let collection_ty = wrapper_inner(ty, "Option").unwrap_or(ty);
    let inner = collection_inner(collection_ty)
        .ok_or_else(|| syn::Error::new_spanned(ty, "`each` requires Vec<T>, [T; N], or [T]"))?;
    let mut bodies = Vec::new();
    for rule in &rules {
        bodies.push(expand_nested_transform(
            rule,
            quote!(__jsonrpc_usecase_item),
            inner,
        )?);
    }
    if bodies.iter().all(TokenStream::is_empty) {
        return Ok(TokenStream::new());
    }

    wrap_optional_transform(value, ty, |value, _| {
        Ok(quote! {
            for __jsonrpc_usecase_item in (#value).iter_mut() {
                #(#bodies)*
            }
        })
    })
}

fn wrap_optional_transform(
    value: TokenStream,
    ty: &Type,
    body: impl FnOnce(TokenStream, &Type) -> syn::Result<TokenStream>,
) -> syn::Result<TokenStream> {
    if let Some(inner) = wrapper_inner(ty, "Option") {
        let body = body(quote!(__jsonrpc_usecase_value), inner)?;
        Ok(quote! {
            {
                let __jsonrpc_usecase_option = #value;
                if let ::std::option::Option::Some(__jsonrpc_usecase_value) =
                    __jsonrpc_usecase_option.as_mut()
                {
                    #body
                }
            }
        })
    } else {
        body(value, ty)
    }
}

fn expand_validation(
    rule: &Meta,
    value: TokenStream,
    ty: &Type,
    path: TokenStream,
) -> syn::Result<TokenStream> {
    let (name, _) = meta_parts(rule)?;
    if name == "required" {
        if wrapper_inner(ty, "Option").is_none() {
            return Err(syn::Error::new_spanned(
                rule,
                "`required` can only be used on Option<T>",
            ));
        }
        return expand_validation_non_optional(rule, value, ty, path);
    }

    if let Some(inner) = wrapper_inner(ty, "Option") {
        let body =
            expand_validation_non_optional(rule, quote!(__jsonrpc_usecase_value), inner, path)?;
        return Ok(quote! {
            {
                let __jsonrpc_usecase_option = #value;
                if let ::std::option::Option::Some(__jsonrpc_usecase_value) =
                    __jsonrpc_usecase_option.as_ref()
                {
                    #body
                }
            }
        });
    }

    expand_validation_non_optional(rule, value, ty, path)
}

fn expand_validation_non_optional(
    rule: &Meta,
    value: TokenStream,
    ty: &Type,
    path: TokenStream,
) -> syn::Result<TokenStream> {
    let (name, arguments) = meta_parts(rule)?;
    let name_string = name.to_string();

    if name_string == "each" {
        let rules = nested_rules(arguments, "each")?;
        let inner = collection_inner(ty)
            .ok_or_else(|| syn::Error::new_spanned(ty, "`each` requires Vec<T>, [T; N], or [T]"))?;
        let mut bodies = Vec::new();
        for rule in &rules {
            bodies.push(expand_validation(
                rule,
                quote!(__jsonrpc_usecase_item),
                inner,
                quote!(__jsonrpc_usecase_item_path.as_str()),
            )?);
        }
        return Ok(quote! {
            for (__jsonrpc_usecase_index, __jsonrpc_usecase_item) in
                (#value).iter().enumerate()
            {
                let __jsonrpc_usecase_item_path =
                    ::std::format!("{}[{}]", #path, __jsonrpc_usecase_index);
                #(#bodies)*
            }
        });
    }

    let (arguments, custom_message) = validator_arguments(arguments, &name_string, rule)?;

    if name_string == "nested" {
        if custom_message.is_some() {
            return Err(syn::Error::new_spanned(
                rule,
                "`nested` preserves child messages and does not accept `message`",
            ));
        }
        reject_unknown_arguments(&arguments, &[], rule)?;
        return Ok(quote! {
            if let ::std::result::Result::Err(__jsonrpc_usecase_nested_errors) =
                ::jsonrpc_usecase::__private::validators::nested::validate(#value)
            {
                __jsonrpc_usecase_errors.extend_prefixed(
                    #path,
                    __jsonrpc_usecase_nested_errors,
                );
            }
        });
    }

    if name_string == "custom" {
        let function = required_argument(&arguments, "function", rule)?;
        let rule_name = optional_argument(&arguments, "name")
            .map(|name| quote!(#name))
            .unwrap_or_else(|| quote!("custom"));
        reject_unknown_arguments(&arguments, &["function", "name"], rule)?;
        let call = quote!(::jsonrpc_usecase::__private::validators::custom::validate(
            #value, |__jsonrpc_usecase_custom_value| {
                (#function)(__jsonrpc_usecase_custom_value)
            },
        ));
        return Ok(validation_violation(call, path, rule_name, custom_message));
    }

    let module = format_ident!("{}", name_string);
    let call = match name_string.as_str() {
        "required" | "not_empty" | "not_blank" | "email" | "url" | "uuid" | "ip" | "ipv4"
        | "ipv6" | "hostname" | "slug" | "ascii" | "alphabetic" | "alphanumeric" | "numeric"
        | "lowercase" | "uppercase" | "hex" | "base64" | "base64_url" | "positive" | "negative"
        | "non_positive" | "non_negative" | "finite" | "integer" | "even" | "odd" | "unique" => {
            reject_unknown_arguments(&arguments, &[], rule)?;
            quote!(::jsonrpc_usecase::__private::validators::#module::validate(#value))
        }
        "contains" | "starts_with" | "ends_with" | "regex" | "min" | "max" | "equals"
        | "not_equals" => {
            let argument = required_argument(&arguments, "value", rule)?;
            reject_unknown_arguments(&arguments, &["value"], rule)?;
            if matches!(
                name_string.as_str(),
                "contains" | "starts_with" | "ends_with" | "regex" | "equals" | "not_equals"
            ) {
                quote!(::jsonrpc_usecase::__private::validators::#module::validate(
                    #value, #argument,
                ))
            } else {
                quote!(::jsonrpc_usecase::__private::validators::#module::validate(
                    #value, &(#argument),
                ))
            }
        }
        "multiple_of" => {
            let argument = required_argument(&arguments, "value", rule)?;
            reject_unknown_arguments(&arguments, &["value"], rule)?;
            quote!(::jsonrpc_usecase::__private::validators::multiple_of::validate(
                #value, #argument,
            ))
        }
        "range" => {
            let minimum = required_argument(&arguments, "min", rule)?;
            let maximum = required_argument(&arguments, "max", rule)?;
            reject_unknown_arguments(&arguments, &["min", "max"], rule)?;
            quote!(::jsonrpc_usecase::__private::validators::range::validate(
                #value, &(#minimum), &(#maximum),
            ))
        }
        "length" => {
            let has_minimum = optional_argument(&arguments, "min").is_some();
            let has_maximum = optional_argument(&arguments, "max").is_some();
            let has_exact = optional_argument(&arguments, "exact").is_some();
            let minimum = optional_argument(&arguments, "min")
                .map(|value| quote!(::std::option::Option::Some(#value)))
                .unwrap_or_else(|| quote!(::std::option::Option::None));
            let maximum = optional_argument(&arguments, "max")
                .map(|value| quote!(::std::option::Option::Some(#value)))
                .unwrap_or_else(|| quote!(::std::option::Option::None));
            let exact = optional_argument(&arguments, "exact")
                .map(|value| quote!(::std::option::Option::Some(#value)))
                .unwrap_or_else(|| quote!(::std::option::Option::None));
            if !has_minimum && !has_maximum && !has_exact {
                return Err(syn::Error::new_spanned(
                    rule,
                    "`length` requires `min`, `max`, or `exact`",
                ));
            }
            if has_exact && (has_minimum || has_maximum) {
                return Err(syn::Error::new_spanned(
                    rule,
                    "`length(exact = ...)` cannot also use `min` or `max`",
                ));
            }
            reject_unknown_arguments(&arguments, &["min", "max", "exact"], rule)?;
            quote!(::jsonrpc_usecase::__private::validators::length::validate(
                #value, #minimum, #maximum, #exact,
            ))
        }
        "one_of" => {
            let values = required_argument(&arguments, "values", rule)?;
            reject_unknown_arguments(&arguments, &["values"], rule)?;
            quote!(::jsonrpc_usecase::__private::validators::one_of::validate(
                #value, &(#values),
            ))
        }
        unknown => {
            return Err(syn::Error::new_spanned(
                rule,
                format!("unknown input validator `{unknown}`"),
            ));
        }
    };

    Ok(validation_violation(
        call,
        path,
        quote!(#name_string),
        custom_message,
    ))
}

fn validation_violation(
    call: TokenStream,
    path: TokenStream,
    rule_name: TokenStream,
    custom_message: Option<LitStr>,
) -> TokenStream {
    let message = custom_message
        .map(|message| quote!(#message.to_owned()))
        .unwrap_or_else(|| quote!(__jsonrpc_usecase_message));

    quote! {
        if let ::std::option::Option::Some(__jsonrpc_usecase_message) = #call {
            __jsonrpc_usecase_errors.push(
                ::jsonrpc_usecase::InputViolation::new(
                    ::std::string::ToString::to_string(&(#path)),
                    #rule_name,
                    #message,
                ),
            );
        }
    }
}

fn meta_parts(meta: &Meta) -> syn::Result<(&Ident, Option<&proc_macro2::TokenStream>)> {
    match meta {
        Meta::Path(path) => path
            .get_ident()
            .map(|ident| (ident, None))
            .ok_or_else(|| syn::Error::new_spanned(path, "rule names must be single identifiers")),
        Meta::List(list) => list
            .path
            .get_ident()
            .map(|ident| (ident, Some(&list.tokens)))
            .ok_or_else(|| {
                syn::Error::new_spanned(&list.path, "rule names must be single identifiers")
            }),
        Meta::NameValue(name_value) => name_value
            .path
            .get_ident()
            .map(|ident| (ident, None))
            .ok_or_else(|| {
                syn::Error::new_spanned(&name_value.path, "rule names must be single identifiers")
            }),
    }
}

fn nested_rules(
    arguments: Option<&proc_macro2::TokenStream>,
    name: &str,
) -> syn::Result<Vec<Meta>> {
    let Some(arguments) = arguments else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("`{name}` requires one or more nested rules"),
        ));
    };
    let rules = Punctuated::<Meta, syn::Token![,]>::parse_terminated.parse2(arguments.clone())?;
    if rules.is_empty() {
        return Err(syn::Error::new_spanned(
            arguments,
            format!("`{name}` must not be empty"),
        ));
    }
    Ok(rules.into_iter().collect())
}

#[derive(Clone)]
struct NamedArgument {
    name: String,
    value: Expr,
}

fn named_arguments(
    arguments: Option<&proc_macro2::TokenStream>,
    rule: &str,
) -> syn::Result<Vec<NamedArgument>> {
    let Some(arguments) = arguments else {
        return Ok(Vec::new());
    };
    let metas = Punctuated::<Meta, syn::Token![,]>::parse_terminated.parse2(arguments.clone())?;
    let mut parsed = Vec::new();
    for meta in metas {
        let Meta::NameValue(name_value) = meta else {
            return Err(syn::Error::new_spanned(
                meta,
                format!("`{rule}` arguments must use `name = value`"),
            ));
        };
        let Some(name) = name_value.path.get_ident() else {
            return Err(syn::Error::new_spanned(
                name_value.path,
                "argument names must be identifiers",
            ));
        };
        if parsed
            .iter()
            .any(|argument: &NamedArgument| name == argument.name.as_str())
        {
            return Err(syn::Error::new_spanned(
                name,
                format!("duplicate `{name}` argument"),
            ));
        }
        parsed.push(NamedArgument {
            name: name.to_string(),
            value: name_value.value,
        });
    }
    Ok(parsed)
}

fn validator_arguments(
    raw: Option<&proc_macro2::TokenStream>,
    rule_name: &str,
    rule: &Meta,
) -> syn::Result<(Vec<NamedArgument>, Option<LitStr>)> {
    let mut arguments = if let Meta::NameValue(name_value) = rule {
        vec![NamedArgument {
            name: match rule_name {
                "one_of" => "values".to_owned(),
                "custom" => "function".to_owned(),
                _ => "value".to_owned(),
            },
            value: name_value.value.clone(),
        }]
    } else {
        named_arguments(raw, rule_name)?
    };

    let message_index = arguments
        .iter()
        .position(|argument| argument.name == "message");
    let message = if let Some(index) = message_index {
        let argument = arguments.remove(index);
        let Expr::Lit(expression) = argument.value else {
            return Err(syn::Error::new_spanned(
                argument.value,
                "`message` must be a string literal",
            ));
        };
        let Lit::Str(message) = expression.lit else {
            return Err(syn::Error::new_spanned(
                expression,
                "`message` must be a string literal",
            ));
        };
        Some(message)
    } else {
        None
    };

    Ok((arguments, message))
}

fn single_value_argument(
    rule: &Meta,
    raw: Option<&proc_macro2::TokenStream>,
    name: &str,
) -> syn::Result<Expr> {
    if let Meta::NameValue(name_value) = rule {
        return Ok(name_value.value.clone());
    }
    let arguments = named_arguments(raw, name)?;
    let value = required_argument(&arguments, "value", rule)?.clone();
    reject_unknown_arguments(&arguments, &["value"], rule)?;
    Ok(value)
}

fn required_argument<'a>(
    arguments: &'a [NamedArgument],
    name: &str,
    span: &impl quote::ToTokens,
) -> syn::Result<&'a Expr> {
    optional_argument(arguments, name)
        .ok_or_else(|| syn::Error::new_spanned(span, format!("missing required `{name}` argument")))
}

fn optional_argument<'a>(arguments: &'a [NamedArgument], name: &str) -> Option<&'a Expr> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| &argument.value)
}

fn reject_unknown_arguments(
    arguments: &[NamedArgument],
    allowed: &[&str],
    span: &impl quote::ToTokens,
) -> syn::Result<()> {
    if let Some(argument) = arguments
        .iter()
        .find(|argument| !allowed.contains(&argument.name.as_str()))
    {
        return Err(syn::Error::new_spanned(
            span,
            format!("unknown `{}` argument", argument.name),
        ));
    }
    Ok(())
}

fn require_no_arguments(
    rule: &Meta,
    arguments: Option<&proc_macro2::TokenStream>,
    name: &str,
) -> syn::Result<()> {
    if matches!(rule, Meta::NameValue(_))
        || arguments.is_some_and(|arguments| !arguments.is_empty())
    {
        return Err(syn::Error::new_spanned(
            rule,
            format!("`{name}` does not accept arguments"),
        ));
    }
    Ok(())
}

fn wrapper_inner<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return None;
    };
    let segment = path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

fn collection_inner(ty: &Type) -> Option<&Type> {
    wrapper_inner(ty, "Vec").or(match ty {
        Type::Array(TypeArray { elem, .. }) | Type::Slice(TypeSlice { elem, .. }) => Some(elem),
        _ => None,
    })
}

fn snake_to_camel(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut uppercase_next = false;
    for character in input.chars() {
        if character == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            output.push(character.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            output.push(character);
        }
    }
    output
}

fn unraw(input: &str) -> &str {
    input.strip_prefix("r#").unwrap_or(input)
}
