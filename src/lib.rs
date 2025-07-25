use itertools::Itertools;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, IsTerminal, Read, Write};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

const MAX_CONCURRENT_DOWNLOADS: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InstalledPackage {
    pub t: Option<f64>,
    pub version: String,
    pub md5sums: HashMap<String, Option<String>>,
    pub backup: Option<Vec<String>>,
}

#[derive(Clone)]
struct PackageUpdate {
    name_old: String,
    version_old: String,
    name_new: String,
    version_new: String,
    url: String,
    local_fn: String,
}

fn as_unix_path(pth: &Path) -> String {
    let ret = pth
        .components()
        .map(|x| x.as_os_str().to_string_lossy())
        .join("/");
    // TODO: this used to fuck up things for windows - investigate and fix
    #[cfg(target_os = "linux")]
    if pth.is_absolute() {
        return format!("/{ret}");
    }
    ret
}

fn move_file(from: &str, to: &str) -> anyhow::Result<()> {
    // we try deletion first because the target file may be held onto by another process
    try_delete(to)?;
    if let Err(err) = std::fs::rename(from, to) {
        if err.kind() == std::io::ErrorKind::Unsupported {
            // probably different filesystem, retry with copy+remove method
            std::fs::copy(from, to)?;
            //std::fs::remove_file(from)?;
            try_delete(from)?;
        } else {
            return Err(err.into());
        }
    }
    Ok(())
}

fn make_progress_bar(
    len: u64,
    prefix: &str,
    template: &str,
) -> anyhow::Result<indicatif::ProgressBar> {
    let progress_bar = indicatif::ProgressBar::new(len).with_prefix(prefix.to_string());
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(template)?
            .tick_chars(r"|/-\ ")
            .progress_chars("##-"),
    );
    Ok(progress_bar)
}

pub fn get_installed_packages(db_fn: &str) -> anyhow::Result<HashMap<String, InstalledPackage>> {
    log::debug!("getting installed packages from {db_fn}");
    if !Path::new(db_fn).exists() {
        return Ok(HashMap::new());
    }
    Ok(serde_json::from_reader(BufReader::new(File::open(db_fn)?))?)
}

pub fn save_installed_packages(
    installed_packages: &HashMap<String, InstalledPackage>,
    db_fn: &str,
) -> anyhow::Result<()> {
    log::debug!(
        "saving {} installed packages to {db_fn}",
        installed_packages.len()
    );
    let mut f = BufWriter::new(File::create(db_fn)?);
    let encoder = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut ser = serde_json::Serializer::with_formatter(&mut f, encoder);
    installed_packages.serialize(&mut ser)?;
    f.flush()?;
    Ok(())
}

fn get_available_packages(
    repos: Vec<String>,
    offline: bool,
    unverified_ssl: bool,
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    log::debug!("getting available packages from {repos:?}");

    let mb = indicatif::MultiProgress::new();

    let ret = {
        let (tx, rx) = std::sync::mpsc::channel();
        scoped_threadpool::Pool::new(MAX_CONCURRENT_DOWNLOADS).scoped(|scope| {
            for repo in repos {
                if offline && is_url(&repo) {
                    continue;
                }
                let tx = &tx;
                let mb = &mb;
                scope.execute(move || {
                    let res = (|| {
                        let pb = make_progress_bar(
                            0,
                            &repo,
                            "{spinner} {prefix} [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec})",
                        )?;
                        mb.add(pb.clone());
                        let listing = get_repo_listing(&repo, unverified_ssl, Some(&pb));
                        pb.finish();
                        anyhow::Ok(
                            listing?
                                .into_iter()
                                .filter_map(|url| {
                                    let package_fn = get_package_fn(&url)?;
                                    if !is_valid_package_fn(&package_fn) {
                                        log::warn!("{package_fn} not a valid package filename");
                                        return None;
                                    }
                                    let package_name = get_package_name(&package_fn);
                                    Some((package_name, url))
                                })
                                .collect::<Vec<_>>(),
                        )
                    })();
                    tx.send(res).unwrap();
                });
            }
        });
        drop(tx);
        let ret: Vec<_> = rx.iter().try_collect()?;
        anyhow::Ok(ret)
    }?
    .into_iter()
    .flatten()
    .into_group_map();

    //mb.clear();
    eprintln!();

    Ok(ret)
}

fn is_valid_package_fn(fn_: &str) -> bool {
    let re = lazy_regex::regex!(r"[\w\-\.]+-[\d.]+-\d+\.atxpkg\.zip");
    re.is_match(fn_)
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn get_repo_listing(
    repo: &str,
    unverified_ssl: bool,
    progress_bar: Option<&indicatif::ProgressBar>,
) -> anyhow::Result<Vec<String>> {
    log::info!("getting repo listing from {repo}");
    if is_url(repo) {
        return get_repo_listing_http(repo, unverified_ssl, progress_bar);
    }
    get_repo_listing_dir(repo, progress_bar)
}

fn get_repo_listing_http(
    url: &str,
    unverified_ssl: bool,
    progress_bar: Option<&indicatif::ProgressBar>,
) -> anyhow::Result<Vec<String>> {
    let client = reqwest::blocking::ClientBuilder::new()
        .danger_accept_invalid_certs(unverified_ssl)
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to download listing: {}", resp.status());
    };

    let total_size = resp.content_length().unwrap_or(0);

    let mut reader: Box<dyn std::io::Read> = if let Some(pb) = progress_bar {
        pb.set_length(total_size);
        pb.enable_steady_tick(Duration::from_millis(200));
        Box::new(pb.wrap_read(resp))
    } else {
        Box::new(resp)
    };

    let mut body = String::with_capacity(total_size.try_into()?);
    reader.read_to_string(&mut body)?;

    let re = lazy_regex::regex!(r#"href\s*=\s*["']?([^"'\s>]+)["']?"#);
    let files = re
        .captures_iter(&body)
        .map(|x| x.get(1).unwrap().as_str())
        .filter(|x| x.ends_with(".atxpkg.zip"))
        .map(|x| format!("{url}/{x}"))
        .collect::<Vec<_>>();

    Ok(files)
}

fn get_repo_listing_dir(
    path: &str,
    progress_bar: Option<&indicatif::ProgressBar>,
) -> anyhow::Result<Vec<String>> {
    let mut ret = Vec::new();

    let walker = walkdir::WalkDir::new(path).into_iter();
    let iter: Box<dyn Iterator<Item = _>> = if let Some(pb) = progress_bar {
        pb.enable_steady_tick(Duration::from_millis(200));
        Box::new(pb.wrap_iter(walker))
    } else {
        Box::new(walker)
    };
    for entry in iter {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }
        let file_path = as_unix_path(entry.path());
        if !file_path.ends_with(".atxpkg.zip") {
            continue;
        }
        ret.push(file_path);
    }

    Ok(ret)
}

