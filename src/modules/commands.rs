use crate::modules::{
    cli::{IssueCertArgs, WriteProxyArgs},
    env::{
        resolve_cert_dir, resolve_optional_path, resolve_optional_value, resolve_path,
        resolve_resolvers, resolve_value,
    },
    log::{info, step, success},
    templates::{NGINX_DEFAULT_TEMPLATE, NGINX_PROXY_TEMPLATE},
};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const DEFAULT_RESOLVER: &str = "1.1.1.1 1.0.0.1 [2606:4700:4700::1111] [2606:4700:4700::1064]";

pub fn setup_system(
    install_zsh: bool,
    install_cron: bool,
    install_nginx: bool,
    dry_run: bool,
) -> Result<(), String> {
    step("System setup");
    ensure_root()?;
    let start = Instant::now();
    let mut changes: Vec<String> = Vec::new();

    if install_zsh {
        if command_exists("zsh") {
            info("zsh is already installed");
        } else if confirm_with_timeout("Install zsh?", DEFAULT_CONFIRM_TIMEOUT, dry_run)? {
            install_if_missing("zsh", &mut changes, dry_run, |dry| {
                run_cmd("apt-get", &["update", "-qq"], dry)?;
                run_cmd("apt-get", &["install", "-y", "zsh"], dry)
            })?;
        } else {
            info("zsh install skipped");
        }
    }

    if install_cron {
        install_if_missing("crontab", &mut changes, dry_run, |dry| {
            run_cmd("apt-get", &["update", "-qq"], dry)?;
            run_cmd("apt-get", &["install", "-y", "cron"], dry)?;
            run_cmd("systemctl", &["enable", "cron"], dry)?;
            run_cmd("systemctl", &["start", "cron"], dry)
        })?;
    }

    if install_nginx {
        install_if_missing("nginx", &mut changes, dry_run, |dry| {
            install_nginx_official(dry)
        })?;
    }

    print_summary(&changes, start.elapsed());
    Ok(())
}

