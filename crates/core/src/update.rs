//! In-app updates: looks for a newer release, downloads its setup, checks the
//! minisign signature against the key built into this program and starts the
//! setup. The release publishes `latest.json` (same format as the Tauri
//! updater, so older versions can update to this one) next to the setup.

use std::path::PathBuf;

use serde::Deserialize;

use crate::proc::hidden;
use crate::remote::is_newer;

/// Where the latest release lists its setup and signature.
const ENDPOINT: &str = "https://github.com/Niklas970/faderust/releases/latest/download/latest.json";

/// Every run of the "Test build" workflow publishes a pre-release `test-<branch>`
/// (replacing the previous one of that branch); the app lists them to choose from.
const RELEASES: &str = "https://api.github.com/repos/Niklas970/faderust/releases?per_page=50";
const TEST_PREFIX: &str = "test-";

/// A test build to choose from: the pre-release of one branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBuild {
    /// The release tag, `test-<branch>`.
    pub tag: String,
}

impl std::fmt::Display for TestBuild {
    /// The branch name, as shown in the list.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag.strip_prefix(TEST_PREFIX).unwrap_or(&self.tag))
    }
}

/// The test builds in a `releases` listing of the GitHub API.
pub fn parse_test_list(json: &str) -> Result<Vec<TestBuild>, String> {
    let list: Vec<serde_json::Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(list
        .iter()
        .filter(|r| r["prerelease"].as_bool() == Some(true))
        .filter_map(|r| r["tag_name"].as_str())
        .filter(|t| t.starts_with(TEST_PREFIX))
        .map(|t| TestBuild { tag: t.into() })
        .collect())
}

/// Asks GitHub which test builds exist.
pub fn list_tests() -> Result<Vec<TestBuild>, String> {
    let body = fetch(&["--max-time", "20", RELEASES])?;
    parse_test_list(&String::from_utf8_lossy(&body))
}

/// Identifies this build for the test channel ("<run>-<commit>", set by the
/// test workflow); empty for every other build.
pub fn own_build() -> &'static str {
    option_env!("FADERUST_BUILD").unwrap_or("")
}

/// The release signing key, as the Tauri updater stores it: the public key
/// file, base64 encoded.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDg2QkYyMEVDMUZBNjkzNkEKUldScWs2WWY3Q0MvaHBVK25RbzlRMXBaUXN0NTJPVTcvTW5YTWt4aWNtOFRhdU1vT0pCU0N0R20K";

const PLATFORM: &str = "windows-x86_64";

/// A newer release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    /// Set for a test build: which one ("<run>-<commit>").
    pub build: String,
    url: String,
    /// The `.sig` file's content (base64 of the minisign signature).
    signature: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    /// Only the test channel's manifest has one.
    #[serde(default)]
    build: String,
    platforms: std::collections::HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

impl Update {
    /// What the update button says: the version, plus the build for a test.
    pub fn label(&self) -> String {
        if self.build.is_empty() {
            self.version.clone()
        } else {
            format!("{} Test {}", self.version, self.build)
        }
    }
}

/// The update `json` offers over `current`, if it is newer and has a setup
/// for this platform.
pub fn parse_manifest(json: &str, current: &str) -> Result<Option<Update>, String> {
    let m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if !is_newer(m.version.trim_start_matches('v'), current) {
        return Ok(None);
    }
    offer(m)
}

/// The test build `json` offers, if it is another build than `own`.
pub fn parse_test_manifest(json: &str, own: &str) -> Result<Option<Update>, String> {
    let m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if m.build.is_empty() || m.build == own {
        return Ok(None);
    }
    offer(m)
}

fn offer(m: Manifest) -> Result<Option<Update>, String> {
    let p = m
        .platforms
        .get(PLATFORM)
        .ok_or_else(|| format!("no setup for {PLATFORM}"))?;
    Ok(Some(Update {
        version: m.version.trim_start_matches('v').to_string(),
        build: m.build.clone(),
        url: p.url.clone(),
        signature: p.signature.clone(),
    }))
}

fn fetch(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = hidden("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-time",
            "600",
        ])
        .args(args)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Asks GitHub for the latest release; `Ok(None)` when this one is current.
/// With a test build's tag, a newer release still wins, otherwise that test
/// build is offered when it is another build than this one.
pub fn check(current: &str, test: Option<&str>) -> Result<Option<Update>, String> {
    let body = fetch(&["--max-time", "20", ENDPOINT])?;
    let release = parse_manifest(&String::from_utf8_lossy(&body), current)?;
    let Some(tag) = test.filter(|t| release.is_none() && t.starts_with(TEST_PREFIX)) else {
        return Ok(release);
    };
    let url =
        format!("https://github.com/Niklas970/faderust/releases/download/{tag}/latest-test.json");
    let body = fetch(&["--max-time", "20", &url])?;
    parse_test_manifest(&String::from_utf8_lossy(&body), own_build())
}

