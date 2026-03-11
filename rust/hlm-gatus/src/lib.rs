pub fn crate_name() -> &'static str {
    "hlm-gatus"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn returns_crate_name() {
        assert_eq!(crate_name(), "hlm-gatus");
    }
}