pub fn issue_cert(
    env_overrides: &HashMap<String, String>,
    args: IssueCertArgs,
    reload_nginx: bool,
    dry_run: bool,
) -> Result<(), String> {
    step("Issuing certificate");
    ensure_root()?;
    let cf_token = resolve_value(
        args.cf_token,
        env_overrides,
        "CF_TOKEN",
        "Cloudflare token",
        true,
    )?;
    let cf_account_id = resolve_value(
        args.cf_account_id,
        env_overrides,
        "CF_ACCOUNT_ID",
        "Cloudflare account ID",
        false,
    )?;
    let cf_zone_id = resolve_value(
        args.cf_zone_id,
        env_overrides,
        "CF_ZONE_ID",
        "Cloudflare zone ID",
        false,
    )?;
    let domain = resolve_value(
        args.domain,
        env_overrides,
        "DOMAIN",
        "Primary domain (e.g., example.com)",
        false,
    )?;
    let wildcard_domain = resolve_optional_value(
        args.wildcard_domain,
        env_overrides,
        "WILDCARD_DOMAIN",
        "Wildcard domain (e.g., *.example.com)",
        false,
    )?
    .unwrap_or_else(|| format!("*.{}", domain));

    let acme_bin = resolve_path(
        args.acme_bin,
        env_overrides,
        "ACME_BIN",
        "/root/.acme.sh/acme.sh",
        "acme.sh path",
    )?;
    let acme_home = resolve_path(
        args.acme_home,
        env_overrides,
        "ACME_HOME",
        "/root/.acme.sh",
        "acme home directory",
    )?;
    let cert_dir = resolve_optional_path(args.cert_dir, env_overrides, "CERT_DIR");
    let cert_dir = resolve_cert_dir(
        cert_dir,
        args.cert_dir_name,
        env_overrides,
        &["CERT_DIR_NAME"],
        "custom",
    )?;
    let cert_output_path =
        resolve_optional_path(args.cert_output_path, env_overrides, "CERT_OUTPUT_PATH");
    let key_output_path =
        resolve_optional_path(args.key_output_path, env_overrides, "KEY_OUTPUT_PATH");
    if cert_output_path.is_some() ^ key_output_path.is_some() {
        return Err("Both CERT_OUTPUT_PATH and KEY_OUTPUT_PATH must be set together".to_string());
    }
    let nginx_bin = resolve_path(
        args.nginx_bin,
        env_overrides,
        "NGINX_BIN",
        "nginx",
        "nginx binary",
    )?;

    let cache_dir = acme_home.join(format!("{}_ecc", domain));
    if dry_run {
        info(&format!(
            "[dry-run] Would remove cache dir if exists: {}",
            cache_dir.display()
        ));
    } else if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to remove cache dir {}: {e}", cache_dir.display()))?;
    }

    let mut acme_cmd = Command::new(&acme_bin);
    acme_cmd
        .env("CF_Token", cf_token)
        .env("CF_Account_ID", cf_account_id)
        .env("CF_Zone_ID", cf_zone_id)
        .arg("--issue")
        .arg("--force")
        .arg("-d")
        .arg(&domain)
        .arg("-d")
        .arg(&wildcard_domain)
        .arg("--dns")
        .arg("dns_cf")
        .arg("--keylength")
        .arg("ec-256")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if dry_run {
        info("[dry-run] Would run acme.sh to issue certificate");
    } else {
        let status = acme_cmd
            .status()
            .map_err(|e| format!("Failed to run acme.sh: {e}"))?;
        if !status.success() {
            return Err("Certificate issuance failed".to_string());
        }
        success("Certificate issuance completed");
    }

    let cert_src = cache_dir.join("fullchain.cer");
    let key_src = cache_dir.join(format!("{}.key", domain));
    let (cert_dst, key_dst) = match (cert_output_path, key_output_path) {
        (Some(cert_path), Some(key_path)) => (cert_path, key_path),
        _ => (
            cert_dir.join(format!("{}.cer", domain)),
            cert_dir.join(format!("{}.key", domain)),
        ),
    };

    let cert_parent_display = cert_dst
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/".to_string());
    if dry_run {
        info(&format!(
            "[dry-run] Would create cert dir: {}",
            cert_parent_display
        ));
    } else if let Some(parent) = cert_dst.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }

    if dry_run {
        info(&format!(
            "[dry-run] Would copy cert: {} -> {}",
            cert_src.display(),
            cert_dst.display()
        ));
        info(&format!(
            "[dry-run] Would copy key: {} -> {}",
            key_src.display(),
            key_dst.display()
        ));
    } else {
        fs::copy(&cert_src, &cert_dst)
            .map_err(|e| format!("Failed to copy cert from {}: {e}", cert_src.display()))?;
        fs::copy(&key_src, &key_dst)
            .map_err(|e| format!("Failed to copy key from {}: {e}", key_src.display()))?;
        success("Certificate files updated");
    }

    if reload_nginx {
        if dry_run {
            info("[dry-run] Would run nginx -t and reload");
        } else {
            let status = Command::new(&nginx_bin)
                .arg("-t")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| format!("Failed to run nginx -t: {e}"))?;
            if !status.success() {
                return Err("nginx -t failed".to_string());
            }

            let status = Command::new(&nginx_bin)
                .arg("-s")
                .arg("reload")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| format!("Failed to reload nginx: {e}"))?;
            if !status.success() {
                return Err("nginx reload failed".to_string());
            }
            success("nginx reloaded");
        }
    }

    setup_acme_renew_cron(&acme_bin, &acme_home, dry_run)?;

    Ok(())
}

