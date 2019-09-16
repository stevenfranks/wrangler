#[macro_use]
extern crate lazy_static;

use assert_cmd::prelude::*;
use fs_extra::dir::{copy, CopyOptions};
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::sync::Mutex;

lazy_static! {
    static ref BUILD_LOCK: Mutex<u8> = Mutex::new(0);
}

const BUNDLE_OUT: &str = "./worker";

macro_rules! single_env_settings {
    ( $f:expr, $x:expr ) => {
        let file_path = fixture_path($f).join("wrangler.toml");
        let mut file = File::create(file_path).unwrap();
        let content = format!(
            r#"
            name = "test"
            zone_id = ""
            account_id = ""
            workers_dev = true
            {}
        "#,
            $x
        );
        file.write_all(content.as_bytes()).unwrap();
    };
}

#[test]
fn it_builds_with_webpack_single_js() {
    let fixture = "webpack_simple_js";
    create_temporary_copy(fixture);
    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_function_config_js() {
    let fixture = "webpack_function_config_js";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_promise_config_js() {
    let fixture = "webpack_promise_config_js";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_function_promise_config_js() {
    let fixture = "webpack_function_promise_config_js";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_single_js_use_package_main() {
    let fixture = "webpack_single_js_use_package_main";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_specify_configs() {
    let fixture = "webpack_specify_config";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
        webpack_config = "webpack.worker.js"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_single_js_missing_package_main() {
    let fixture = "webpack_single_js_missing_package_main";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build_fails_with(
        fixture,
        "The `main` key in your `package.json` file is required",
    );
    cleanup(fixture);
}

#[test]
fn it_fails_with_multiple_webpack_configs() {
    let fixture = "webpack_multiple_config";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build_fails_with(fixture, "Multiple webpack configurations are not supported. You can specify a different path for your webpack configuration file in wrangler.toml with the `webpack_config` field");
    cleanup(fixture);
}

#[test]
fn it_fails_with_multiple_specify_webpack_configs() {
    let fixture = "webpack_multiple_specify_config";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
        webpack_config = "webpack.worker.js"
    "#};

    build_fails_with(fixture, "Multiple webpack configurations are not supported. You can specify a different path for your webpack configuration file in wrangler.toml with the `webpack_config` field");
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_wast() {
    let fixture = "webpack_wast";
    create_temporary_copy(fixture);

    single_env_settings! {fixture, r#"
        type = "Webpack"
    "#};

    build(fixture);
    assert!(fixture_out_path(fixture).join("script.js").exists());
    assert!(fixture_out_path(fixture).join("module.wasm").exists());

    cleanup(fixture);
}

#[test]
fn it_fails_with_webpack_target_node() {
    let fixture = "webpack_target_node";
    create_temporary_copy(fixture);

    webpack_config(
        fixture,
        r#"{
          entry: "./index.js",
          target: "node",
        }"#,
    );
    single_env_settings! {fixture, r#"
        type = "webpack"
    "#};

    build_fails_with(
        fixture,
        "Building a Cloudflare Worker with target \"node\" is not supported",
    );
    cleanup(fixture);
}

#[test]
fn it_fails_with_webpack_target_web() {
    let fixture = "webpack_target_web";
    create_temporary_copy(fixture);

    webpack_config(
        fixture,
        r#"{
          entry: "./index.js",
          target: "web",
        }"#,
    );
    single_env_settings! {fixture, r#"
        type = "webpack"
    "#};

    build_fails_with(
        fixture,
        "Building a Cloudflare Worker with target \"web\" is not supported",
    );
    cleanup(fixture);
}

#[test]
fn it_builds_with_webpack_target_webworker() {
    let fixture = "webpack_target_webworker";
    create_temporary_copy(fixture);

    webpack_config(
        fixture,
        r#"{
          entry: "./index.js",
          target: "webworker",
        }"#,
    );
    single_env_settings! {fixture, r#"
        type = "webpack"
    "#};

    build(fixture);
    cleanup(fixture);
}

fn cleanup(fixture: &str) {
    let path = fixture_path(fixture);
    assert!(path.exists(), format!("{:?} does not exist", path));

    // Workaround https://github.com/rust-lang/rust/issues/29497
    if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("rmdir");
        command.arg("/s");
        command.arg(&path);
    } else {
        fs::remove_dir_all(&path).unwrap();
    }
}

fn build(fixture: &str) {
    // Lock to avoid having concurrent builds
    let _g = BUILD_LOCK.lock().unwrap();

    let mut build = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    build.current_dir(fixture_path(fixture));
    build.arg("build").assert().success();
}

fn build_fails_with(fixture: &str, expected_message: &str) {
    // Lock to avoid having concurrent builds
    let _g = BUILD_LOCK.lock().unwrap();

    let mut build = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    build.current_dir(fixture_path(fixture));
    build.arg("build");

    let output = build.output().expect("failed to execute process");
    assert!(!output.status.success());
    assert!(
        str::from_utf8(&output.stderr)
            .unwrap()
            .contains(expected_message),
        format!(
            "expected {:?} not found, given: {:?}",
            expected_message,
            str::from_utf8(&output.stderr)
        )
    );
}

fn fixture_path(fixture: &str) -> PathBuf {
    let mut dest = env::temp_dir();
    dest.push(fixture);
    dest
}

fn fixture_out_path(fixture: &str) -> PathBuf {
    fixture_path(fixture).join(BUNDLE_OUT)
}

fn create_temporary_copy(fixture: &str) {
    let current_dir = env::current_dir().unwrap();
    let src = Path::new(&current_dir).join("tests/fixtures").join(fixture);

    let dest = env::temp_dir();

    if dest.join(fixture).exists() {
        cleanup(fixture);
    }

    fs::create_dir_all(dest.clone()).unwrap();
    let mut options = CopyOptions::new();
    options.overwrite = true;
    copy(src, dest, &options).unwrap();
}

// TODO: remove once https://github.com/cloudflare/wrangler/pull/489 is merged
pub fn webpack_config(fixture: &str, config: &str) {
    let file_path = fixture_path(fixture).join("webpack.config.js");
    let mut file = File::create(file_path).unwrap();
    let content = format!(
        r#"
                 module.exports = {};
             "#,
        config
    );
    file.write_all(content.as_bytes()).unwrap();
}
