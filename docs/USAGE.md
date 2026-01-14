# Common Operations

This document shows typical operations for Emby Proxy CLI.

## 1. Inspect parameters

```bash
emby-proxy-cli print-params
```

## 2. Issue certificate (interactive)

```bash
emby-proxy-cli issue-cert
```

## 3. Issue certificate with explicit outputs

Use this when you only want to replace cert files:

```bash
emby-proxy-cli issue-cert \
  --cert-output-path /etc/ca-certificates/custom/example.com.cer \
  --key-output-path /etc/ca-certificates/custom/example.com.key
```

## 4. Generate reverse proxy config

```bash
emby-proxy-cli write-proxy-config \
  --proxy-domain proxy.example.com \
  --backend-url https://emby.example.com:443
```

## 5. Select DNS resolver

- Use repeatable args:

```bash
emby-proxy-cli write-proxy-config \
  --resolver "1.1.1.1 1.0.0.1" \
  --resolver "8.8.8.8 8.8.4.4"
```

- Or set env:

```bash
export RESOLVER="1.1.1.1 1.0.0.1"
```

- Or enter interactive mode (default Cloudflare after timeout).

## 6. Dry run

```bash
emby-proxy-cli write-proxy-config --dry-run
```

## 7. Use env overrides

```bash
emby-proxy-cli --env DOMAIN=example.com issue-cert
```
