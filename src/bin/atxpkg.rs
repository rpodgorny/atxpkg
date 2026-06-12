use std::{path::Path, process::ExitCode};

use atxpkg::*;
use clap::{Args, Parser, Subcommand};
use itertools::Itertools;

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct MainArgs {
    #[command(subcommand)]
    command: Command,
    /// Path prefix.
    #[cfg(target_os = "linux")]
    #[arg(long, default_value = "/")]
    prefix: String,
    #[cfg(target_os = "windows")]
    #[arg(long, default_value = "c:/")]
    prefix: String,
    /// Repository url (can be specified multiple times). When specified, repos.txt is not loaded.
    #[arg(long = "repo")]
    repos: Vec<String>,
    /// Enable debug mode.
    #[arg(long, default_value = "false")]
    debug: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Install packages.
    Install(InstallArgs),
    /// Update packages.
    Update(InstallArgs),
    /// Install or update packages (install if not installed, update if installed).
    Upstall(InstallArgs),
    /// Remove packages.
    Remove(InstallArgs),
    /// Check packages.
    Check(CheckArgs),
    /// List available packages.
    #[command(name = "list_available")]
    ListAvailable(ListAvailableArgs),
    /// List installed packages.
    #[command(name = "list_installed")]
    ListInstalled,
    /// Show untracked files.
    #[command(name = "show_untracked")]
    ShowUntracked(ShowUntrackedArgs),
    /// Clean cache.
    #[command(name = "clean_cache")]
    CleanCache,
}

#[derive(Args, Debug)]
struct InstallArgs {
    /// Packages
    packages: Vec<String>,
    /// Force operation (overwrite files etc.)
    #[arg(short = 'f', long, default_value = "false")]
    force: bool,
    /// Only download packages, don't install/update anything.
    #[arg(short = 'w', long, default_value = "false")]
    downloadonly: bool,
    /// Automatically answer yes to all questions.
    #[arg(short = 'y', long, default_value = "false")]
    yes: bool,
    /// Automatically answer no to all questions.
    #[arg(short = 'n', long, default_value = "false")]
    no: bool,
    /// Don't connect to online repositories.
    #[arg(long, default_value = "false")]
    offline: bool,
    /// Only perform install/update/remove if listed packages are installed.
    #[arg(long)]
    if_installed: Option<String>,
    /// Don't verify ssl certificate validity.
    #[arg(long, default_value = "false")]
    unverified_ssl: bool,
}

#[derive(Args, Debug)]
struct ListAvailableArgs {
    /// Packages
    packages: Vec<String>,
    /// Don't connect to online repositories.
    #[arg(long, default_value = "false")]
    offline: bool,
    /// Don't verify ssl certificate validity.
    #[arg(long, default_value = "false")]
    unverified_ssl: bool,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Packages
    packages: Vec<String>,
}

#[derive(Args, Debug)]
struct ShowUntrackedArgs {
    /// Paths
    paths: Vec<String>,
}

// TODO: cut-n-pasted from router and modified - unite!
fn log_init(
    fn_: Option<&str>,
    level: Option<&str>,
    show: bool,
    lenient: bool,
) -> anyhow::Result<()> {
    let log_level_term = if let Some(level) = level {
        level.to_string()
    } else if let Ok(level) = std::env::var("RUST_LOG") {
        level.to_string()
    } else {
        "debug".to_string()
    };
    let log_level_term = match log_level_term.as_str() {
        "trace" => simplelog::LevelFilter::Trace,
        "info" => simplelog::LevelFilter::Info,
        "warn" => simplelog::LevelFilter::Warn,
        "error" => simplelog::LevelFilter::Error,
        _ => simplelog::LevelFilter::Debug,
    };
    let log_level_file = simplelog::LevelFilter::Debug;
    let log_config = simplelog::ConfigBuilder::new()
        .set_time_format_custom(time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
        ))
        .set_time_offset_to_local()
        .map_err(|_e| anyhow::anyhow!("failed to set time offset to local"))?
        .build();
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![];
    if show {
        let termlogger = simplelog::TermLogger::new(
            log_level_term,
            log_config.clone(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        );
        loggers.push(termlogger);
    }
    if let Some(fn_) = fn_ {
        match std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(fn_)
        {
            Ok(file) => {
                let filelogger = simplelog::WriteLogger::new(log_level_file, log_config, file);
                loggers.push(filelogger);
            }
            Err(e) if lenient => {
                eprintln!(
                    "WARNING: failed to open log file {fn_}: {e}, continuing without file logging"
                );
            }
            Err(e) => return Err(e.into()),
        }
    }
    if !loggers.is_empty() {
        simplelog::CombinedLogger::init(loggers)?;
    }

    log_panics::init();

    Ok(())
}