fn download_package_if_needed(
    url: &str,
    cache_dir: &str,
    unverified_ssl: bool,
    progress_bar: Option<&indicatif::ProgressBar>,
) -> anyhow::Result<String> {
    if !is_url(url) {
        return Ok(url.to_string());
    }

    let fn_ = format!("{cache_dir}/{}", get_package_fn(url).unwrap());
    let fn_temp = format!("{fn_}_");

    if Path::new(&fn_).exists() {
        log::info!("using cached {fn_}");
        return Ok(fn_);
    }

    log::info!("downloading {url} to {fn_temp}");
    let mut resume_from = 0;

    if Path::new(&fn_temp).exists() {
        let client = reqwest::blocking::ClientBuilder::new()
            .danger_accept_invalid_certs(unverified_ssl)
            .build()?;
        let resp = client.head(url).send()?;
        if resp.status().is_success()
            && resp
                .headers()
                .get("Accept-Ranges")
                .is_some_and(|v| v == "bytes")
        {
            if let Ok(metadata) = std::fs::metadata(&fn_temp) {
                resume_from = metadata.len();
            }
        }
    }

    let client = reqwest::blocking::ClientBuilder::new()
        .danger_accept_invalid_certs(unverified_ssl)
        .build()?;
    let mut req = client.get(url);

    if resume_from > 0 {
        log::info!("resuming from {resume_from}");
        req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }

    let resp = req.send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to download file: {}", resp.status());
    };

    let size_to_download = resp.content_length().unwrap_or(0);

    let mut reader: Box<dyn std::io::Read> = Box::new(resp);

    if let Some(pb) = progress_bar {
        pb.set_length(resume_from + size_to_download);
        pb.set_position(resume_from);
        pb.reset_eta();
        pb.enable_steady_tick(Duration::from_millis(200));
        reader = Box::new(pb.wrap_read(reader));
    }

    let f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&fn_temp)?;
    let mut f = BufWriter::new(f);
    std::io::copy(&mut reader, &mut f)?;
    f.flush()?;

    if let Some(pb) = progress_bar {
        pb.finish();
    }

    log::trace!("renaming {fn_temp} to {fn_}");
    std::fs::rename(&fn_temp, &fn_)?;

    Ok(fn_)
}

fn try_delete(fn_: &str) -> anyhow::Result<()> {
    if std::fs::metadata(fn_).is_err() {
        // TODO: shouldn't we fail here?
        return Ok(());
    }

    if !Path::new(&fn_).is_file() {
        anyhow::bail!("not a file: {fn_}");
    }

    let mut del_fn = format!("{fn_}.atxpkg_delete");
    while Path::new(&del_fn).exists() {
        if let Err(err) = std::fs::remove_file(&del_fn) {
            log::warn!("failed to remove file: {del_fn} - error: {err}");
        } else {
            break;
        }
        del_fn.push_str("_delete");
    }

    log::trace!("renaming {fn_} to {del_fn}");
    std::fs::rename(fn_, &del_fn)?;

    if let Err(err) = std::fs::remove_file(&del_fn) {
        log::warn!("failed to remove file: {del_fn} - error: {err}");
    }

    Ok(())
}

pub fn list_available(
    packages: Vec<String>,
    repos: Vec<String>,
    offline: bool,
    unverified_ssl: bool,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut ret = Vec::new();
    let available_packages = get_available_packages(repos, offline, unverified_ssl)?;

    if packages.is_empty() {
        let mut keys = available_packages
            .keys()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        for k in keys {
            ret.push((k.clone(), String::new()));
        }
    } else {
        for p in &packages {
            let Some(urls) = available_packages.get(p) else {
                anyhow::bail!("package {p} not available");
            };
            for url in urls {
                let version = get_package_version(&get_package_fn(url).unwrap());
                ret.push((p.clone(), version.clone()));
            }
        }
        ret.sort_unstable();
    }
    Ok(ret)
}

fn split_package_name_version(pkg_spec: &str) -> (String, String) {
    let re = lazy_regex::regex!(r"^(.+?)(?:-([\d.-]+))?(?:\.atxpkg\.zip)?$");
    let matches = re.captures(pkg_spec);

    if let Some(caps) = matches {
        let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
        let version = caps.get(2).map_or("", |m| m.as_str()).to_string();
        return (name, version);
    }

    (String::new(), String::new())
}

fn get_package_fn(url: &str) -> Option<String> {
    let parts = url.split('/').map(|x| x.to_string()).collect::<Vec<_>>();
    parts.last().map(|x| x.to_string())
}

fn get_package_name(fn_: &str) -> String {
    let (name, _) = split_package_name_version(fn_);
    name
}

fn get_package_version(fn_: &str) -> String {
    let (_, version) = split_package_name_version(fn_);
    version
}

pub fn if_installed(
    packages: Vec<String>,
    installed_packages: &HashMap<String, InstalledPackage>,
) -> anyhow::Result<()> {
    for p in &packages {
        let (package_name, package_version) = split_package_name_version(p);
        let Some(installed_package) = installed_packages.get(&package_name) else {
            anyhow::bail!("package {package_name} not installed");
        };
        if !package_version.is_empty() && package_version != installed_package.version {
            anyhow::bail!("package {package_name}-{package_version} not installed");
        }
    }
    Ok(())
}

pub fn clean_cache(cache_dir: &str) -> anyhow::Result<()> {
    let files = std::fs::read_dir(cache_dir)?;
    for file in files {
        let file = file?;
        let file_path = file.path();
        if file_path.file_name().is_some() {
            let file_path_str = file_path.to_string_lossy().into_owned();
            std::fs::remove_file(&file_path)?;
            eprintln!("D {file_path_str}");
        }
    }
    Ok(())
}

