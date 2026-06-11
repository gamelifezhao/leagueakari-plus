use base64::{Engine, engine::general_purpose::STANDARD};

pub fn basic_auth_header(password: &str) -> String {
    let encoded = STANDARD.encode(format!("riot:{password}"));
    format!("Basic {encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_basic_auth_header() {
        assert_eq!(basic_auth_header("secret"), "Basic cmlvdDpzZWNyZXQ=");
    }
}