fn b64_text(s: &str) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn verify_with(
    key: &minisign_verify::PublicKey,
    data: &[u8],
    signature: &str,
) -> Result<(), String> {
    let sig =
        minisign_verify::Signature::decode(&b64_text(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &sig, true)
        .map_err(|e| format!("Signatur ungültig / bad signature: {e}"))
}

/// Whether `data` carries a valid signature of the release key (`signature`
/// as in the manifest).
pub fn verify(data: &[u8], signature: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&b64_text(PUBKEY)?).map_err(|e| e.to_string())?;
    verify_with(&key, data, signature)
}

fn setup_path(version: &str) -> PathBuf {
    std::env::temp_dir()
        .join("faderust-update")
        .join(format!("Faderust_{version}_x64-setup.exe"))
}

/// Downloads the setup, checks its signature and starts it silently; it
/// replaces the files and starts Faderust again. The caller quits right after
/// (the setup stops a still running Faderust anyway).
pub fn install(update: &Update) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Updates gibt es nur unter Windows / Updates are Windows only".into());
    }
    let data = fetch(&[&update.url])?;
    verify(&data, &update.signature)?;
    let path = setup_path(&update.version);
    std::fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    hidden(&path)
        .args(["/S", "/UPDATE"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{"version":"0.7.0","pub_date":"x","notes":"","platforms":{"windows-x86_64":{"signature":"c2ln","url":"https://example.invalid/setup.exe"}}}"#;

    #[test]
    fn a_newer_manifest_is_an_update() {
        let u = parse_manifest(MANIFEST, "0.6.4").unwrap().unwrap();
        assert_eq!(u.version, "0.7.0");
        assert_eq!(u.url, "https://example.invalid/setup.exe");
    }

    #[test]
    fn a_test_build_is_offered_when_it_is_another_build() {
        let json = MANIFEST.replace(
            "\"version\":\"0.7.0\"",
            "\"version\":\"0.7.0\",\"build\":\"12-abc1234\"",
        );
        let u = parse_test_manifest(&json, "11-def5678").unwrap().unwrap();
        assert_eq!(u.label(), "0.7.0 Test 12-abc1234");
        assert!(parse_test_manifest(&json, "12-abc1234").unwrap().is_none());
        // A manifest of a release has no build: never a test build.
        assert!(parse_test_manifest(MANIFEST, "").unwrap().is_none());
    }

    #[test]
    fn test_builds_are_the_prereleases_named_test() {
        let json = r#"[{"tag_name":"v0.8.3","prerelease":false},{"tag_name":"test-claude-x","prerelease":true},{"tag_name":"test-main","prerelease":true},{"tag_name":"test-fake","prerelease":false}]"#;
        let list = parse_test_list(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].to_string(), "claude-x");
        assert_eq!(list[1].tag, "test-main");
    }

    #[test]
    fn the_same_or_an_older_version_is_not() {
        assert!(parse_manifest(MANIFEST, "0.7.0").unwrap().is_none());
        assert!(parse_manifest(MANIFEST, "0.8.1").unwrap().is_none());
    }

    #[test]
    fn a_manifest_without_this_platform_is_an_error() {
        let json = MANIFEST.replace("windows-x86_64", "linux-x86_64");
        assert!(parse_manifest(&json, "0.6.4").is_err());
    }

    #[test]
    fn a_good_signature_verifies_and_a_changed_file_does_not() {
        use base64::Engine as _;
        // Test vector of the minisign-verify crate (pre-hashed, file "test").
        let key = minisign_verify::PublicKey::from_base64(
            "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3",
        )
        .unwrap();
        let sig = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1633700835\tfile:test\tprehashed\nwLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";
        let encoded = base64::engine::general_purpose::STANDARD.encode(sig);
        assert!(verify_with(&key, b"test", &encoded).is_ok());
        assert!(verify_with(&key, b"tampered", &encoded).is_err());
        // Signed by another key: the release key refuses it.
        assert!(verify(b"test", &encoded).is_err());
    }

    #[test]
    fn garbage_does_not_verify() {
        assert!(verify(b"setup", "bm90IGEgc2lnbmF0dXJl").is_err());
        assert!(verify(b"setup", "!!!").is_err());
    }
}
