# Emby Proxy CLI

<p align="center">
  <a href="https://github.com/PiliPili-Team/emby-proxy-cli">
    <img src="https://img.shields.io/github/languages/top/PiliPili-Team/emby-proxy-cli" alt="Top Language" />
  </a>
  <a href="https://github.com/PiliPili-Team/emby-proxy-cli/actions/workflows/ci.yml">
    <img src="https://github.com/PiliPili-Team/emby-proxy-cli/actions/workflows/ci.yml/badge.svg" alt="CI Status" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
  </a>
</p>

Emby Proxy CLI is a Rust-based tool for issuing certificates with acme.sh and generating Nginx reverse proxy configs without hardcoded secrets.

## Features

- Issue certificates with Cloudflare DNS API inputs from args, env, or interactive prompts.
- Generate Nginx default and reverse proxy configs from embedded templates.
- Dry-run mode for safe simulation.
- Resolver selection via args/env or interactive menu with default Cloudflare.

## Requirements

- Rust toolchain (stable)
- acme.sh available on the target server (for `issue-cert`)
- Nginx available if you enable reload

## Build

```bash
cargo build
```

## Quick Start

Show all parameters:

```bash
emby-proxy-cli print-params
```

Issue certificate (interactive if missing):

```bash
emby-proxy-cli issue-cert \
  --domain example.com \
  --wildcard-domain '*.example.com'
```

Replace cert files only:

```bash
emby-proxy-cli issue-cert \
  --cert-output-path /etc/ca-certificates/custom/example.com.cer \
  --key-output-path /etc/ca-certificates/custom/example.com.key
```

Generate reverse proxy config:

```bash
emby-proxy-cli write-proxy-config \
  --proxy-domain proxy.example.com \
  --backend-url https://emby.example.com:443
```

Simulate without changes:

```bash
emby-proxy-cli write-proxy-config --dry-run
```

## Parameters

### Global

| Parameter/ENV | Description |
| --- | --- |
| `--env KEY=VALUE` | Override env values (repeatable) |

Example:

```bash
emby-proxy-cli --env CF_TOKEN=*** --env DOMAIN=example.com issue-cert
```

### issue-cert

| Parameter/ENV | Description |
| --- | --- |
| `--cf-token` / `CF_TOKEN` | Cloudflare token |
| `--cf-account-id` / `CF_ACCOUNT_ID` | Cloudflare account ID |
| `--cf-zone-id` / `CF_ZONE_ID` | Cloudflare zone ID |
| `--domain` / `DOMAIN` | Primary domain |
| `--wildcard-domain` / `WILDCARD_DOMAIN` | Wildcard domain |
| `--acme-bin` / `ACME_BIN` | acme.sh path |
| `--acme-home` / `ACME_HOME` | acme home directory |
| `--cert-dir` / `CERT_DIR` | Certificate directory (absolute path) |
| `--cert-dir-name` / `CERT_DIR_NAME` | Certificate directory name |
| `--cert-output-path` / `CERT_OUTPUT_PATH` | Certificate output path (pair with key) |
| `--key-output-path` / `KEY_OUTPUT_PATH` | Key output path (pair with cert) |
| `--nginx-bin` / `NGINX_BIN` | nginx binary |
| `--reload-nginx` | Reload nginx after issuance |
| `--dry-run` | Simulate actions without changes |

Example:

```bash
emby-proxy-cli issue-cert --domain example.com --reload-nginx
```

### write-nginx-default

| Parameter/ENV | Description |
| --- | --- |
| `--cert-path` / `NGINX_CERT_PATH` | Nginx cert path |
| `--key-path` / `NGINX_KEY_PATH` | Nginx key path |
| `--cert-dir-name` / `NGINX_CERT_DIR_NAME` | Certificate directory name |
| `--domain` / `DOMAIN` | Primary domain (used for default cert/key) |
| `--output-path` / `NGINX_DEFAULT_OUTPUT` | Output path for default config |
| `--dry-run` | Simulate actions without changes |

Example:

```bash
emby-proxy-cli write-nginx-default --output-path /etc/nginx/conf.d/default/00-default.conf
```

### write-proxy-config

| Parameter/ENV | Description |
| --- | --- |
| `--proxy-domain` / `PROXY_DOMAIN` | Proxy domain |
| `--backend-url` / `BACKEND_URL` | Backend URL |
| `--resolver` / `RESOLVER` | DNS resolver list (repeatable or env) |
| `--cert-path` / `NGINX_CERT_PATH` | Nginx cert path |
| `--key-path` / `NGINX_KEY_PATH` | Nginx key path |
| `--cert-dir` / `CERT_DIR` | Certificate directory (absolute path) |
| `--cert-dir-name` / `CERT_DIR_NAME` | Certificate directory name |
| `--output-dir` / `PROXY_OUTPUT_DIR` | Proxy config output dir |
| `--dry-run` | Simulate actions without changes |

Example:

```bash
emby-proxy-cli write-proxy-config --proxy-domain proxy.example.com --backend-url https://emby.example.com:443
```

## Environment Overrides

You can pass env overrides inline:

```bash
emby-proxy-cli --env CF_TOKEN=*** --env DOMAIN=example.com issue-cert
```

Or create a local `.env` file (see `.env.example`) and export it before running the CLI.

## Docs

- Common operations (EN): `docs/USAGE.md`
- 常用操作（中文）: `docs/USAGE_CN.md`
