//! Version ordering (supports SemVer pre-release tags like `0.3.3-preview1`).

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedVersion {
    pub main: Vec<u32>,
    pub prerelease: Option<String>,
}

impl PartialOrd for ParsedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ParsedVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 1. Compare main version numbers ([0, 3, 3] vs [0, 3, 2])
        match self.main.cmp(&other.main) {
            std::cmp::Ordering::Equal => {}
            other_ord => return other_ord,
        }
        // 2. Same main version: official release (None) is newer than prerelease (Some)
        match (&self.prerelease, &other.prerelease) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater, // 0.3.3 > 0.3.3-preview1
            (Some(_), None) => std::cmp::Ordering::Less,    // 0.3.3-preview1 < 0.3.3
            (Some(a), Some(b)) => a.cmp(b),                 // 0.3.3-preview2 > 0.3.3-preview1
        }
    }
}

pub fn parse_version(version_text: &str) -> Option<ParsedVersion> {
    let mut s = version_text.trim();
    if s.to_ascii_lowercase().starts_with('v') {
        s = &s[1..];
    }
    let mut parts = s.splitn(2, '-');
    let main_str = parts.next()?;
    let prerelease = parts.next().map(|p| p.to_ascii_lowercase());

    let mut main = Vec::new();
    for part in main_str.split('.') {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        main.push(part.parse().ok()?);
    }
    if main.is_empty() {
        return None;
    }
    Some(ParsedVersion { main, prerelease })
}

pub fn parse_version_tuple(version_text: &str) -> Option<Vec<u32>> {
    parse_version(version_text).map(|v| v.main)
}

pub fn is_newer_version(remote_tag: &str, local_version: &str) -> bool {
    let remote = parse_version(remote_tag);
    let local = parse_version(local_version);
    match (remote, local) {
        (Some(r), Some(l)) => r > l,
        _ => {
            let r_str = remote_tag.trim().to_ascii_lowercase();
            let l_str = format!("v{}", local_version.trim().to_ascii_lowercase());
            r_str != l_str
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newer_semver() {
        assert!(is_newer_version("v0.1.7", "0.1.6"));
        assert!(!is_newer_version("v0.1.6", "0.1.6"));

        // 0.3.3-preview1 은 0.3.2 보다 최신 버전임 (0.3.2 업데이트 방지)
        assert!(!is_newer_version("v0.3.2", "0.3.3-preview1"));
        assert!(!is_newer_version("0.3.2", "0.3.3-preview1"));

        // 0.3.3 정식 출시본은 0.3.3-preview1 보다 최신임
        assert!(is_newer_version("v0.3.3", "0.3.3-preview1"));

        // 0.3.3-preview2 는 0.3.3-preview1 보다 최신임
        assert!(is_newer_version("v0.3.3-preview2", "0.3.3-preview1"));

        // 0.3.4 는 0.3.3-preview1 보다 최신임
        assert!(is_newer_version("v0.3.4", "0.3.3-preview1"));
    }
}
