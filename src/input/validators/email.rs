use super::hostname;

pub fn validate(value: &str) -> Option<String> {
    if is_email(value) {
        None
    } else {
        Some("must be a valid email address".to_owned())
    }
}

fn is_email(value: &str) -> bool {
    if value.len() > 254 || !value.is_ascii() {
        return false;
    }

    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(is_local_byte)
    {
        return false;
    }

    if let Some(address) = domain
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return address.parse::<std::net::IpAddr>().is_ok();
    }

    hostname::validate(domain).is_none() && domain.contains('.')
}

fn is_local_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'.' | b'!'
                | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_common_email_shape() {
        assert_eq!(validate("person+tag@example.com"), None);
        assert!(validate("person@example").is_some());
        assert!(validate("person..tag@example.com").is_some());
    }
}
