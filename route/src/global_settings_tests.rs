#[test]
fn service_credentials_have_global_routes_and_exact_shared_keys() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("../petal.toml")).unwrap();
    assert!(root.join("files/settings/api-key.rs").is_file());
    assert!(
        !root
            .join("files/settings/[wallet]/[index]/api-key.rs")
            .exists()
    );
    assert!(root.join("files/settings/status.json.rs").is_file());
    assert!(
        !root
            .join("files/settings/[wallet]/[index]/status.json.rs")
            .exists()
    );
    assert!(manifest.contains(r#"shared_keys = ["secrets/credentials/partner-jwt"]"#));
}

#[test]
fn venue_policy_routes_select_the_same_private_account_as_swaps() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let leaf = root.join("files/settings/wallets/[wallet]/[index]/venue.toml.rs");
    let source = std::fs::read_to_string(leaf).unwrap();
    assert!(source.contains("crate::account_number(ctx)"));
    assert!(
        !root
            .join("files/settings/wallets/[wallet]/venue.toml.rs")
            .exists()
    );
    let manifest = std::fs::read_to_string(root.join("../petal.toml")).unwrap();
    assert!(!manifest.contains("state/settings/wallets"));
    for directory in ["wallets/$index.rs", "wallets/[wallet]/$index.rs"] {
        let source = std::fs::read_to_string(root.join("files/settings").join(directory)).unwrap();
        assert!(!source.contains("bloom:store"));
    }
}
