use crate::modules::{
    cli::{IssueCertArgs, WriteProxyArgs},
    env::{
        resolve_cert_dir, resolve_optional_path, resolve_optional_value, resolve_path,
        resolve_resolvers, resolve_value,
    },
    log::info,
    templates::{NGINX_DEFAULT_TEMPLATE, NGINX_PROXY_TEMPLATE},
};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

const DEFAULT_RESOLVER: &str = "1.1.1.1 1.0.0.1 [2606:4700:4700::1111] [2606:4700:4700::1064]";

pub fn issue_cert(
    env_overrides: &HashMap<String, String>,
    args: IssueCertArgs,
    reload_nginx: bool,
    dry_run: bool,
) -> Result<(), String> {
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
        }
    }

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
    let cert_path = cert_path.unwrap_or_else(|| {
        cert_dir
            .as_ref()
            .expect("cert_dir missing")
            .join(format!("{}.cer", domain.as_ref().expect("domain missing")))
    });
    let key_path = key_path.unwrap_or_else(|| {
        cert_dir
            .as_ref()
            .expect("cert_dir missing")
            .join(format!("{}.key", domain.as_ref().expect("domain missing")))
    });
    let output_path = resolve_path(
        output_path,
        env_overrides,
        "NGINX_DEFAULT_OUTPUT",
        "/etc/nginx/conf.d/default/00-default.conf",
        "nginx default output path",
    )?;

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
    }
    Ok(())
}

pub fn write_proxy_config(
    env_overrides: &HashMap<String, String>,
    args: WriteProxyArgs,
    dry_run: bool,
) -> Result<(), String> {
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
    let cert_path = cert_path.unwrap_or_else(|| {
        cert_dir
            .as_ref()
            .expect("cert_dir missing")
            .join(format!("{}.cer", domain.as_ref().expect("domain missing")))
    });
    let key_path = key_path.unwrap_or_else(|| {
        cert_dir
            .as_ref()
            .expect("cert_dir missing")
            .join(format!("{}.key", domain.as_ref().expect("domain missing")))
    });

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
    Ok(())
}

pub fn print_params_table() -> Result<(), String> {
    let rows = vec![
        (
            "--env KEY=VALUE",
            "Override environment values (repeatable)",
        ),
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
