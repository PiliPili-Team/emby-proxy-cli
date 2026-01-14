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

Emby Proxy CLI 是一个基于 Rust 的工具，用于通过 acme.sh 申请证书并生成 Nginx 反向代理配置，避免硬编码敏感信息。

## 功能

- 证书申请：参数 / 环境变量 / 交互式输入三选一。
- 生成 Nginx 默认配置与反代配置 [LICENSE](LICENSE) （内置模板）。
- Dry-run 模式安全预演。
- DNS resolver 可通过参数/环境变量或交互选择（默认 Cloudflare）。

## 环境要求

- Rust 工具链（stable）
- 目标服务器已安装 acme.sh（用于 `issue-cert`）
- 如需 reload，需要 Nginx

## 构建

```bash
cargo build
```

## 快速开始

查看所有参数：

```bash
emby-proxy-cli print-params
```

申请证书（缺少参数会进入交互）：

```bash
emby-proxy-cli issue-cert \
  --domain example.com \
  --wildcard-domain '*.example.com'
```

仅替换证书文件：

```bash
emby-proxy-cli issue-cert \
  --cert-output-path /etc/ca-certificates/custom/example.com.cer \
  --key-output-path /etc/ca-certificates/custom/example.com.key
```

生成反代配置：

```bash
emby-proxy-cli write-proxy-config \
  --proxy-domain proxy.example.com \
  --backend-url https://emby.example.com:443
```

Dry-run 预演：

```bash
emby-proxy-cli write-proxy-config --dry-run
```

## 参数

### 全局

| 参数/ENV | 说明 |
| --- | --- |
| `--env KEY=VALUE` | 覆盖环境变量（可重复） |

示例：

```bash
emby-proxy-cli --env CF_TOKEN=*** --env DOMAIN=example.com issue-cert
```

### issue-cert

| 参数/ENV | 说明 |
| --- | --- |
| `--cf-token` / `CF_TOKEN` | Cloudflare token |
| `--cf-account-id` / `CF_ACCOUNT_ID` | Cloudflare account ID |
| `--cf-zone-id` / `CF_ZONE_ID` | Cloudflare zone ID |
| `--domain` / `DOMAIN` | 主域名 |
| `--wildcard-domain` / `WILDCARD_DOMAIN` | 泛域名 |
| `--acme-bin` / `ACME_BIN` | acme.sh 路径 |
| `--acme-home` / `ACME_HOME` | acme home 目录 |
| `--cert-dir` / `CERT_DIR` | 证书目录（绝对路径） |
| `--cert-dir-name` / `CERT_DIR_NAME` | 证书目录名 |
| `--cert-output-path` / `CERT_OUTPUT_PATH` | 证书输出路径（需配对 key） |
| `--key-output-path` / `KEY_OUTPUT_PATH` | key 输出路径（需配对 cert） |
| `--nginx-bin` / `NGINX_BIN` | nginx 路径 |
| `--reload-nginx` | 申请后 reload nginx |
| `--dry-run` | 模拟执行不落地 |

示例：

```bash
emby-proxy-cli issue-cert --domain example.com --reload-nginx
```

仅更换证书：

```bash
emby-proxy-cli issue-cert \
  --cert-output-path /etc/ca-certificates/custom/example.com.cer \
  --key-output-path /etc/ca-certificates/custom/example.com.key
```

### write-nginx-default

| 参数/ENV | 说明 |
| --- | --- |
| `--cert-path` / `NGINX_CERT_PATH` | Nginx 证书路径 |
| `--key-path` / `NGINX_KEY_PATH` | Nginx key 路径 |
| `--cert-dir-name` / `NGINX_CERT_DIR_NAME` | 证书目录名 |
| `--domain` / `DOMAIN` | 主域名（用于默认证书路径） |
| `--output-path` / `NGINX_DEFAULT_OUTPUT` | 默认配置输出路径 |
| `--dry-run` | 模拟执行不落地 |

示例：

```bash
emby-proxy-cli write-nginx-default --output-path /etc/nginx/conf.d/default/00-default.conf
```

### write-proxy-config

| 参数/ENV | 说明 |
| --- | --- |
| `--proxy-domain` / `PROXY_DOMAIN` | 代理域名 |
| `--backend-url` / `BACKEND_URL` | 后端地址 |
| `--resolver` / `RESOLVER` | DNS resolver 列表（可重复或 env） |
| `--cert-path` / `NGINX_CERT_PATH` | Nginx 证书路径 |
| `--key-path` / `NGINX_KEY_PATH` | Nginx key 路径 |
| `--cert-dir` / `CERT_DIR` | 证书目录（绝对路径） |
| `--cert-dir-name` / `CERT_DIR_NAME` | 证书目录名 |
| `--output-dir` / `PROXY_OUTPUT_DIR` | 代理配置输出目录 |
| `--dry-run` | 模拟执行不落地 |

示例：

```bash
emby-proxy-cli write-proxy-config --proxy-domain proxy.example.com --backend-url https://emby.example.com:443
```

## 环境变量覆盖

可以直接传入：

```bash
emby-proxy-cli --env CF_TOKEN=*** --env DOMAIN=example.com issue-cert
```

也可以准备 `.env` (参考 [`.env.example`](./.env.example))，并在运行前导出。

## 文档

- 常用操作: [`USAGE.md`](./docs/USAGE_CN.md)
