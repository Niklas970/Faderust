//! In-app updates: looks for a newer release, downloads its setup, checks the
//! minisign signature against the key built into this program and starts the
//! setup. The release publishes `latest.json` (same format as the Tauri
//! updater, so older versions can update to this one) next to the setup.

use std::path::PathBuf;

use serde::Deserialize;

use crate::proc::hidden;
use crate::remote::is_newer;

/// Where the latest release lists its setup and signature.
const RELEASE_API: &str = "https://api.github.com/repos/Niklas970/faderust/releases/latest";
const ENDPOINT: &str = "https://github.com/Niklas970/faderust/releases/latest/download/latest.json";

/// Every run of the "Test build" workflow pushes the build to this package
/// (GitHub container registry) under the tag `<branch>`, replacing the
/// previous one of that branch. A real release deletes the package again.
const REGISTRY: &str = "https://ghcr.io/v2/niklas970/faderust-test";
const TOKEN_URL: &str =
    "https://ghcr.io/token?service=ghcr.io&scope=repository:niklas970/faderust-test:pull";
const MANIFEST_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
const TITLE: &str = "org.opencontainers.image.title";

/// A test build to choose from: the tag of one branch in the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBuild {
    pub tag: String,
    /// Its sub-version, e.g. "0.8.5.2" (empty if unknown).
    pub name: String,
    /// What the build is about: the title of its last commit (empty if unknown).
    pub title: String,
}

impl std::fmt::Display for TestBuild {
    /// "0.8.5.2 · title", as shown in the list; the branch name when the
    /// build has neither.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.name.is_empty(), self.title.is_empty()) {
            (true, true) => f.write_str(&self.tag),
            (false, true) => f.write_str(&self.name),
            (true, false) => f.write_str(&self.title),
            (false, false) => write!(f, "{} · {}", self.name, self.title),
        }
    }
}

/// The tags of a registry `tags/list` answer.
pub fn parse_tag_list(json: &str) -> Result<Vec<TestBuild>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(v["tags"]
        .as_array()
        .map(|a| a.iter().filter_map(|t| t.as_str()))
        .into_iter()
        .flatten()
        .map(|t| TestBuild {
            tag: t.into(),
            name: String::new(),
            title: String::new(),
        })
        .collect())
}

/// An anonymous pull token of the (public) package.
fn registry_token() -> Result<String, String> {
    let body = fetch(&["--max-time", "20", TOKEN_URL])?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    v["token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "no registry token".to_string())
}

fn registry_get(token: &str, path: &str, accept: bool) -> Result<Vec<u8>, String> {
    let auth = format!("Authorization: Bearer {token}");
    let url = format!("{REGISTRY}/{path}");
    let mut args = vec!["--max-time", "20", "-H", &auth];
    let accept_header = format!("Accept: {MANIFEST_TYPE}");
    if accept {
        args.extend(["-H", &accept_header]);
    }
    args.push(&url);
    fetch(&args)
}

/// Asks the registry which test builds exist.
pub fn list_tests() -> Result<Vec<TestBuild>, String> {
    let token = registry_token()?;
    let body = registry_get(&token, "tags/list", false)?;
    let mut list = parse_tag_list(&String::from_utf8_lossy(&body))?;
    // A few builds at most: one small request each for the title.
    for b in list.iter_mut().take(20) {
        if let Ok(m) = registry_get(&token, &format!("manifests/{}", b.tag), true) {
            let json = String::from_utf8_lossy(&m);
            b.title = parse_note(&json, "title");
            b.name = parse_note(&json, "name");
        }
    }
    Ok(list)
}

/// An annotation of a test build's manifest (empty if missing).
fn parse_note(json: &str, key: &str) -> String {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["annotations"][key].as_str().map(|t| t.trim().to_string()))
        .unwrap_or_default()
}

/// A test build's manifest: which build it is and where its files are.
#[derive(Debug, PartialEq, Eq)]
struct OciBuild {
    version: String,
    build: String,
    name: String,
    setup: String,
    sig: String,
}

fn parse_oci(json: &str) -> Result<OciBuild, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let layer = |suffix: &str| {
        v["layers"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|l| {
                l["annotations"][TITLE]
                    .as_str()
                    .is_some_and(|t| t.ends_with(suffix))
            })
            .and_then(|l| l["digest"].as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("test build without {suffix}"))
    };
    let note = |k: &str| v["annotations"][k].as_str().unwrap_or("").to_string();
    Ok(OciBuild {
        version: note("version"),
        build: note("build"),
        name: note("name"),
        setup: layer("-setup.exe")?,
        sig: layer("-setup.exe.sig")?,
    })
}