pub fn write_nginx_default(
    env_overrides: &HashMap<String, String>,
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
    cert_dir_name: Option<String>,
    domain: Option<String>,
    output_path: Option<PathBuf>,
    dry_run: bool,
) -> Result<(), String> {
    let cert_path = resolve_optional_path(cert_path, env_overrides, "NGINX_CERT_PATH");
    let key_path = resolve_optional_path(key_path, env_overrides, "NGINX_KEY_PATH");
    let needs_domain = cert_path.is_none() || key_path.is_none();
    let domain = if needs_domain {
        Some(resolve_value(
            domain,
            env_overrides,
            "DOMAIN",
            "Primary domain (e.g., example.com)",
            false,
        )?)
    } else {
        None
    };
    let cert_dir = if needs_domain {
        Some(resolve_cert_dir(
            None,
            cert_dir_name,
            env_overrides,
            &["NGINX_CERT_DIR_NAME", "CERT_DIR_NAME"],
            "custom",
        )?)
    } else {
        None
    };
    let (cert_path, key_path) = resolve_cert_paths(cert_path, key_path, cert_dir, domain)?;
    let output_path = resolve_path(
        output_path,
        env_overrides,
        "NGINX_DEFAULT_OUTPUT",
        "/etc/nginx/conf.d/default/00-default.conf",
        "nginx default output path",
    )?;

    step("Writing nginx default config");
    if let Some(parent) = output_path.parent() {
        if dry_run {
            info(&format!(
                "[dry-run] Would create directory: {}",
                parent.display()
            ));
        } else {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
    }

    let content = NGINX_DEFAULT_TEMPLATE
        .replace("{{CERT_PATH}}", &cert_path.display().to_string())
        .replace("{{KEY_PATH}}", &key_path.display().to_string());

    if dry_run {
        info(&format!(
            "[dry-run] Would write nginx default config to: {}",
            output_path.display()
        ));
    } else {
        fs::write(&output_path, content)
            .map_err(|e| format!("Failed to write {}: {e}", output_path.display()))?;
        success("nginx default config written");
    }
    Ok(())
}

pub fn write_proxy_config(
    env_overrides: &HashMap<String, String>,
    args: WriteProxyArgs,
    dry_run: bool,
) -> Result<(), String> {
    step("Writing reverse proxy config");
    let proxy_domain = resolve_value(
        args.proxy_domain,
        env_overrides,
        "PROXY_DOMAIN",
        "Proxy domain (e.g., proxy.example.com)",
        false,
    )?;
    let backend_url = resolve_value(
        args.backend_url,
        env_overrides,
        "BACKEND_URL",
        "Backend URL (e.g., https://emby.example.com:443)",
        false,
    )?;

    let resolver = resolve_resolvers(&args.resolvers, env_overrides, "RESOLVER", DEFAULT_RESOLVER)?;

    let cert_path = resolve_optional_path(args.cert_path, env_overrides, "NGINX_CERT_PATH");
    let key_path = resolve_optional_path(args.key_path, env_overrides, "NGINX_KEY_PATH");
    let needs_domain = cert_path.is_none() || key_path.is_none();
    let domain = if needs_domain {
        Some(resolve_value(
            Some(proxy_domain.clone()),
            env_overrides,
            "DOMAIN",
            "Primary domain (e.g., example.com)",
            false,
        )?)
    } else {
        None
    };
    let cert_dir = if needs_domain {
        Some(resolve_cert_dir(
            resolve_optional_path(args.cert_dir, env_overrides, "CERT_DIR"),
            args.cert_dir_name,
            env_overrides,
            &["NGINX_CERT_DIR_NAME", "CERT_DIR_NAME"],
            "custom",
        )?)
    } else {
        None
    };
    let (cert_path, key_path) = resolve_cert_paths(cert_path, key_path, cert_dir, domain)?;

    let output_dir = resolve_path(
        args.output_dir,
        env_overrides,
        "PROXY_OUTPUT_DIR",
        "/etc/nginx/conf.d/proxy",
        "proxy config output dir",
    )?;
    let output_path = output_dir.join(format!("{}.conf", proxy_domain.replace('.', "-")));

    let content = NGINX_PROXY_TEMPLATE
        .replace("{{PROXY_DOMAIN}}", &proxy_domain)
        .replace("{{BACKEND_URL}}", &backend_url)
        .replace("{{CERT_PATH}}", &cert_path.display().to_string())
        .replace("{{KEY_PATH}}", &key_path.display().to_string())
        .replace("{{RESOLVER}}", &resolver);

    if dry_run {
        info(&format!(
            "[dry-run] Would write proxy config to: {}",
            output_path.display()
        ));
        return Ok(());
    }

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create {}: {e}", output_dir.display()))?;
    fs::write(&output_path, content)
        .map_err(|e| format!("Failed to write {}: {e}", output_path.display()))?;
    success("reverse proxy config written");
    Ok(())
}

pub fn print_params_table() -> Result<(), String> {
    step("Supported parameters");
    let rows = vec![
        (
            "--env KEY=VALUE",
            "Override environment values (repeatable)",
        ),
        ("setup", "Install zsh/cron/nginx if missing"),
        ("--install-zsh", "Install zsh if missing"),
        ("--install-cron", "Install cron if missing"),
        ("--install-nginx", "Install nginx if missing"),
        ("--dry-run", "Simulate actions without changes"),
        ("issue-cert", "Issue certs and optionally reload nginx"),
        ("--cf-token", "Cloudflare token"),
        ("CF_TOKEN", "Cloudflare token (env)"),
        ("--cf-account-id", "Cloudflare account ID"),
        ("CF_ACCOUNT_ID", "Cloudflare account ID (env)"),
        ("--cf-zone-id", "Cloudflare zone ID"),
        ("CF_ZONE_ID", "Cloudflare zone ID (env)"),
        ("--domain", "Primary domain"),
        ("DOMAIN", "Primary domain (env)"),
        ("--wildcard-domain", "Wildcard domain"),
        ("WILDCARD_DOMAIN", "Wildcard domain (env)"),
        ("--acme-bin", "acme.sh path"),
        ("ACME_BIN", "acme.sh path (env)"),
        ("--acme-home", "acme home directory"),
        ("ACME_HOME", "acme home directory (env)"),
        ("--cert-dir", "Certificate directory (absolute path)"),
        ("CERT_DIR", "Certificate directory (env)"),
        ("--cert-dir-name", "Certificate directory name"),
        ("CERT_DIR_NAME", "Certificate directory name (env)"),
        ("--cert-output-path", "Certificate output path"),
        ("CERT_OUTPUT_PATH", "Certificate output path (env)"),
        ("--key-output-path", "Key output path"),
        ("KEY_OUTPUT_PATH", "Key output path (env)"),
        ("--nginx-bin", "nginx binary"),
        ("NGINX_BIN", "nginx binary (env)"),
        ("--reload-nginx", "Reload nginx after issuance"),
        ("--dry-run", "Simulate actions without changes"),
        ("write-nginx-default", "Write default nginx 444 config"),
        ("--cert-path", "Nginx cert path (absolute)"),
        ("NGINX_CERT_PATH", "Nginx cert path (env)"),
        ("--key-path", "Nginx key path (absolute)"),
        ("NGINX_KEY_PATH", "Nginx key path (env)"),
        ("--cert-dir-name", "Certificate directory name"),
        ("NGINX_CERT_DIR_NAME", "Certificate dir name (env)"),
        ("--domain", "Primary domain (used for default cert/key)"),
        ("DOMAIN", "Primary domain (env)"),
        ("--output-path", "Output path for default config"),
        (
            "NGINX_DEFAULT_OUTPUT",
            "Output path for default config (env)",
        ),
        ("--dry-run", "Simulate actions without changes"),
        ("write-proxy-config", "Write reverse proxy config"),
        ("--proxy-domain", "Proxy domain"),
        ("PROXY_DOMAIN", "Proxy domain (env)"),
        ("--backend-url", "Backend URL"),
        ("BACKEND_URL", "Backend URL (env)"),
        ("--resolver", "DNS resolver (repeatable)"),
        ("RESOLVER", "DNS resolver list (env or interactive)"),
        ("--cert-path", "Nginx cert path (absolute)"),
        ("NGINX_CERT_PATH", "Nginx cert path (env)"),
        ("--key-path", "Nginx key path (absolute)"),
        ("NGINX_KEY_PATH", "Nginx key path (env)"),
        ("--cert-dir", "Certificate directory (absolute path)"),
        ("CERT_DIR", "Certificate directory (env)"),
        ("--cert-dir-name", "Certificate directory name"),
        ("CERT_DIR_NAME", "Certificate directory name (env)"),
        ("--output-dir", "Proxy config output dir"),
        ("PROXY_OUTPUT_DIR", "Proxy config output dir (env)"),
        ("--dry-run", "Simulate actions without changes"),
    ];

    let name_width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    let desc_width = rows.iter().map(|(_, desc)| desc.len()).max().unwrap_or(0);
    let name_width = name_width.max("Parameter/ENV".len());
    let desc_width = desc_width.max("Description".len());

    let border = format!(
        "+-{}-+-{}-+",
        "-".repeat(name_width),
        "-".repeat(desc_width)
    );
    println!("{}", border);
    println!(
        "| {:width$} | {:desc_width$} |",
        "Parameter/ENV",
        "Description",
        width = name_width,
        desc_width = desc_width
    );
    println!("{}", border);
    for (name, desc) in rows {
        println!(
            "| {:width$} | {:desc_width$} |",
            name,
            desc,
            width = name_width,
            desc_width = desc_width
        );
    }
    println!("{}", border);
    Ok(())
}

fn install_if_missing<F>(
    command_name: &str,
    changes: &mut Vec<String>,
    dry_run: bool,
    installer: F,
) -> Result<(), String>
where
    F: Fn(bool) -> Result<(), String>,
{
    if command_exists(command_name) {
        info(&format!("{} is already installed", command_name));
        return Ok(());
    }

    info(&format!("Installing {}", command_name));
    installer(dry_run)?;
    if dry_run {
        changes.push(format!("Would install {}", command_name));
    } else {
        changes.push(format!("Installed {}", command_name));
    }
    Ok(())
}

fn install_nginx_official(dry_run: bool) -> Result<(), String> {
    let os_id = read_os_id()?;
    match os_id.as_str() {
        "debian" => install_nginx_debian_like("debian", dry_run),
        "ubuntu" => install_nginx_debian_like("ubuntu", dry_run),
        "alpine" => install_nginx_alpine(dry_run),
        _ => Err(format!("Unsupported OS for nginx install: {}", os_id)),
    }
}

fn install_nginx_debian_like(os_id: &str, dry_run: bool) -> Result<(), String> {
    let keyring_pkg = if os_id == "ubuntu" {
        "ubuntu-keyring"
    } else {
        "debian-archive-keyring"
    };
    run_cmd(
        "apt",
        &[
            "install",
            "-y",
            "curl",
            "gnupg2",
            "ca-certificates",
            "lsb-release",
            keyring_pkg,
        ],
        dry_run,
    )?;

    run_cmd(
        "curl",
        &[
            "-o",
            "/tmp/nginx_signing.key",
            "https://nginx.org/keys/nginx_signing.key",
        ],
        dry_run,
    )?;
    run_cmd(
        "gpg",
        &[
            "--dearmor",
            "-o",
            "/usr/share/keyrings/nginx-archive-keyring.gpg",
            "/tmp/nginx_signing.key",
        ],
        dry_run,
    )?;

    let codename = read_os_codename()?;
    let repo_line = format!(
        "deb [signed-by=/usr/share/keyrings/nginx-archive-keyring.gpg] https://nginx.org/packages/mainline/{os_id} {codename} nginx\n"
    );
    if dry_run {
        info("[dry-run] Would write /etc/apt/sources.list.d/nginx.list");
        info("[dry-run] Would write /etc/apt/preferences.d/99nginx");
    } else {
        fs::write("/etc/apt/sources.list.d/nginx.list", repo_line)
            .map_err(|e| format!("Failed to write nginx.list: {e}"))?;
        let pin = "Package: *\nPin: origin nginx.org\nPin: release o=nginx\nPin-Priority: 900\n";
        fs::write("/etc/apt/preferences.d/99nginx", pin)
            .map_err(|e| format!("Failed to write 99nginx: {e}"))?;
    }

    run_cmd("apt", &["update"], dry_run)?;
    run_cmd("apt", &["install", "-y", "nginx"], dry_run)?;
    Ok(())
}

fn install_nginx_alpine(dry_run: bool) -> Result<(), String> {
    run_cmd(
        "apk",
        &["add", "openssl", "curl", "ca-certificates"],
        dry_run,
    )?;

    let release = fs::read_to_string("/etc/alpine-release")
        .map_err(|e| format!("Failed to read /etc/alpine-release: {e}"))?;
    let version = release
        .trim()
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let repo_line = format!(
        "@nginx https://nginx.org/packages/mainline/alpine/v{}/main\n",
        version
    );

    if dry_run {
        info("[dry-run] Would append nginx repo to /etc/apk/repositories");
    } else {
        let repos_path = "/etc/apk/repositories";
        let mut repos = fs::read_to_string(repos_path)
            .map_err(|e| format!("Failed to read {}: {e}", repos_path))?;
        if !repos.contains(&repo_line) {
            if !repos.ends_with('\n') {
                repos.push('\n');
            }
            repos.push_str(&repo_line);
            fs::write(repos_path, repos)
                .map_err(|e| format!("Failed to write {}: {e}", repos_path))?;
        }
    }

    run_cmd(
        "curl",
        &[
            "-o",
            "/tmp/nginx_signing.rsa.pub",
            "https://nginx.org/keys/nginx_signing.rsa.pub",
        ],
        dry_run,
    )?;
    if dry_run {
        info("[dry-run] Would move nginx signing key to /etc/apk/keys/");
    } else {
        fs::rename(
            "/tmp/nginx_signing.rsa.pub",
            "/etc/apk/keys/nginx_signing.rsa.pub",
        )
        .map_err(|e| format!("Failed to move nginx signing key: {e}"))?;
    }

    run_cmd("apk", &["add", "nginx@nginx"], dry_run)?;
    Ok(())
}

fn setup_acme_renew_cron(acme_bin: &Path, acme_home: &Path, dry_run: bool) -> Result<(), String> {
    if !command_exists("crontab") {
        info("crontab not found, skipping renew cron setup");
        return Ok(());
    }

    step("Setting up acme renew cron");
    let cron_line = format!(
        "0 0 1,16 * * /bin/sh {} --cron --home {} >/dev/null 2>&1",
        acme_bin.display(),
        acme_home.display()
    );

    if dry_run {
        info(&format!("[dry-run] Would ensure cron: {}", cron_line));
        return Ok(());
    }

    let existing = Command::new("crontab")
        .arg("-l")
        .output()
        .map_err(|e| format!("Failed to read crontab: {e}"))?;
    let mut content = String::from_utf8_lossy(&existing.stdout).to_string();
    if content.contains(&cron_line) {
        info("acme renew cron already exists");
        return Ok(());
    }

    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&cron_line);
    content.push('\n');

    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to write crontab: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write crontab: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("Failed to write crontab: {e}"))?;
    if !status.success() {
        return Err("Failed to update crontab".to_string());
    }

    success("acme renew cron added");
    Ok(())
}

