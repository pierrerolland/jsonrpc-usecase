pub fn validate(value: &str) -> Option<String> {
    value
        .parse::<std::net::Ipv6Addr>()
        .is_err()
        .then(|| "must be a valid IPv6 address".to_owned())
}