pub fn install_packages(
    packages: Vec<String>,
    installed_packages: &mut HashMap<String, InstalledPackage>,
    prefix: &str,
    repos: Vec<String>,
    force: bool,
    offline: bool,
    yes: bool,
    no: bool,
    download_only: bool,
    unverified_ssl: bool,
    cache_dir: &str,
    tmp_dir_prefix: &str,
) -> anyhow::Result<bool> {
    let available_packages = get_available_packages(repos, offline, unverified_ssl)?;

    for p in &packages {
        let package_name = get_package_name(p);
        if installed_packages.contains_key(&package_name) && !force && !download_only {
            anyhow::bail!("package {package_name} already installed");
        }
        if !available_packages.contains_key(&package_name) {
            anyhow::bail!("unable to find url for package {package_name}");
        }
    }

    let mut urls_to_install = vec![];
    for p in &packages {
        let (package_name, package_version) = split_package_name_version(p);
        let package_urls = available_packages
            .get(&package_name)
            .expect("safe unwrap due to earlier check");
        let url = if !package_version.is_empty() {
            get_specific_version_url(package_urls.clone(), &package_version)
        } else {
            get_max_version_url(package_urls.clone())
        }
        .unwrap();
        urls_to_install.push(url.clone());
        let (package_name, package_version) =
            split_package_name_version(&get_package_fn(&url).unwrap());
        match download_only {
            true => println!("download {package_name}-{package_version}"),
            false => println!("install {package_name}-{package_version}"),
        }
    }
    if no || !(yes || yes_no("continue?", "y")?) {
        return Ok(false);
    }

    let mb = indicatif::MultiProgress::new();

    let local_fns_to_install = {
        let (tx, rx) = std::sync::mpsc::channel();
        scoped_threadpool::Pool::new(MAX_CONCURRENT_DOWNLOADS).scoped(|scope| {
            for url in &urls_to_install {
                let tx = &tx;
                let mb = &mb;
                scope.execute(move || {
                    let res = (|| {
                        let package_name = get_package_name(&get_package_fn(url).unwrap());
                        let pb = make_progress_bar(
                            0,
                            &package_name,
                            "{spinner} {prefix} [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )?;
                        mb.add(pb.clone());
                        download_package_if_needed(url, cache_dir, unverified_ssl, Some(&pb))
                    })();
                    tx.send(res).unwrap();
                });
            }
        });
        drop(tx);
        let local_fns_to_install: Vec<_> = rx.iter().try_collect()?;
        anyhow::Ok(local_fns_to_install)
    }?;

    //mb.clear();
    eprintln!();

    if download_only {
        return Ok(false);
    }

    for local_fn in &local_fns_to_install {
        let (package_name, package_version) =
            split_package_name_version(&get_package_fn(local_fn).unwrap());
        let package_info = install_package(local_fn, prefix, force, tmp_dir_prefix)?;
        installed_packages.insert(package_name.clone(), package_info);
        println!("{package_name}-{package_version} is now installed");
    }

    Ok(true)
}

fn yes_no(prompt: &str, default: &str) -> anyhow::Result<bool> {
    if default.is_empty() && std::io::stdin().is_terminal() {
        anyhow::bail!("input is not a tty");
    };

    let question = match default {
        "y" => format!("{prompt} [Y/n] "),
        "n" => format!("{prompt} [y/N] "),
        _ => format!("{prompt} [y/n] "),
    };

    loop {
        print!("{question}");
        std::io::stdout().flush()?;

        let mut ans = String::new();
        std::io::stdin()
            .read_line(&mut ans)
            .expect("Failed to read line");
        let ans = ans.trim().to_lowercase();

        match ans.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            "" => {
                if default == "y" {
                    return Ok(true);
                } else if default == "n" {
                    return Ok(false);
                }
            }
            _ => println!("Invalid input. Please enter 'y' or 'n'."),
        }
    }
}

fn get_max_version_url(urls: Vec<String>) -> Option<String> {
    let mut max_version_url: Option<String> = None;
    for url in urls {
        let package_version = get_package_version(&get_package_fn(&url)?);
        if let Some(max_version_url_) = &max_version_url {
            let max_version = get_package_version(&get_package_fn(max_version_url_)?);
            if compare_versions(&package_version, &max_version) == std::cmp::Ordering::Greater {
                max_version_url = Some(url);
            }
        } else {
            max_version_url = Some(url);
        }
    }
    max_version_url
}

fn get_max_version(urls: Vec<String>) -> Option<String> {
    Some(get_package_version(&get_package_fn(&get_max_version_url(
        urls,
    )?)?))
}

fn split_ver(ver: &str) -> Vec<u64> {
    let regex = lazy_regex::regex!(r"[.-]");
    let parts = regex.split(ver).map(|x| x.to_string()).collect::<Vec<_>>();
    parts
        .into_iter()
        .map(|x| x.parse::<u64>().unwrap())
        .collect()
}

fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let split_v1 = split_ver(v1);
    let split_v2 = split_ver(v2);
    split_v1.cmp(&split_v2)
}

fn get_specific_version_url(urls: Vec<String>, version: &str) -> Option<String> {
    for url in urls {
        if get_package_version(&get_package_fn(&url)?) == version {
            return Some(url.clone());
        }
    }
    None
}