const DEFAULT_CONFIRM_TIMEOUT: Duration = Duration::from_secs(10);

fn confirm_with_timeout(prompt: &str, timeout: Duration, dry_run: bool) -> Result<bool, String> {
    if dry_run {
        info(&format!("[dry-run] Would prompt: {}", prompt));
        return Ok(false);
    }

    info(&format!(
        "{} (y/N) [timeout {}s]",
        prompt,
        timeout.as_secs()
    ));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let _ = tx.send(input);
    });

    match rx.recv_timeout(timeout) {
        Ok(input) => {
            let trimmed = input.trim();
            Ok(trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes"))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(false),
        Err(_) => Err("Failed to read input".to_string()),
    }
}

fn command_exists(command_name: &str) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(command_name);
            if candidate.exists() {
                return true;
            }
        }
    }
    false
}

fn run_cmd(cmd: &str, args: &[&str], dry_run: bool) -> Result<(), String> {
    if dry_run {
        info(&format!("[dry-run] Would run: {} {}", cmd, args.join(" ")));
        return Ok(());
    }
    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run {}: {e}", cmd))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command failed: {}", cmd))
    }
}

fn read_os_id() -> Result<String, String> {
    let content = fs::read_to_string("/etc/os-release")
        .map_err(|e| format!("Failed to read /etc/os-release: {e}"))?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            return Ok(value.trim_matches('"').to_string());
        }
    }
    Err("OS ID not found in /etc/os-release".to_string())
}

