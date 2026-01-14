# Flow Diagram

```mermaid
flowchart TD
    A[Start] --> B[Choose command]
    B --> S{setup?}
    B --> C{issue-cert?}
    B --> D{write-nginx-default?}
    B --> E{write-proxy-config?}

    S --> S1[Check root]
    S1 --> S2[Prompt zsh install (timeout => skip)]
    S2 --> S3[Install cron if missing]
    S3 --> S4[Install nginx if missing]
    S4 --> C9

    C --> C0[Check root]
    C --> C1[Resolve inputs from args/env]
    C1 --> C2{Missing values?}
    C2 -->|Yes| C3[Interactive prompts]
    C2 -->|No| C4[Continue]
    C3 --> C4
    C4 --> C5[Issue cert via acme.sh]
    C5 --> C6[Write cert/key outputs]
    C6 --> C6a[Ensure renew cron (1st & 16th monthly)]
    C6 --> C7{Reload nginx?}
    C7 -->|Yes| C8[nginx -t && reload]
    C7 -->|No| C9[Done]
    C8 --> C9

    D --> D1[Resolve cert/key paths]
    D1 --> D2[Fill default template]
    D2 --> D3[Write config file]
    D3 --> C9

    E --> E1[Resolve domain/backend]
    E1 --> E2[Resolve resolver list]
    E2 --> E3[Resolve cert/key paths]
    E3 --> E4[Fill proxy template]
    E4 --> E5[Write proxy config]
    E5 --> C9
```
