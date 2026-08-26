use semver::Version;

pub fn bump_minor(current: &str) -> Result<String, semver::Error> {
    let v = Version::parse(current)?;
    let (major, minor, patch) = (v.major, v.minor, v.patch);
    Ok(Version::new(major, minor + 1, patch).to_string())
}

pub fn parse(s: &str) -> Result<Version, semver::Error> {
    Version::parse(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_plain_semver() {
        assert!(parse("1.0.0").is_ok());
        assert!(parse("2.1.3").is_ok());
    }

    #[test]
    fn parse_rejects_v_prefix() {
        // "v1.0" and "v2.0" are not valid semver — the backend must reject them
        // so the stored business_version is always a plain "1.0.0" format.
        assert!(parse("v1.0").is_err());
        assert!(parse("v2.0").is_err());
        assert!(parse("v1.0.0").is_err());
    }

    #[test]
    fn parse_rejects_two_component() {
        // "1.0" (no patch) is also rejected by strict semver
        assert!(parse("1.0").is_err());
    }

    #[test]
    fn bump_minor_produces_plain_semver() {
        let next = bump_minor("1.0.0").unwrap();
        assert_eq!(next, "1.1.0");
    }
}