fn read_os_codename() -> Result<String, String> {
    let content = fs::read_to_string("/etc/os-release")
        .map_err(|e| format!("Failed to read /etc/os-release: {e}"))?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("VERSION_CODENAME=") {
            return Ok(value.trim_matches('"').to_string());
        }
    }
    let output = Command::new("lsb_release")
        .arg("-cs")
        .output()
        .map_err(|e| format!("Failed to run lsb_release: {e}"))?;
    if !output.status.success() {
        return Err("Failed to read OS codename".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_root() -> Result<(), String> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| format!("Failed to check uid: {e}"))?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid != "0" {
        return Err("This command must be run as root".to_string());
    }
    Ok(())
}

fn print_summary(changes: &[String], elapsed: std::time::Duration) {
    step("Summary");
    if changes.is_empty() {
        info("No changes were made");
    } else {
        for change in changes {
            success(change);
        }
    }
    let seconds = elapsed.as_secs();
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    info(&format!("Execution time: {}m {}s", minutes, remainder));
}

fn resolve_cert_paths(
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
    cert_dir: Option<PathBuf>,
    domain: Option<String>,
) -> Result<(PathBuf, PathBuf), String> {
    match (cert_path, key_path) {
        (Some(cert_path), Some(key_path)) => Ok((cert_path, key_path)),
        (None, None) => {
            let cert_dir =
                cert_dir.ok_or("cert_dir is required to derive cert paths".to_string())?;
            let domain = domain.ok_or("domain is required to derive cert paths".to_string())?;
            Ok((
                cert_dir.join(format!("{}.cer", domain)),
                cert_dir.join(format!("{}.key", domain)),
            ))
        }
        _ => Err("Both cert and key paths must be set together".to_string()),
    }
}