fn main_sub() -> anyhow::Result<u8> {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let mainargs = MainArgs::parse();

    #[cfg(target_os = "linux")]
    let root_dir = "/tmp/atxpkg";
    #[cfg(target_os = "windows")]
    let root_dir = "c:/atxpkg";

    // on linux a missing root dir is tolerated (warn and continue); on other
    // platforms (windows) the folder is required.
    let lenient = std::env::consts::OS == "linux";

    let log_fn = format!("{root_dir}/atxpkg.log");

    log_init(Some(&log_fn), Some("debug"), mainargs.debug, lenient)?;

    log::info!("starting atxpkg v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("starting atxpkg v{}", env!("CARGO_PKG_VERSION"));

    log::debug!("args: {mainargs:#?}");

    let root_dir_exists = Path::new(root_dir).exists();
    if !root_dir_exists {
        if !lenient {
            anyhow::bail!("root dir {root_dir} does not exist");
        }
        log::warn!("root dir {root_dir} does not exist, continuing");
        eprintln!("WARNING: root dir {root_dir} does not exist, continuing");
    }
    if !Path::new(&mainargs.prefix).exists() {
        anyhow::bail!("prefix dir {} does not exist", &mainargs.prefix);
    }

    let cache_dir = format!("{root_dir}/cache");
    let tmp_dir_prefix = format!("{root_dir}/tmp");
    let db_fn = format!("{root_dir}/installed.json");
    let repos_fn = format!("{root_dir}/repos.txt");

    if root_dir_exists && !Path::new(&cache_dir).exists() {
        log::info!("creating cache dir {cache_dir}");
        std::fs::create_dir(&cache_dir)?;
    }
    if root_dir_exists && !Path::new(&tmp_dir_prefix).exists() {
        log::info!("creating tmp dir {tmp_dir_prefix}");
        std::fs::create_dir(&tmp_dir_prefix)?;
    }

    let mut repos = vec![cache_dir.clone()];
    if !mainargs.repos.is_empty() {
        repos.extend(mainargs.repos.iter().cloned());
    } else if Path::new(&repos_fn).exists() {
        for line in std::fs::read_to_string(&repos_fn)?.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                repos.push(line.to_string());
            }
        }
    } else {
        repos.push("https://atxpkg.asterix.cz".to_string());
        repos.push("https://atxpkg-dev.asterix.cz".to_string());
    }
    log::debug!("repos: {repos:?}");

    match &mainargs.command {
        Command::Install(args) => {
            let mut installed_packages = get_installed_packages(&db_fn)?;
            if let Some(if_installed_) = &args.if_installed {
                if_installed(
                    if_installed_.split(',').map(|x| x.to_string()).collect(),
                    &installed_packages,
                )?;
            }
            let res = install_packages(
                args.packages.to_vec(),
                &mut installed_packages,
                &mainargs.prefix,
                repos,
                args.force,
                args.offline,
                args.yes,
                args.no,
                args.downloadonly,
                args.unverified_ssl,
                &cache_dir,
                &tmp_dir_prefix,
            );
            save_installed_packages(&installed_packages, &db_fn)?;
            if res? {
                log::info!("install completed");
                println!("install completed");
            }
        }
        Command::Update(args) => {
            let mut installed_packages = get_installed_packages(&db_fn)?;
            if let Some(if_installed_) = &args.if_installed {
                if_installed(
                    if_installed_.split(',').map(|x| x.to_string()).collect(),
                    &installed_packages,
                )?;
            }
            let packages = if args.packages.is_empty() {
                installed_packages
                    .keys()
                    .map(|x| x.to_string())
                    .sorted()
                    .collect()
            } else {
                args.packages.to_vec()
            };
            let res = update_packages(
                packages,
                &mut installed_packages,
                &mainargs.prefix,
                repos,
                args.force,
                args.offline,
                args.yes,
                args.no,
                args.downloadonly,
                args.unverified_ssl,
                &cache_dir,
                &tmp_dir_prefix,
            );
            save_installed_packages(&installed_packages, &db_fn)?;
            if res? {
                log::info!("update completed");
                println!("update completed");
            }
        }
        Command::Upstall(args) => {
            let mut installed_packages = get_installed_packages(&db_fn)?;
            if let Some(if_installed_) = &args.if_installed {
                if_installed(
                    if_installed_.split(',').map(|x| x.to_string()).collect(),
                    &installed_packages,
                )?;
            }
            let res = upstall_packages(
                args.packages.to_vec(),
                &mut installed_packages,
                &mainargs.prefix,
                repos,
                args.force,
                args.offline,
                args.yes,
                args.no,
                args.downloadonly,
                args.unverified_ssl,
                &cache_dir,
                &tmp_dir_prefix,
            );
            save_installed_packages(&installed_packages, &db_fn)?;
            if res? {
                log::info!("upstall completed");
                println!("upstall completed");
            }
        }
        Command::Remove(args) => {
            let mut installed_packages = get_installed_packages(&db_fn)?;
            if let Some(if_installed_) = &args.if_installed {
                if_installed(
                    if_installed_.split(',').map(|x| x.to_string()).collect(),
                    &installed_packages,
                )?;
            }
            let res = remove_packages(
                args.packages.to_vec(),
                &mut installed_packages,
                &mainargs.prefix,
                args.yes,
                args.no,
            );
            save_installed_packages(&installed_packages, &db_fn)?;
            if res? {
                log::info!("remove completed");
                println!("remove completed");
            }
        }
        Command::Check(args) => {
            let installed_packages = get_installed_packages(&db_fn)?;
            let packages = if args.packages.is_empty() {
                installed_packages
                    .keys()
                    .map(|x| x.to_string())
                    .sorted()
                    .collect()
            } else {
                args.packages.to_vec()
            };
            check_packages(packages, &installed_packages, &mainargs.prefix)?;
        }
        Command::ListAvailable(args) => {
            for (package_name, package_ver) in list_available(
                args.packages.to_vec(),
                repos,
                args.offline,
                args.unverified_ssl,
            )? {
                if package_ver.is_empty() {
                    println!("{package_name}");
                } else {
                    println!("{package_name}-{package_ver}");
                }
            }
        }
        Command::ListInstalled => {
            let installed_packages = get_installed_packages(&db_fn)?;
            for (package_name, package_info) in
                installed_packages.iter().sorted_unstable_by_key(|x| x.0)
            {
                println!("{package_name}-{}", package_info.version);
            }
        }
        Command::ShowUntracked(args) => {
            let installed_packages = get_installed_packages(&db_fn)?;
            let untracked =
                get_untracked(args.paths.clone(), &installed_packages, &mainargs.prefix)?;
            for x in untracked {
                println!("unknown: {x}");
            }
        }
        Command::CleanCache => {
            clean_cache(&cache_dir)?;
        }
    }

    Ok(0)
}

fn main() -> ExitCode {
    //ExitCode::from(main_sub().unwrap())
    let res = main_sub();
    if let Err(err) = &res {
        eprintln!("{err}");
        //eprintln!("{}", err.backtrace());
    }
    ExitCode::from(res.unwrap())
}
