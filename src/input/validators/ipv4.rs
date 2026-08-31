pub fn validate(value: &str) -> Option<String> {
    value
        .parse::<std::net::Ipv4Addr>()
        .is_err()
        .then(|| "must be a valid IPv4 address".to_owned())
}
