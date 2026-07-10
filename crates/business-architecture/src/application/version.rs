use semver::Version;

pub fn bump_minor(current: &str) -> Result<String, semver::Error> {
    let v = Version::parse(current)?;
    let (major, minor, patch) = (v.major, v.minor, v.patch);
    Ok(Version::new(major, minor + 1, patch).to_string())
}

pub fn parse(s: &str) -> Result<Version, semver::Error> {
    Version::parse(s)
}