fn install_package(
    fn_zip: &str,
    prefix: &str,
    force: bool,
    tmp_dir_prefix: &str,
) -> anyhow::Result<InstalledPackage> {
    let (name, version_new) = split_package_name_version(&get_package_fn(fn_zip).unwrap());
    log::info!("installing {name}-{version_new}");
    println!("installing {name}-{version_new}");

    let tmp_dir = tempfile::Builder::new().tempdir_in(tmp_dir_prefix)?;
    let tmp_dir_path = as_unix_path(tmp_dir.path());
    unzip_to(fn_zip, &tmp_dir_path, &name)?;

    let backup = read_lines(&format!("{tmp_dir_path}/.atxpkg_backup")).ok();

    let (dirs, mut files) = get_recursive_listing(&tmp_dir_path)?;
    files.retain(|x| !x.starts_with(".atxpkg_"));

    if !force {
        let progress_bar = make_progress_bar(
            files.len().try_into()?,
            &name,
            "{spinner} {prefix}: check [{wide_bar}] {pos}/{len}",
        )?;

        for f in progress_bar.wrap_iter(files.iter()) {
            let target_fn = format!("{prefix}/{f}");
            if Path::new(&target_fn).exists() {
                anyhow::bail!("file exists: {target_fn}");
            }
        }

        progress_bar.finish();
        eprintln!();
    }

    let mut md5sums = HashMap::new();

    let progress_bar = make_progress_bar(
        (dirs.len() + files.len()).try_into()?,
        &name,
        "{spinner} {prefix}: install [{wide_bar}] {pos}/{len}",
    )?;

    for d in progress_bar.wrap_iter(dirs.into_iter().sorted_by_key(|x| x.len())) {
        let target_dir = format!("{prefix}/{d}");
        log::trace!("ID {d}");
        if !Path::new(&target_dir).exists() {
            std::fs::create_dir(&target_dir)?;
        }
        let src_info = std::fs::metadata(format!("{tmp_dir_path}/{d}"))?;
        std::fs::set_permissions(&target_dir, src_info.permissions())?;
        let mod_time = src_info.modified().unwrap_or(std::time::SystemTime::now());
        filetime::set_file_times(&target_dir, mod_time.into(), mod_time.into())?;
        md5sums.insert(d, None);
    }

    for f in progress_bar.wrap_iter(files.into_iter()) {
        let sum = get_md5_sum(&format!("{tmp_dir_path}/{f}"))?;
        md5sums.insert(f.clone(), Some(sum));

        let target_fn = format!("{prefix}/{f}");
        if Path::new(&target_fn).exists() && backup.clone().unwrap_or_default().contains(&f) {
            log::info!("saving untracked {target_fn} as {target_fn}.atxpkg_save");
            //progress_bar.println(format!(
            //    "saving untracked {target_fn} as {target_fn}.atxpkg_save"
            //));
            progress_bar.suspend(|| {
                eprintln!("saving untracked {target_fn} as {target_fn}.atxpkg_save");
            });
            move_file(&target_fn, &format!("{target_fn}.atxpkg_save"))?;
        }
        log::trace!("IF {target_fn}");
        move_file(&format!("{tmp_dir_path}/{f}"), &target_fn)?;
    }

    progress_bar.finish();
    eprintln!();

    Ok(InstalledPackage {
        t: Some(UNIX_EPOCH.elapsed()?.as_secs_f64()),
        version: version_new.clone(),
        md5sums,
        backup,
    })
}

