# 常用操作

本文档展示 Emby Proxy CLI 的常见使用方式。

## 1. 查看参数

```bash
emby-proxy-cli print-params
```

## 2. 申请证书（交互）

```bash
emby-proxy-cli issue-cert
```

## 3. 仅替换证书文件

```bash
emby-proxy-cli issue-cert \
  --cert-output-path /etc/ca-certificates/custom/example.com.cer \
  --key-output-path /etc/ca-certificates/custom/example.com.key
```

## 4. 生成反代配置

```bash
emby-proxy-cli write-proxy-config \
  --proxy-domain proxy.example.com \
  --backend-url https://emby.example.com:443
```

## 5. 选择 DNS resolver

- 通过参数（可重复）：

```bash
emby-proxy-cli write-proxy-config \
  --resolver "1.1.1.1 1.0.0.1" \
  --resolver "8.8.8.8 8.8.4.4"
```

- 或环境变量：

```bash
export RESOLVER="1.1.1.1 1.0.0.1"
```

- 或进入交互选择（超时默认 Cloudflare）。

## 6. Dry-run 预演

```bash
emby-proxy-cli write-proxy-config --dry-run
```

## 7. 使用 env 覆盖

```bash
emby-proxy-cli --env DOMAIN=example.com issue-cert
```
