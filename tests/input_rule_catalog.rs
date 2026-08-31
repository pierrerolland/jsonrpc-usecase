use jsonrpc_usecase::UseCaseInput;

#[allow(dead_code)]
#[derive(UseCaseInput)]
struct StringValidatorCatalog {
    #[validate(
        not_empty,
        not_blank,
        length(min = 1, max = 255),
        email(message = "custom email message"),
        url,
        regex = "^.+$",
        uuid,
        ip,
        ipv4,
        ipv6,
        hostname,
        slug,
        ascii,
        alphabetic,
        alphanumeric,
        numeric,
        lowercase,
        uppercase,
        hex,
        base64,
        base64_url,
        contains = "a",
        starts_with = "a",
        ends_with = "z",
        equals = "value",
        not_equals = "other",
        one_of = ["first", "second"]
    )]
    value: String,

    #[validate(length(exact = 3))]
    exact_length: String,
}

#[allow(dead_code)]
#[derive(UseCaseInput)]
struct NumberValidatorCatalog {
    #[validate(
        min = 1,
        max = 100,
        range(min = 1, max = 100),
        positive,
        non_negative,
        multiple_of = 2,
        integer,
        even,
        odd,
        equals = 42,
        not_equals = 41,
        one_of = [1, 2, 3]
    )]
    unsigned: u32,

    #[validate(negative, non_positive)]
    signed: i64,

    #[validate(finite, integer, min = -100.0, max = 100.0, multiple_of = 0.5)]
    float: f64,
}

#[allow(dead_code)]
#[derive(UseCaseInput)]
struct TransformerCatalog {
    #[transform(
        trim,
        trim_start,
        trim_end,
        lowercase,
        uppercase,
        capitalize,
        titlecase,
        collapse_whitespace,
        remove_whitespace,
        replace(from = "a", to = "b"),
        strip_prefix = "prefix",
        strip_suffix = "suffix",
        truncate = 64,
        normalize_nfc,
        normalize_nfkc
    )]
    text: String,

    #[transform(clamp(min = -10.0, max = 10.0), abs, round(precision = 2), floor, ceil)]
    float: f64,

    #[transform(round)]
    whole_float: f32,

    #[transform(each(trim, lowercase), sort, dedup)]
    #[validate(
        not_empty,
        length(min = 1, max = 8),
        unique,
        each(not_blank, length(max = 20))
    )]
    values: Vec<String>,
}

#[derive(UseCaseInput)]
struct OptionalInput {
    #[transform(trim, lowercase)]
    #[validate(required, email)]
    email: Option<String>,
}

fn add_custom_prefix(value: &mut String) {
    value.insert_str(0, "rpc:");
}

fn require_custom_prefix(value: &str) -> Result<(), &'static str> {
    value
        .starts_with("rpc:")
        .then_some(())
        .ok_or("must start with the custom prefix")
}

#[derive(UseCaseInput)]
struct CustomRuleInput {
    #[transform(custom = add_custom_prefix)]
    #[validate(custom(function = require_custom_prefix, name = "rpc_prefix"))]
    value: String,

    #[validate(custom = require_custom_prefix)]
    already_prefixed: String,
}

#[test]
fn optional_rules_skip_none_except_for_required() {
    let mut input = OptionalInput { email: None };
    let errors = input.process().unwrap_err();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors.violations()[0].rule(), "required");
}

#[test]
fn optional_values_are_transformed_then_validated() {
    let mut input = OptionalInput {
        email: Some("  PERSON@EXAMPLE.COM ".to_owned()),
    };

    assert_eq!(input.process(), Ok(()));
    assert_eq!(input.email.as_deref(), Some("person@example.com"));
}

#[test]
fn custom_validation_messages_are_supported() {
    #[derive(UseCaseInput)]
    struct CustomMessageInput {
        #[validate(email(message = "please provide a deliverable address"))]
        email: String,
    }

    let input = CustomMessageInput {
        email: "invalid".to_owned(),
    };
    let errors = input.validate().unwrap_err();

    assert_eq!(
        errors.violations()[0].message(),
        "please provide a deliverable address"
    );
}

#[test]
fn custom_transformers_and_validators_are_supported() {
    let mut input = CustomRuleInput {
        value: "method".to_owned(),
        already_prefixed: "rpc:ready".to_owned(),
    };

    assert_eq!(input.process(), Ok(()));
    assert_eq!(input.value, "rpc:method");

    input.value = "method".to_owned();
    let errors = input.validate().unwrap_err();
    assert_eq!(errors.violations()[0].rule(), "rpc_prefix");
    assert_eq!(
        errors.violations()[0].message(),
        "must start with the custom prefix"
    );
}
