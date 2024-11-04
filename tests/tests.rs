use atxpkg::*;
use std::collections::HashMap;
use std::path::Path;

#[test]
fn test_list_available() {
    let packages = vec![];
    let repos = vec!["./test_data".to_string()];
    let avail = list_available(packages, repos, false, false).unwrap();
    assert_eq!(
        avail,
        vec![
            ("atx300-base".to_string(), "".to_string()),
            ("atx300-base.dev".to_string(), "".to_string()),
            ("atxpkg".to_string(), "".to_string()),
            ("test".to_string(), "".to_string()),
        ]
    );
}

#[test]
fn test_clean_cache() {
    let tmp_dir = tempfile::Builder::new().tempdir().unwrap();
    let fn_ = format!("{}/some_file", tmp_dir.path().to_str().unwrap());
    std::fs::write(&fn_, "test").unwrap();

    assert!(Path::new(&fn_).exists());
    atxpkg::clean_cache(tmp_dir.path().to_str().unwrap()).unwrap();
    assert!(!Path::new(&fn_).exists());

    assert!(atxpkg::clean_cache("NON_EXISTENT").is_err());
}

#[test]
fn test_install_packages() {
    let dest_dir = tempfile::Builder::new().tempdir().unwrap();
    let dest_dir_str = dest_dir.path().to_str().unwrap();
    let tmp_dir = tempfile::Builder::new().tempdir().unwrap();
    let cache_dir = tempfile::Builder::new().tempdir().unwrap();

    let mut installed_packages = HashMap::default();
    atxpkg::install_packages(
        vec!["atx300-base".to_string(), "test".to_string()],
        &mut installed_packages,
        dest_dir_str,
        vec!["./test_data".to_string()],
        false,
        false,
        true,
        false,
        false,
        false,
        cache_dir.path().to_str().unwrap(),
        tmp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let pkginfo = installed_packages.get("atx300-base").unwrap();
    assert_eq!(pkginfo.version, "6.3-1");
    assert_eq!(pkginfo.md5sums.len(), 43);
    assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
    assert!(!Path::new(&format!("{dest_dir_str}/atx300/.atxpkg_backup")).exists());
}

#[test]
fn test_update_package() {
    let dest_dir = tempfile::Builder::new().tempdir().unwrap();
    let dest_dir_str = dest_dir.path().to_str().unwrap();
    let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

    let pkginfo = atxpkg::update_package(
        "./test_data/atx300-base-6.3-1.atxpkg.zip",
        "atx300-base",
        InstalledPackage {
            t: None,
            version: "6.3-1".to_string(),
            md5sums: HashMap::new(),
            backup: Some(Vec::new()),
        },
        dest_dir_str,
        false,
        tmp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    assert_eq!(pkginfo.version, "6.3-1");
    assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
    assert!(!Path::new(&format!("{dest_dir_str}/atx300/.atxpkg_backup")).exists());
}

#[test]
fn test_update_packages() {
    let dest_dir = tempfile::Builder::new().tempdir().unwrap();
    let dest_dir_str = dest_dir.path().to_str().unwrap();
    let tmp_dir = tempfile::Builder::new().tempdir().unwrap();
    let cache_dir = tempfile::Builder::new().tempdir().unwrap();

    let mut installed_packages = HashMap::default();
    atxpkg::install_packages(
        vec!["atx300-base".to_string(), "test-1.0-1".to_string()],
        &mut installed_packages,
        dest_dir_str,
        vec!["./test_data".to_string()],
        false,
        false,
        true,
        false,
        false,
        false,
        cache_dir.path().to_str().unwrap(),
        tmp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let pkginfo = installed_packages.get("atx300-base").unwrap();
    assert_eq!(pkginfo.version, "6.3-1");
    assert_eq!(pkginfo.md5sums.len(), 43);
    assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
    assert!(!Path::new(&format!("{dest_dir_str}/atx300/.atxpkg_backup")).exists());

    let pkginfo = installed_packages.get("test").unwrap();
    assert_eq!(pkginfo.version, "1.0-1");
    assert!(Path::new(&format!("{dest_dir_str}/test/protected1")).exists());

    atxpkg::update_packages(
        vec![
            "atx300-base..atx300-base.dev".to_string(),
            "test".to_string(),
        ],
        &mut installed_packages,
        dest_dir_str,
        vec!["./test_data".to_string()],
        false,
        false,
        true,
        false,
        false,
        false,
        cache_dir.path().to_str().unwrap(),
        tmp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    assert!(!installed_packages.contains_key("atx300-base"));
    let pkginfo = installed_packages.get("atx300-base.dev").unwrap();
    assert_eq!(pkginfo.version, "0-1");
    assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());

    let pkginfo = installed_packages.get("test").unwrap();
    assert_eq!(pkginfo.version, "2.0-1");
    assert!(Path::new(&format!("{dest_dir_str}/test/protected1")).exists());
}

#[test]
fn test_remove_packages() {
    let dest_dir = tempfile::Builder::new().tempdir().unwrap();
    let dest_dir_str = dest_dir.path().to_str().unwrap();
    let tmp_dir = tempfile::Builder::new().tempdir().unwrap();
    let cache_dir = tempfile::Builder::new().tempdir().unwrap();

    let mut installed_packages = HashMap::default();
    atxpkg::install_packages(
        vec!["atx300-base".to_string(), "test".to_string()],
        &mut installed_packages,
        dest_dir_str,
        vec!["./test_data".to_string()],
        false,
        false,
        true,
        false,
        false,
        false,
        cache_dir.path().to_str().unwrap(),
        tmp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let pkginfo = installed_packages.get("atx300-base").unwrap();
    assert_eq!(pkginfo.version, "6.3-1");
    assert_eq!(pkginfo.md5sums.len(), 43);
    assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
    assert!(!Path::new(&format!("{dest_dir_str}/atx300/.atxpkg_backup")).exists());
    assert!(Path::new(&format!("{dest_dir_str}/test/protected1")).exists());

    atxpkg::remove_packages(
        vec!["atx300-base".to_string(), "test".to_string()],
        &mut installed_packages,
        dest_dir_str,
        true,
        false,
    )
    .unwrap();

    assert!(installed_packages.is_empty());
    assert!(!Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
    assert!(!Path::new(&format!("{dest_dir_str}/test/protected1")).exists());

    let lst = Path::new(dest_dir_str)
        .read_dir()
        .unwrap()
        .map(|x| x.unwrap().path().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(lst, Vec::<String>::new());
}
