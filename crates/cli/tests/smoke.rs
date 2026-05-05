use assert_cmd::Command;

#[test]
fn nxc_help_exits_zero() {
    Command::cargo_bin("nxc").unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn nxc_models_requires_api_key() {
    Command::cargo_bin("nxc").unwrap()
        .arg("models")
        .env("NXC_API_KEY", "")
        .assert()
        .failure();
}
