use super::*;
use std::os::unix::fs::symlink;
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "containerdesk-discovery-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        ));
        fs::create_dir_all(path.join(".ssh")).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, data: impl AsRef<[u8]>) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, data).unwrap();
    }
    fn read(&self) -> HostDiscovery {
        discover(&self.0, self.0.join(".ssh/config").to_str().unwrap()).unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn aliases(report: &HostDiscovery) -> Vec<&str> {
    report.candidates.iter().map(|c| c.alias.as_str()).collect()
}
fn warning(report: &HostDiscovery, code: DiscoveryWarningCode) -> bool {
    report.warnings.iter().any(|w| w.code == code)
}
#[test]
fn aliases_quotes_comments_and_manual_subset_are_conservative() {
    let lab = Lab::new();
    lab.write(".ssh/config", "# comment\nhOsT = one two \"three\" *.example !excluded # comment\n  HostName internal.example\nHost one\nHost -oProxyCommand=x \"space name\" 日本語 user@host\n");
    let report = lab.read();
    assert_eq!(aliases(&report), ["one", "three", "two"]);
    assert_eq!(report.candidates[0].line, 2);
    assert!(warning(&report, DiscoveryWarningCode::PatternsSkipped));
    for valid in ["manual-01", "a_b.c", "127.0.0.1"] {
        assert!(crate::ssh::validate_alias(valid).is_ok());
    }
    for invalid in [
        "",
        "-option",
        "a b",
        "a\nb",
        "a;id",
        "a$(id)",
        "a/b",
        "::1",
        "user@host",
        &"a".repeat(257),
    ] {
        assert!(crate::ssh::validate_alias(invalid).is_err());
    }
}
#[test]
fn includes_use_user_ssh_base_lexical_order_spaces_and_cycle_detection() {
    let lab = Lab::new();
    lab.write(
        ".ssh/config",
        "Include \"parts with spaces/*.conf\" missing.conf\nHost root\n",
    );
    lab.write(
        ".ssh/parts with spaces/b.conf",
        "Host *\nInclude nested.conf\nHost duplicate from-b\nHost *\n",
    );
    lab.write(
        ".ssh/parts with spaces/a.conf",
        "Host duplicate from-a\nHost *\n",
    );
    lab.write(".ssh/nested.conf", "Include config\nHost nested\nHost *\n");
    let report = lab.read();
    assert_eq!(
        aliases(&report),
        ["duplicate", "from-a", "from-b", "nested", "root"]
    );
    assert!(report.candidates[0].source.ends_with("a.conf"));
    assert!(warning(&report, DiscoveryWarningCode::IncludeCycle));
    assert!(warning(&report, DiscoveryWarningCode::MissingFile));
    // Relative includes are based on ~/.ssh, not the including file's parent.
    assert!(
        report
            .candidates
            .iter()
            .any(|c| c.source.ends_with("/.ssh/nested.conf"))
    );
}
#[test]
fn match_exec_proxycommand_and_conditional_includes_are_never_evaluated_or_rewritten() {
    let lab = Lab::new();
    let marker = lab.0.join("executed");
    let config = format!(
        "Match exec \"touch {}\"\nInclude conditional.conf\nHost safe\nProxyCommand touch {}\nInclude conditional.conf\nHost *\nInclude ${{UNTRUSTED}} %h.conf ~someone/config [ab].conf\n",
        marker.display(),
        marker.display()
    );
    lab.write(".ssh/config", &config);
    lab.write(".ssh/conditional.conf", "Host conditional\n");
    let report = lab.read();
    assert_eq!(aliases(&report), ["safe"]);
    assert!(warning(&report, DiscoveryWarningCode::ConditionalInclude));
    assert!(warning(&report, DiscoveryWarningCode::MatchSkipped));
    assert!(warning(&report, DiscoveryWarningCode::UnsupportedSyntax));
    assert!(!marker.exists());
    assert_eq!(
        fs::read_to_string(lab.0.join(".ssh/config")).unwrap(),
        config
    );
    assert!(!serde_json::to_string(&report).unwrap().contains("touch"));
}
#[test]
fn symlink_cycles_missing_root_malformed_input_and_special_files_are_bounded() {
    let lab = Lab::new();
    assert!(warning(&lab.read(), DiscoveryWarningCode::MissingFile));
    lab.write(".ssh/config", "Include link\nHost okay\n");
    symlink("config", lab.0.join(".ssh/link")).unwrap();
    assert!(warning(&lab.read(), DiscoveryWarningCode::IncludeCycle));
    lab.write(
        ".ssh/config",
        "Host \"unterminated\nInclude link\nHost good\n",
    );
    assert_eq!(aliases(&lab.read()), ["good"]);
    lab.write(".ssh/config", vec![b'x'; FILE_BYTES + 1]);
    assert!(warning(&lab.read(), DiscoveryWarningCode::LimitReached));
    let report = discover(&lab.0, "/dev/null").unwrap();
    assert!(warning(&report, DiscoveryWarningCode::UnreadableFile));
    assert!(config_path(&lab.0, Some("relative/config")).is_err());
    assert!(config_path(&lab.0, Some("/tmp/config\0")).is_err());
    assert_eq!(
        config_path(&lab.0, None).unwrap(),
        lab.0.join(".ssh/config").to_str().unwrap()
    );
}
#[test]
fn recursion_candidate_and_glob_enumeration_limits_surface_warnings() {
    let lab = Lab::new();
    lab.write(".ssh/config", "Include depth0\n");
    for i in 0..20 {
        lab.write(
            &format!(".ssh/depth{i}"),
            format!("Include depth{}\n", i + 1),
        );
    }
    assert!(warning(&lab.read(), DiscoveryWarningCode::LimitReached));
    lab.write(
        ".ssh/config",
        (0..1010)
            .map(|i| format!("Host host-{i}\n"))
            .collect::<String>(),
    );
    let report = lab.read();
    assert_eq!(report.candidates.len(), 1000);
    assert!(warning(&report, DiscoveryWarningCode::LimitReached));
    let mut entries = MAX_ENTRIES;
    assert!(matches!(
        expand(&lab.0.join(".ssh/*"), &mut entries),
        Err(DiscoveryWarningCode::LimitReached)
    ));
    assert!(wildcard(b"a*?z", b"abcdz"));
    assert!(!wildcard(b"a?z", b"abcdz"));
}