fn get_md5_sum(file_path: &str) -> anyhow::Result<String> {
    let mut hasher = Md5::new();
    let mut buffer = [0u8; 1024 * 1024];
    let mut reader = BufReader::new(File::open(file_path)?);
    while let Ok(size) = reader.read(&mut buffer) {
        hasher.update(&buffer[..size]);
        if size == 0 {
            break;
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

fn unzip_to(
    zip_file_path: &str,
    output_dir: &str,
    progress_bar_prefix: &str,
) -> anyhow::Result<()> {
    log::debug!("unzip {zip_file_path} to {output_dir}");

    let mut archive = zip::read::ZipArchive::new(BufReader::new(File::open(zip_file_path)?))?;

    let progress_bar = make_progress_bar(
        archive.len().try_into()?,
        progress_bar_prefix,
        "{spinner} {prefix}: unzip [{wide_bar}] {pos}/{len}",
    )?;

    for i in progress_bar.wrap_iter(0..archive.len()) {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(&output_dir).join(file.name());
        log::trace!("unzip {}", as_unix_path(&outpath));

        if (file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outf = BufWriter::new(File::create(&outpath)?);
            std::io::copy(&mut file, &mut outf)?;
            outf.flush()?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &outpath,
                std::fs::Permissions::from_mode(file.unix_mode().unwrap_or(0o755)),
            )?;
        }

        // TODO: so i have to do this shit to get file times right - still, i don't like it
        if let Some(mtime) = file.last_modified() {
            let stime = time::OffsetDateTime::try_from(mtime)?;
            // TODO: getting local offset seems to fail on linux - solve somehow
            let stime = stime.replace_offset(
                time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
            );
            let ftime = filetime::FileTime::from_unix_time(stime.unix_timestamp(), 0);
            filetime::set_file_times(&outpath, ftime, ftime)?;
        } else {
            log::warn!("failed to get file time for {}", as_unix_path(&outpath));
        };
    }

    progress_bar.finish();
    eprintln!();

    log::debug!("done unzipping");
    Ok(())
}

fn get_recursive_listing(path_base: &str) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let (mut ret_dirs, mut ret_files) = (Vec::new(), Vec::new());

    for entry in walkdir::WalkDir::new(path_base) {
        let entry = entry?;
        let path = entry.path();
        let path_relative = path.strip_prefix(path_base)?;
        let path_str = as_unix_path(path_relative);
        if path_str.is_empty() {
            continue;
        }
        if path.is_dir() {
            log::trace!("cut path D: {path_str}");
            ret_dirs.push(path_str);
        } else {
            log::trace!("cut path F: {path_str}");
            ret_files.push(path_str);
        }
    }

    Ok((ret_dirs, ret_files))
}

fn read_lines(fn_: &str) -> anyhow::Result<Vec<String>> {
    Ok(std::fs::read_to_string(fn_)?
        .split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect::<Vec<_>>())
}

pub fn update_package(
    fn_zip: &str,
    name_old: &str,
    installed_package: InstalledPackage,
    prefix: &str,
    force: bool,
    tmp_dir_prefix: &str,
) -> anyhow::Result<InstalledPackage> {
    let version_old = installed_package.version.clone();
    let (name, version_new) = split_package_name_version(&get_package_fn(fn_zip).unwrap());
    log::info!("updating {name_old}-{version_old} -> {name}-{version_new}");
    println!("updating {name_old}-{version_old} -> {name}-{version_new}");

    let tmp_dir = tempfile::Builder::new().tempdir_in(tmp_dir_prefix)?;
    let tmp_dir_path = as_unix_path(tmp_dir.path());
    unzip_to(fn_zip, &tmp_dir_path, &name)?;

    let backup = read_lines(&format!("{tmp_dir_path}/.atxpkg_backup")).ok();

    let (dirs, mut files) = get_recursive_listing(&tmp_dir_path)?;
    files.retain(|x| !x.starts_with(".atxpkg_"));

    if !force {
        let progress_bar = make_progress_bar(
            files.len().try_into()?,
            &name,
            "{spinner} {prefix}: check [{wide_bar}] {pos}/{len}",
        )?;

        for f in &files {
            let target_fn = format!("{prefix}/{f}");
            if Path::new(&target_fn).exists() && !installed_package.md5sums.contains_key(f) {
                anyhow::bail!("{f} already exists but is not part of original package");
            }
        }

        progress_bar.finish();
        eprintln!();
    }

    let mut md5sums = HashMap::new();

    let progress_bar = make_progress_bar(
        (dirs.len() + files.len()).try_into()?,
        &name,
        "{spinner} {prefix}: update [{wide_bar}] {pos}/{len}",
    )?;

    for d in progress_bar.wrap_iter(dirs.into_iter().sorted()) {
        let target_dir = format!("{prefix}/{d}");
        log::trace!("UD {target_dir}");
        if !Path::new(&target_dir).exists() {
            std::fs::create_dir(&target_dir)?;
        }
        let src_info = std::fs::metadata(format!("{tmp_dir_path}/{d}"))?;
        std::fs::set_permissions(&target_dir, src_info.permissions())?;
        let mod_time = src_info.modified().unwrap_or(std::time::SystemTime::now());
        filetime::set_file_times(&target_dir, mod_time.into(), mod_time.into())?;
        md5sums.insert(d, None);
    }

    for f in progress_bar.wrap_iter(files.into_iter()) {
        let sum_new = get_md5_sum(&format!("{tmp_dir_path}/{f}"))?;
        md5sums.insert(f.clone(), Some(sum_new.clone()));

        let mut target_fn = format!("{prefix}/{f}");
        if Path::new(&target_fn).exists() && backup.clone().unwrap_or_default().contains(&f) {
            if let Some(Some(sum_original)) = installed_package.md5sums.get(&f) {
                let sum_current = get_md5_sum(&target_fn)?;
                // only if the user has altered the file and it's altered in a way that it is not the same as the to-be-installed version - only then install the new file to different location
                if sum_original != &sum_current && sum_current != sum_new {
                    // user has altered the file in a way that it is different from the one in the new package, install the new file to different location
                    log::info!(
                        "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                    );
                    //progress_bar.println(format!(
                    //    "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                    //));
                    progress_bar.suspend(|| {
                        eprintln!("sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new");
                    });
                    target_fn += ".atxpkg_new";
                }
                /*
                if sum_original != &sum_current {
                    // user has altered the file

                    if &sum_current == &sum_new {
                        // the file in new package is the same as the one on disk, overwrite normally (to update file metadata)
                    } else {
                        // user has altered the file in a way that it is different from the one in the new package, install the new file to different location
                        log::info!(
                            "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                        );
                        progress_bar.println(format!(
                            "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                        ));
                        target_fn += ".atxpkg_new";
                    }
                }
                */
                /*
                if sum_original == &sum_new {
                    // file not changed between package versions - leave the file currently on disk as is - the user may or may not have altered it
                    log::trace!("S {target_fn}");
                    continue;
                }
                if &sum_current == &sum_new {
                    // file changed between package versions but the current on-disk version is the same as the one that is just to be installed so it does not really matter if we overwrite it or not - leave the on-disk version
                    continue;
                }
                if sum_original != &sum_current {
                    // user altered the file
                    // file changed between package versions but the on-disk file does not match the to-be-installed version - let's not overwrite user's altered file and let's install the new version "next" to it
                    log::info!(
                        "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                    );
                    progress_bar.println(format!(
                        "sum for file {target_fn} changed, installing new version as {target_fn}.atxpkg_new"
                    ));
                    target_fn += ".atxpkg_new";
                }
                */
            }
        }
        log::trace!("UF {target_fn}");
        move_file(&format!("{tmp_dir_path}/{f}"), &target_fn)?;
    }

    progress_bar.finish();
    eprintln!();

    let (mut dirs_old, mut files_old) = (vec![], vec![]);
    for (fn_or_dir_old, md5sum_old) in installed_package.md5sums.into_iter() {
        if let Some(md5sum_old) = md5sum_old {
            files_old.push((fn_or_dir_old, md5sum_old));
        } else {
            dirs_old.push(fn_or_dir_old);
        }
    }

    let progress_bar = make_progress_bar(
        (dirs_old.len() + files_old.len()).try_into()?,
        &name,
        "{spinner} {prefix}: cleanup [{wide_bar}] {pos}/{len}",
    )?;

    for (fn_old, md5sum_old) in progress_bar.wrap_iter(files_old.into_iter()) {
        if md5sums.contains_key(&fn_old) {
            continue;
        }
        let target_fn = format!("{prefix}/{fn_old}");
        if !Path::new(&target_fn).exists() {
            log::warn!("file {target_fn} does not exist");
            //progress_bar.println(format!("file {target_fn} does not exist!"));
            progress_bar.suspend(|| eprintln!("file {target_fn} does not exist!"));
            continue;
        }
        if installed_package
            .backup
            .clone()
            .unwrap_or_default()
            .contains(&fn_old)
        {
            let sum_current = get_md5_sum(&target_fn)?;
            if sum_current != *md5sum_old {
                // this file is not in the new version of package but user has altered it - keep a copy
                log::info!("saving changed {target_fn} as {target_fn}.atxpkg_save");
                //progress_bar.println(format!(
                //    "saving changed {target_fn} as {target_fn}.atxpkg_save"
                //));
                progress_bar.suspend(|| {
                    eprintln!("saving changed {target_fn} as {target_fn}.atxpkg_save");
                });
                move_file(&target_fn, &format!("{target_fn}.atxpkg_save"))?;
            } else {
                log::trace!("DF {target_fn}");
                try_delete(&target_fn)?;
            }
        } else {
            log::trace!("DF {target_fn}");
            try_delete(&target_fn)?;
        }
    }

    for dir_name in progress_bar.wrap_iter(dirs_old.into_iter().sorted_by_key(|x| x.len()).rev()) {
        if md5sums.contains_key(&dir_name) {
            continue;
        }

        let target_fn = format!("{prefix}/{dir_name}");
        if !Path::new(&target_fn).exists() {
            log::warn!("dir {target_fn} does not exist");
            //progress_bar.println(format!("{target_fn} does not exist!"));
            progress_bar.suspend(|| eprintln!("{target_fn} does not exist!"));
            continue;
        }

        let dir_path = Path::new(&target_fn);
        if dir_path != Path::new(&prefix) && is_empty_dir(dir_path)? {
            log::trace!("DD {target_fn}");
            std::fs::remove_dir(dir_path)?;
        }
    }

    progress_bar.finish();
    eprintln!();

    Ok(InstalledPackage {
        t: Some(UNIX_EPOCH.elapsed()?.as_secs_f64()),
        version: version_new.clone(),
        md5sums,
        backup,
    })
}

pub fn remove_packages(
    packages: Vec<String>,
    installed_packages: &mut HashMap<String, InstalledPackage>,
    prefix: &str,
    yes: bool,
    no: bool,
) -> anyhow::Result<bool> {
    for p in &packages {
        let (package_name, mut package_version) = split_package_name_version(p);
        let Some(installed_package) = installed_packages.get(&package_name) else {
            anyhow::bail!("package {package_name} not installed");
        };
        if !package_version.is_empty() {
            if package_version != installed_package.version {
                anyhow::bail!("package {package_name}-{package_version} not installed");
            }
        } else {
            package_version.clone_from(&installed_package.version);
        }

        println!("remove {package_name}-{package_version}");
    }
    if no || !(yes || yes_no("continue?", "n")?) {
        return Ok(false);
    }

    for p in &packages {
        let package_name = get_package_name(p);
        remove_package(
            &package_name,
            installed_packages[&package_name].clone(),
            prefix,
        )?;
        installed_packages.remove(&package_name);
    }

    Ok(true)
}

pub fn remove_package(
    package_name: &str,
    installed_package: InstalledPackage,
    prefix: &str,
) -> anyhow::Result<()> {
    let version = &installed_package.version;
    log::info!("removing {package_name}-{version}");

    let (mut dirs, mut files) = (vec![], vec![]);
    for (file_or_dir_name, md5sum) in installed_package.md5sums.iter() {
        if let Some(md5sum) = md5sum {
            files.push((file_or_dir_name, md5sum.clone()));
        } else {
            dirs.push(file_or_dir_name);
        }
    }

    let progress_bar = make_progress_bar(
        (dirs.len() + files.len()).try_into()?,
        package_name,
        "{spinner} {prefix}: remove [{wide_bar}] {pos}/{len}",
    )?;

    for (file_name, md5sum) in progress_bar.wrap_iter(files.into_iter()) {
        let target_fn = format!("{prefix}/{file_name}");
        if !Path::new(&target_fn).exists() {
            log::warn!("file {target_fn} does not exist!");
            //progress_bar.println(format!("{target_fn} does not exist!"));
            progress_bar.suspend(|| eprintln!("{target_fn} does not exist!"));
            continue;
        }

        if installed_package
            .backup
            .clone()
            .unwrap_or_default()
            .contains(file_name)
        {
            let current_sum = get_md5_sum(&target_fn)?;
            if current_sum != *md5sum {
                log::info!("{target_fn} changed, saving as {target_fn}.atxpkg_backup");
                //progress_bar.println(format!(
                //    "{target_fn} changed, saving as {target_fn}.atxpkg_backup"
                //));
                progress_bar.suspend(|| {
                    eprintln!("{target_fn} changed, saving as {target_fn}.atxpkg_backup");
                });
                move_file(&target_fn, &format!("{target_fn}.atxpkg_backup"))?;
            } else {
                log::trace!("DF {target_fn}");
                try_delete(&target_fn)?;
            }
        } else {
            log::trace!("DF {target_fn}");
            try_delete(&target_fn)?;
        }
    }

    for dir_name in progress_bar.wrap_iter(dirs.into_iter().sorted_by_key(|x| x.len()).rev()) {
        let target_fn = format!("{prefix}/{dir_name}");
        if !Path::new(&target_fn).exists() {
            log::warn!("dir {target_fn} does not exist!");
            //progress_bar.println(format!("{target_fn} does not exist!"));
            progress_bar.suspend(|| eprintln!("{target_fn} does not exist!"));
            continue;
        }

        let dir_path = Path::new(&target_fn);
        if dir_path != Path::new(&prefix) && is_empty_dir(dir_path)? {
            log::trace!("DD {target_fn}");
            std::fs::remove_dir(dir_path)?;
        }
    }

    progress_bar.finish();
    eprintln!();

    Ok(())
}

pub fn is_empty_dir(path: &Path) -> anyhow::Result<bool> {
    Ok(path.read_dir()?.next().is_none())
}

pub fn update_packages(
    packages: Vec<String>,
    installed_packages: &mut HashMap<String, InstalledPackage>,
    prefix: &str,
    repos: Vec<String>,
    force: bool,
    offline: bool,
    yes: bool,
    no: bool,
    download_only: bool,
    unverified_ssl: bool,
    cache_dir: &str,
    tmp_dir_prefix: &str,
) -> anyhow::Result<bool> {
    let mut package_updates = vec![];

    for p in &packages {
        let pu = if p.contains("..") {
            let package_parts = p.split("..").collect::<Vec<_>>();
            let (package_old, package_new) = (package_parts[0], package_parts[1]);
            let (package_name_old, package_version_old) = split_package_name_version(package_old);
            let (package_name_new, package_version_new) = split_package_name_version(package_new);
            PackageUpdate {
                name_old: package_name_old,
                version_old: package_version_old,
                name_new: package_name_new,
                version_new: package_version_new,
                url: String::new(),
                local_fn: String::new(),
            }
        } else {
            let (name, version) = split_package_name_version(p);
            PackageUpdate {
                name_old: name.clone(),
                version_old: String::new(),
                name_new: name,
                version_new: version,
                url: String::new(),
                local_fn: String::new(),
            }
        };
        package_updates.push(pu);
    }

    for pu in &mut package_updates {
        let Some(installed_package) = installed_packages.get(&pu.name_old) else {
            anyhow::bail!("package {} not installed", pu.name_old);
        };
        if pu.version_old.is_empty() {
            pu.version_old.clone_from(&installed_package.version);
        } else if pu.version_old != installed_package.version {
            anyhow::bail!("package {}-{} not installed", pu.name_old, pu.version_old);
        }
        if pu.name_old != pu.name_new && installed_packages.contains_key(&pu.name_new) {
            anyhow::bail!("package {} already installed", pu.name_new);
        }
    }

    let available_packages = get_available_packages(repos, offline, unverified_ssl)?;

    for pu in &mut package_updates {
        let Some(avail_pkg) = available_packages.get(&pu.name_new) else {
            anyhow::bail!("package {} not available", pu.name_new);
        };
        if pu.version_new.is_empty() {
            pu.version_new = get_max_version(avail_pkg.clone()).unwrap();
        }
        let url = get_specific_version_url(avail_pkg.clone(), &pu.version_new).unwrap();
        if url.is_empty() {
            anyhow::bail!("package {}-{} not available", pu.name_new, pu.version_new);
        }
        pu.url = url;
    }

    package_updates
        .retain(|pu| force || pu.name_old != pu.name_new || pu.version_old != pu.version_new);

    if package_updates.is_empty() {
        println!("nothing to update");
        return Ok(false);
    }

    for pu in &package_updates {
        println!(
            "update {}-{} -> {}-{}",
            pu.name_old, pu.version_old, pu.name_new, pu.version_new
        );
    }
    if no || !(yes || yes_no("continue?", "y")?) {
        return Ok(false);
    }

    let mb = indicatif::MultiProgress::new();

    let package_updates = {
        let (tx, rx) = std::sync::mpsc::channel();
        scoped_threadpool::Pool::new(MAX_CONCURRENT_DOWNLOADS).scoped(|scope| {
            for pu in &package_updates {
                let tx = &tx;
                let mb = &mb;
                scope.execute(move || {
                    let res = (|| {
                        let pb = make_progress_bar(
                            0,
                            &pu.name_new,
                            "{spinner} {prefix} [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )?;
                        mb.add(pb.clone());
                        let Ok(local_fn) =
                            download_package_if_needed(&pu.url, cache_dir, unverified_ssl, Some(&pb))
                        else {
                            pb.suspend(|| eprintln!("download failed"));
                            anyhow::bail!("download failed");
                        };
                        Ok(PackageUpdate {
                            name_old: pu.name_old.clone(),
                            version_old: pu.version_old.clone(),
                            name_new: pu.name_new.clone(),
                            version_new: pu.version_new.clone(),
                            url: pu.url.clone(),
                            local_fn,
                        })
                    })();
                    tx.send(res).unwrap();
                });
            };
        });
        drop(tx);
        let package_updates: Vec<_> = rx.iter().try_collect()?;
        anyhow::Ok(package_updates)
    }?;

    //mb.clear();
    eprintln!();

    if download_only {
        return Ok(false);
    }

    for pu in package_updates {
        let mut package_info = update_package(
            &pu.local_fn,
            &pu.name_old,
            installed_packages[&pu.name_old].clone(),
            prefix,
            force,
            tmp_dir_prefix,
        )?;

        package_info.t = Some(UNIX_EPOCH.elapsed()?.as_secs_f64());
        installed_packages.remove(&pu.name_old);
        installed_packages.insert(pu.name_new.clone(), package_info);
        log::info!(
            "{}-{} updated to {}-{}",
            pu.name_old,
            pu.version_old,
            pu.name_new,
            pu.version_new
        );
        println!(
            "{}-{} updated to {}-{}",
            pu.name_old, pu.version_old, pu.name_new, pu.version_new
        );
    }

    Ok(true)
}

fn check_package(package_name: &str, pkg: &InstalledPackage, prefix: &str) -> anyhow::Result<u32> {
    let mut res = vec![];

    let progress_bar = make_progress_bar(
        pkg.md5sums.len().try_into()?,
        package_name,
        "{spinner} {prefix} [{wide_bar}] {pos}/{len}",
    )?;

    let mut err_count = 0;
    for (fn_name, md5sum) in progress_bar.wrap_iter(pkg.md5sums.iter()) {
        let file_path = format!("{prefix}/{fn_name}");
        if !Path::new(&file_path).exists() {
            res.push(format!("{package_name}: does not exist: {file_path}"));
            err_count += 1;
        }
        if let Some(md5sum) = md5sum {
            if pkg.backup.clone().unwrap_or_default().contains(fn_name) {
                continue;
            }
            if let Ok(current_md5sum) = get_md5_sum(&file_path) {
                if current_md5sum != *md5sum {
                    res.push(format!("{package_name}: checksum difference: {file_path}"));
                    err_count += 1;
                }
            }
        }
    }

    progress_bar.finish();
    eprintln!();

    for r in res {
        println!("{r}");
    }

    Ok(err_count)
}

pub fn check_packages(
    packages: Vec<String>,
    installed_packages: &HashMap<String, InstalledPackage>,
    prefix: &str,
) -> anyhow::Result<()> {
    for package in &packages {
        let (package_name, package_version) = split_package_name_version(package);
        if !installed_packages.contains_key(&package_name)
            || (!package_version.is_empty()
                && package_version != installed_packages[&package_name].version)
        {
            anyhow::bail!("{package_name} not installed");
        }
    }

    let mut err_count = 0;
    for package in &packages {
        let package_name = split_package_name_version(package).0;
        if let Some(installed_package) = installed_packages.get(&package_name) {
            err_count += check_package(package, installed_package, prefix)?;
        }
    }

    if err_count > 0 {
        anyhow::bail!("error count: {err_count}");
    }

    Ok(())
}

fn gen_fn_to_package_name_mapping(
    installed_packages: &HashMap<String, InstalledPackage>,
) -> HashMap<String, String> {
    let mut fn_to_package_name = HashMap::new();
    for (package_name, pkginfo) in installed_packages {
        for fn_name in pkginfo.md5sums.keys() {
            fn_to_package_name.insert(fn_name.clone(), package_name.clone());
        }
    }
    fn_to_package_name
}

pub fn get_untracked(
    paths: Vec<String>,
    installed_packages: &HashMap<String, InstalledPackage>,
    prefix: &str,
) -> anyhow::Result<Vec<String>> {
    let fn_to_package_name = gen_fn_to_package_name_mapping(installed_packages);

    let paths = if paths.is_empty() {
        let mut first_dirs = HashSet::new();
        for fn_ in fn_to_package_name.keys() {
            if let Some((first_dir, _)) = fn_.split_once('/') {
                first_dirs.insert(first_dir.to_string());
            } else {
                first_dirs.insert(fn_.to_string());
            }
        }
        let first_dirs = first_dirs.into_iter().collect::<Vec<_>>();
        log::debug!("first_dirs: {first_dirs:?}");
        first_dirs
    } else {
        paths
    };

    let mut ret = Vec::new();

    for path in paths.into_iter() {
        let full_path = format!("{prefix}/{path}");
        let (dirs, files) = get_recursive_listing(&full_path)?;

        let progress_bar = make_progress_bar(
            (dirs.len() + files.len()).try_into()?,
            &path,
            "{spinner} {prefix} [{wide_bar}] {pos}/{len}",
        )?;

        for dir_name in progress_bar.wrap_iter(dirs.into_iter()) {
            let full_dir_name = format!("{path}/{dir_name}");
            if !fn_to_package_name.contains_key(&full_dir_name) {
                ret.push(full_dir_name);
            }
        }
        for fn_name in progress_bar.wrap_iter(files.into_iter()) {
            let full_fn_name = format!("{path}/{fn_name}");
            if !fn_to_package_name.contains_key(&full_fn_name) {
                ret.push(full_fn_name);
            }
        }

        progress_bar.finish();
        eprintln!();
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_md5_sum() {
        assert_eq!(
            get_md5_sum("./test_data/atxpkg-1.5-3.atxpkg.zip").unwrap(),
            "456e4527d437e2f3cbbe0e9311bc5a13"
        )
    }

    #[test]
    fn test_get_max_version() {
        assert_eq!(
            get_max_version(vec![
                "http://atxpkg.asterix.cz/neco.dev-20240722223041-1.atxpkg.zip".to_string(),
                "http://atxpkg-dev.asterix.cz/neco.dev-20240722223042-1.atxpkg.zip".to_string(),
                "/neco/na/disku/neco.dev-20240722223043-1.atxpkg.zip".to_string(),
            ]),
            Some("20240722223043-1".to_string())
        )
    }

    #[test]
    fn test_get_recursive_listing() {
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();
        let tmp_dir_str = tmp_dir.path().to_str().unwrap();

        std::fs::create_dir(format!("{tmp_dir_str}/test")).unwrap();
        std::fs::write(format!("{tmp_dir_str}/test/new"), "x\n").unwrap();

        let (dirs, files) = get_recursive_listing(tmp_dir_str).unwrap();

        assert_eq!(dirs, vec!["test"]);
        assert_eq!(files, vec!["test/new"]);

        std::fs::create_dir(format!("{tmp_dir_str}/some_prefix")).unwrap();
        std::fs::create_dir(format!("{tmp_dir_str}/some_prefix/test")).unwrap();
        std::fs::write(format!("{tmp_dir_str}/some_prefix/test/new"), "x\n").unwrap();

        let (dirs, files) = get_recursive_listing(&format!("{tmp_dir_str}/some_prefix")).unwrap();

        assert_eq!(dirs, vec!["test"]);
        assert_eq!(files, vec!["test/new"]);
    }

    #[test]
    fn test_install_package() {
        let dest_dir = tempfile::Builder::new().tempdir().unwrap();
        let dest_dir_str = dest_dir.path().to_str().unwrap().to_string();
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

        let pkginfo = install_package(
            "./test_data/atx300-base-6.3-1.atxpkg.zip",
            &dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(pkginfo.version, "6.3-1");
        assert!(Path::new(&format!("{dest_dir_str}/atx300/memsh.mem")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/atx300/.atxpkg_backup")).exists());
    }

    #[test]
    fn test_install_update_package_with_conflict() {
        let dest_dir = tempfile::Builder::new().tempdir().unwrap();
        let dest_dir_str = dest_dir.path().to_str().unwrap();
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

        let pkginfo = install_package(
            "./test_data/test-1.0-1.atxpkg.zip",
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(pkginfo.version, "1.0-1");

        std::fs::write(format!("{dest_dir_str}/test/new"), "x\n").unwrap();

        let pkginfo = update_package(
            "./test_data/test-2.0-1.atxpkg.zip",
            "test",
            pkginfo,
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        );
        assert!(pkginfo.is_err());
    }

    #[test]
    fn test_install_update_package_with_backup() {
        let dest_dir = tempfile::Builder::new().tempdir().unwrap();
        let dest_dir_str = dest_dir.path().to_str().unwrap();
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

        let pkginfo = install_package(
            "./test_data/test-1.0-1.atxpkg.zip",
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(pkginfo.version, "1.0-1");

        std::fs::write(format!("{dest_dir_str}/test/protected1"), "x\n").unwrap();
        std::fs::write(format!("{dest_dir_str}/test/protected2"), "2\n").unwrap();
        std::fs::write(format!("{dest_dir_str}/test/unprotected2"), "2\n").unwrap();

        let pkginfo = update_package(
            "./test_data/test-2.0-1.atxpkg.zip",
            "test",
            pkginfo,
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(pkginfo.version, "2.0-1");

        assert!(Path::new(&format!("{dest_dir_str}/test/protected1.atxpkg_new")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/test/protected2.atxpkg_new")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/test/protected3.atxpkg_new")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/test/unprotected.atxpkg_new")).exists());
    }

    #[test]
    fn test_install_remove_with_backup() {
        let dest_dir = tempfile::Builder::new().tempdir().unwrap();
        let dest_dir_str = dest_dir.path().to_str().unwrap();
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

        let pkginfo = install_package(
            "./test_data/test-1.0-1.atxpkg.zip",
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        assert_eq!(pkginfo.version, "1.0-1");

        std::fs::write(format!("{dest_dir_str}/test/protected1"), "x\n").unwrap();
        std::fs::write(format!("{dest_dir_str}/test/protected2"), "2\n").unwrap();
        std::fs::write(format!("{dest_dir_str}/test/unprotected2"), "2\n").unwrap();

        remove_package("test", pkginfo, dest_dir_str).unwrap();

        assert!(Path::new(&format!("{dest_dir_str}/test/protected1.atxpkg_backup")).exists());
        assert!(Path::new(&format!("{dest_dir_str}/test/protected2.atxpkg_backup")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/test/protected3.atxpkg_backup")).exists());
        assert!(!Path::new(&format!("{dest_dir_str}/test/unprotected.atxpkg_backup")).exists());
    }

    #[test]
    fn test_get_untracked() {
        let dest_dir = tempfile::Builder::new().tempdir().unwrap();
        let dest_dir_str = dest_dir.path().to_str().unwrap();
        let tmp_dir = tempfile::Builder::new().tempdir().unwrap();

        let pkginfo = install_package(
            "./test_data/test-1.0-1.atxpkg.zip",
            dest_dir_str,
            false,
            tmp_dir.path().to_str().unwrap(),
        )
        .unwrap();

        let mut installed_packages = HashMap::new();
        installed_packages.insert("test".to_string(), pkginfo);

        let untracked =
            get_untracked(vec!["test".to_string()], &installed_packages, dest_dir_str).unwrap();
        assert!(untracked.is_empty());

        std::fs::write(format!("{dest_dir_str}/test/extra_file"), "some content").unwrap();

        let untracked =
            get_untracked(vec!["test".to_string()], &installed_packages, dest_dir_str).unwrap();
        assert_eq!(untracked, vec!["test/extra_file".to_string()]);
    }
}