/// Identifies this build for the test channel ("<run>-<commit>", set by the
/// test workflow); empty for every other build.
pub fn own_build() -> &'static str {
    option_env!("FADERUST_BUILD").unwrap_or("")
}

/// The sub-version of this test build ("0.8.5.3"); empty for every other build.
pub fn own_name() -> &'static str {
    option_env!("FADERUST_BUILD_NAME").unwrap_or("")
}

/// The version to show: the release version, or the test build's sub-version.
pub fn own_version() -> &'static str {
    match own_name() {
        "" => env!("CARGO_PKG_VERSION"),
        name => name,
    }
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
    /// Sub-version of a test build ("0.8.6.4"), empty otherwise.
    pub name: String,
    url: String,
    /// The setup lies in the test package: the download needs a pull token.
    registry: bool,
    /// The `.sig` file's content (base64 of the minisign signature).
    signature: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
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
        } else if self.name.is_empty() {
            format!("Test {}", self.version)
        } else {
            format!("Test {}", self.name)
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

fn offer(m: Manifest) -> Result<Option<Update>, String> {
    let p = m
        .platforms
        .get(PLATFORM)
        .ok_or_else(|| format!("no setup for {PLATFORM}"))?;
    Ok(Some(Update {
        version: m.version.trim_start_matches('v').to_string(),
        build: String::new(),
        name: String::new(),
        url: p.url.clone(),
        registry: false,
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
            // A connection that does not come up, or a download that stalls,
            // fails (and is tried again) instead of hanging for minutes.
            "--connect-timeout",
            "15",
            "--speed-limit",
            "5000",
            "--speed-time",
            "30",
            "--retry",
            "2",
            "--retry-delay",
            "2",
            // On stderr, so a failure names the status and the host that
            // answered (a 403 from GitHub, a proxy or a filter).
            "--write-out",
            "%{stderr}[%{http_code} %{url_effective}]",
        ])
        .args(args)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        let text = String::from_utf8_lossy(&out.stderr);
        // The URL may carry a signed query: keep scheme and host only.
        Err(shorten_urls(text.trim()))
    }
}

/// `https://host/path?query` becomes `https://host/…` in a message.
fn shorten_urls(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("https://") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail
            .find(|c: char| c.is_whitespace() || c == ']')
            .unwrap_or(tail.len());
        let url = &tail[..end];
        let host_end = url[8..].find('/').map_or(url.len(), |k| k + 8);
        out.push_str(&url[..host_end]);
        if host_end < url.len() {
            out.push_str("/…");
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// The release manifest: the direct link first, then through the release
/// list of the API when that is refused.
fn release_manifest() -> Result<Vec<u8>, String> {
    let direct = fetch(&["--max-time", "20", ENDPOINT]);
    let first = match direct {
        Ok(b) => return Ok(b),
        Err(e) => e,
    };
    let api = || -> Result<Vec<u8>, String> {
        let list = fetch(&[
            "--max-time",
            "20",
            "-H",
            "Accept: application/vnd.github+json",
            RELEASE_API,
        ])?;
        let v: serde_json::Value = serde_json::from_slice(&list).map_err(|e| e.to_string())?;
        let url = v["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|a| a["name"] == "latest.json"))
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or_else(|| "no latest.json in the release".to_string())?
            .to_string();
        fetch(&["--max-time", "20", &url])
    };
    api().map_err(|second| format!("{first}; {second}"))
}

/// Asks GitHub for the latest release; `Ok(None)` when this one is current.
/// With a test build's tag, a newer release still wins, otherwise that test
/// build is offered when it is another build than this one.
pub fn check(current: &str, test: Option<&str>) -> Result<Option<Update>, String> {
    let body = release_manifest()?;
    let release = parse_manifest(&String::from_utf8_lossy(&body), current)?;
    let Some(tag) = test.filter(|t| release.is_none() && !t.is_empty()) else {
        return Ok(release);
    };
    let token = registry_token()?;
    let m = registry_get(&token, &format!("manifests/{tag}"), true)?;
    let b = parse_oci(&String::from_utf8_lossy(&m))?;
    if b.build.is_empty() || b.build == own_build() {
        return Ok(None);
    }
    let sig = registry_get(&token, &format!("blobs/{}", b.sig), false)?;
    Ok(Some(Update {
        version: b.version,
        build: b.build,
        name: b.name,
        url: format!("{REGISTRY}/blobs/{}", b.setup),
        registry: true,
        signature: String::from_utf8_lossy(&sig).trim().to_string(),
    }))
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

/// Where the setup of `update` is kept. A test build has the version of its
/// release, so its build is part of the name.
fn setup_path(update: &Update) -> PathBuf {
    let build = if update.build.is_empty() {
        String::new()
    } else {
        format!("_{}", update.build)
    };
    std::env::temp_dir()
        .join("faderust-update")
        .join(format!("Faderust_{}{build}_x64-setup.exe", update.version))
}

/// Downloads the setup and checks its signature.
fn download(update: &Update) -> Result<Vec<u8>, String> {
    let data = if update.registry {
        let auth = format!("Authorization: Bearer {}", registry_token()?);
        fetch(&["-H", &auth, &update.url])?
    } else {
        fetch(&[&update.url])?
    };
    verify(&data, &update.signature)?;
    Ok(data)
}

/// The setup of `update` if it was downloaded before and is still intact.
fn cached(update: &Update) -> Option<PathBuf> {
    let path = setup_path(update);
    let data = std::fs::read(&path).ok()?;
    verify(&data, &update.signature).ok()?;
    Some(path)
}

/// Downloads the setup of `update` ahead of time, so that installing it
/// needs no network. Nothing happens when it is there already.
pub fn prefetch(update: &Update) -> Result<(), String> {
    if cached(update).is_some() {
        return Ok(());
    }
    let data = download(update)?;
    let path = setup_path(update);
    let dir = path.parent().unwrap_or(&path);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // Older downloads are of no use any more.
    for old in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let _ = std::fs::remove_file(old.path());
    }
    let part = path.with_extension("part");
    std::fs::write(&part, &data).map_err(|e| e.to_string())?;
    std::fs::rename(&part, &path).map_err(|e| e.to_string())
}

/// Starts the setup silently (downloading it first unless [`prefetch`]
/// did); it replaces the files and starts Faderust again. The caller quits
/// right after (the setup stops a still running Faderust anyway).
pub fn install(update: &Update) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Updates gibt es nur unter Windows / Updates are Windows only".into());
    }
    prefetch(update)?;
    hidden(setup_path(update))
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
    fn an_error_names_the_host_but_not_the_query() {
        let m = shorten_urls(
            "curl: (22) The requested URL returned error: 403 [403 https://objects.example.com/a/b?X-Sig=secret]",
        );
        assert!(m.contains("[403 https://objects.example.com/…]"));
        assert!(!m.contains("secret"));
    }

    #[test]
    fn a_newer_manifest_is_an_update() {
        let u = parse_manifest(MANIFEST, "0.6.4").unwrap().unwrap();
        assert_eq!(u.version, "0.7.0");
        assert_eq!(u.url, "https://example.invalid/setup.exe");
    }

    #[test]
    fn update_button_names_the_test_build() {
        let mut u = Update {
            version: "0.8.6".into(),
            build: "12-abc1234".into(),
            name: "0.8.6.4".into(),
            url: String::new(),
            registry: true,
            signature: String::new(),
        };
        assert_eq!(u.label(), "Test 0.8.6.4");
        u.name.clear();
        assert_eq!(u.label(), "Test 0.8.6");
        u.build.clear();
        assert_eq!(u.label(), "0.8.6");
    }

    #[test]
    fn test_builds_are_the_tags_of_the_package() {
        let list =
            parse_tag_list(r#"{"name":"niklas970/faderust-test","tags":["claude-x","main"]}"#)
                .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].to_string(), "claude-x");
        assert_eq!(list[1].tag, "main");
        assert_eq!(list[1].to_string(), "main");
        let titled = TestBuild {
            tag: "main".into(),
            name: "0.8.5.2".into(),
            title: "Fenster merkt sich die Position".into(),
        };
        assert_eq!(
            titled.to_string(),
            "0.8.5.2 · Fenster merkt sich die Position"
        );
        assert_eq!(
            parse_note(r#"{"annotations":{"title":" Neu "}}"#, "title"),
            "Neu"
        );
        assert_eq!(parse_note("{}", "title"), "");
        // A package without tags answers with null.
        assert!(parse_tag_list(r#"{"tags":null}"#).unwrap().is_empty());
    }

    #[test]
    fn a_test_manifest_names_build_and_files() {
        let json = r#"{"annotations":{"version":"0.8.4","build":"12-abc1234"},"layers":[
            {"digest":"sha256:aa","annotations":{"org.opencontainers.image.title":"Faderust_0.8.4_x64-setup.exe"}},
            {"digest":"sha256:bb","annotations":{"org.opencontainers.image.title":"Faderust_0.8.4_x64-setup.exe.sig"}}]}"#;
        let b = parse_oci(json).unwrap();
        assert_eq!(b.build, "12-abc1234");
        assert_eq!(b.setup, "sha256:aa");
        assert_eq!(b.sig, "sha256:bb");
        assert!(parse_oci(r#"{"layers":[]}"#).is_err());
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
