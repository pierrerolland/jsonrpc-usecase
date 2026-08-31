pub fn validate(value: &str) -> Option<String> {
    value
        .parse::<std::net::IpAddr>()
        .is_err()
        .then(|| "must be a valid IPv4 or IPv6 address".to_owned())
}
