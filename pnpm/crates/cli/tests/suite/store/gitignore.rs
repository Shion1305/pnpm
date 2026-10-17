use super::pacquet_at;
use assert_cmd::prelude::*;
use pnpm_store_dir::{STORE_VERSION, StoreDir};
use pnpm_testing_utils::bin::CommandTempCwd;
use pretty_assertions::assert_eq;
use std::{fs, path::Path, process::Command};

#[test]
fn install_ignores_custom_stores_without_ignoring_unrelated_files() {
    for store_path in [".pnpm-store".to_string(), ".".to_string(), format!("cache/{STORE_VERSION}")]
    {
        let CommandTempCwd { workspace, root: _root, .. } =
            CommandTempCwd::init().add_mocked_registry();
        fs::write(workspace.join("package.json"), r#"{"dependencies":{"is-positive":"1.0.0"}}"#)
            .expect("write package manifest");
        fs::write(workspace.join(".gitignore"), "node_modules/\n")
            .expect("write project gitignore");

        pacquet_at(&workspace)
            .args(["install", "--store-dir", &store_path])
            .assert()
            .success();

        let store = StoreDir::new(workspace.join(&store_path));
        assert_store_is_git_ignored(&workspace, &store);
        assert_eq!(fs::read_to_string(workspace.join(".gitignore")).unwrap(), "node_modules/\n",);
    }
}

#[test]
fn store_add_ignores_a_local_store() {
    let CommandTempCwd { workspace, root: _root, .. } =
        CommandTempCwd::init().add_mocked_registry();
    pacquet_at(&workspace)
        .args(["store", "add", "is-positive@1.0.0", "--store-dir=.pnpm-store"])
        .assert()
        .success();

    assert_store_is_git_ignored(&workspace, &StoreDir::new(workspace.join(".pnpm-store")));
    eprintln!("store add must leave the project gitignore absent");
    assert!(!workspace.join(".gitignore").exists());
}

#[test]
fn frozen_store_commands_do_not_create_a_gitignore() {
    let CommandTempCwd { workspace, root: _root, .. } =
        CommandTempCwd::init().add_mocked_registry();
    pacquet_at(&workspace)
        .args(["add", "is-positive@1.0.0", "--store-dir=.pnpm-store"])
        .assert()
        .success();
    let store = StoreDir::new(workspace.join(".pnpm-store"));
    let gitignore = store.root().join(".gitignore");
    fs::remove_file(&gitignore).expect("remove the store gitignore");
    fs::remove_dir_all(workspace.join("node_modules")).expect("remove node_modules");

    pacquet_at(&workspace)
        .args([
            "store",
            "add",
            "is-positive@1.0.0",
            "--store-dir=.pnpm-store",
            "--frozen-store",
            "--offline",
        ])
        .assert()
        .success();
    eprintln!("frozen store add must not create a gitignore");
    assert!(!gitignore.exists());

    pacquet_at(&workspace)
        .args([
            "install",
            "--store-dir=.pnpm-store",
            "--frozen-store",
            "--frozen-lockfile",
            "--offline",
        ])
        .assert()
        .success();
    eprintln!("frozen install must link the dependency without creating a store gitignore");
    assert!(workspace.join("node_modules/is-positive").is_dir());
    assert!(!gitignore.exists());
}

fn assert_store_is_git_ignored(workspace: &Path, store: &StoreDir) {
    assert_eq!(fs::read_to_string(store.root().join(".gitignore")).unwrap(), "*\n");
    eprintln!("The store must contain the fetched package index");
    assert!(store.root().join("index.db").is_file());
    let unrelated = store
        .root()
        .parent()
        .unwrap()
        .join("keep.txt");
    fs::write(&unrelated, "unrelated to the store\n").expect("write unrelated file");
    Command::new("git")
        .current_dir(workspace)
        .args(["init", "--quiet", "--template="])
        .assert()
        .success();
    Command::new("git")
        .current_dir(workspace)
        .args(["-c", "core.excludesFile=", "status", "--porcelain", "--untracked-files=all", "--"])
        .arg(
            store
                .root()
                .strip_prefix(workspace)
                .unwrap(),
        )
        .arg(unrelated.strip_prefix(workspace).unwrap())
        .assert()
        .success()
        .stdout(format!(
            "?? {}\n",
            unrelated
                .strip_prefix(workspace)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
        ));
}
