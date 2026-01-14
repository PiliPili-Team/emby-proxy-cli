use std::{
    collections::HashMap,
    env,
    io::{self, Write},
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

const RESOLVER_TIMEOUT_SECS: u64 = 10;
const RESOLVER_CLOUDFLARE: &str = "1.1.1.1 1.0.0.1 [2606:4700:4700::1111] [2606:4700:4700::1064]";
const RESOLVER_TENCENT: &str = "119.29.29.29 182.254.116.116";
const RESOLVER_ALI: &str = "223.5.5.5 223.6.6.6";
const RESOLVER_GOOGLE: &str = "8.8.8.8 8.8.4.4";

pub fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let mut split = s.splitn(2, '=');
    let key = split.next().unwrap_or("").trim();
    let value = split.next().unwrap_or("").to_string();
    if key.is_empty() {
        return Err("--env expects KEY=VALUE".to_string());
    }
    Ok((key.to_string(), value))
}

pub fn to_env_map(pairs: &[(String, String)]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.clone(), v.clone());
    }
    map
}

pub fn resolve_value(
    cli_value: Option<String>,
    env_overrides: &HashMap<String, String>,
    env_key: &str,
    prompt_label: &str,
    sensitive: bool,
) -> Result<String, String> {
    if let Some(value) = cli_value {
        return Ok(value);
    }
    if let Some(value) = env_overrides.get(env_key)
        && !value.trim().is_empty()
    {
        return Ok(value.clone());
    }
    if let Ok(value) = env::var(env_key)
        && !value.trim().is_empty()
    {
        return Ok(value);
    }

    prompt_value(prompt_label, sensitive)
}

pub fn resolve_optional_value(
    cli_value: Option<String>,
    env_overrides: &HashMap<String, String>,
    env_key: &str,
    prompt_label: &str,
    sensitive: bool,
) -> Result<Option<String>, String> {
    if let Some(value) = cli_value {
        return Ok(Some(value));
    }
    if let Some(value) = env_overrides.get(env_key)
        && !value.trim().is_empty()
    {
        return Ok(Some(value.clone()));
    }
    if let Ok(value) = env::var(env_key)
        && !value.trim().is_empty()
    {
        return Ok(Some(value));
    }

    let input = prompt_value(prompt_label, sensitive)?;
    if input.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

pub fn resolve_path(
    cli_value: Option<PathBuf>,
    env_overrides: &HashMap<String, String>,
    env_key: &str,
    default: &str,
    prompt_label: &str,
) -> Result<PathBuf, String> {
    if let Some(value) = cli_value {
        return Ok(value);
    }
    if let Some(value) = env_overrides.get(env_key)
        && !value.trim().is_empty()
    {
        return Ok(PathBuf::from(value));
    }
    if let Ok(value) = env::var(env_key)
        && !value.trim().is_empty()
    {
        return Ok(PathBuf::from(value));
    }

    let prompt = format!("{} [{}]", prompt_label, default);
    let input = prompt_value(&prompt, false)?;
    if input.trim().is_empty() {
        Ok(PathBuf::from(default))
    } else {
        Ok(PathBuf::from(input))
    }
}

pub fn resolve_optional_path(
    cli_value: Option<PathBuf>,
    env_overrides: &HashMap<String, String>,
    env_key: &str,
) -> Option<PathBuf> {
    if let Some(value) = cli_value {
        return Some(value);
    }
    if let Some(value) = env_overrides.get(env_key)
        && !value.trim().is_empty()
    {
        return Some(PathBuf::from(value));
    }
    if let Ok(value) = env::var(env_key)
        && !value.trim().is_empty()
    {
        return Some(PathBuf::from(value));
    }
    None
}

pub fn resolve_cert_dir(
    cert_dir: Option<PathBuf>,
    cert_dir_name: Option<String>,
    env_overrides: &HashMap<String, String>,
    env_keys: &[&str],
    default_name: &str,
) -> Result<PathBuf, String> {
    if let Some(dir) = cert_dir {
        return Ok(dir);
    }
    let name = resolve_name_with_default(
        cert_dir_name,
        env_overrides,
        env_keys,
        default_name,
        "certificate directory name",
    )?;
    Ok(PathBuf::from("/etc/ca-certificates").join(name))
}

pub fn resolve_name_with_default(
    cli_value: Option<String>,
    env_overrides: &HashMap<String, String>,
    env_keys: &[&str],
    default: &str,
    prompt_label: &str,
) -> Result<String, String> {
    if let Some(value) = cli_value {
        return Ok(value);
    }
    if let Some(value) = resolve_from_envs(env_overrides, env_keys) {
        return Ok(value);
    }
    let prompt = format!("{} [{}]", prompt_label, default);
    let input = prompt_value(&prompt, false)?;
    if input.trim().is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input)
    }
}

pub fn resolve_from_envs(
    env_overrides: &HashMap<String, String>,
    env_keys: &[&str],
) -> Option<String> {
    for key in env_keys {
        if let Some(value) = env_overrides.get(*key)
            && !value.trim().is_empty()
        {
            return Some(value.clone());
        }
        if let Ok(value) = env::var(key)
            && !value.trim().is_empty()
        {
            return Some(value);
        }
    }
    None
}

pub fn resolve_resolvers(
    cli_values: &[String],
    env_overrides: &HashMap<String, String>,
    env_key: &str,
    default_value: &str,
) -> Result<String, String> {
    if !cli_values.is_empty() {
        return Ok(cli_values.join(" "));
    }
    if let Some(value) = env_overrides.get(env_key)
        && !value.trim().is_empty()
    {
        return Ok(value.clone());
    }
    if let Ok(value) = env::var(env_key)
        && !value.trim().is_empty()
    {
        return Ok(value);
    }

    select_resolver_with_timeout(default_value)
}

fn select_resolver_with_timeout(default_value: &str) -> Result<String, String> {
    println!("Select DNS resolver (default: Cloudflare):");
    println!("  1) Cloudflare");
    println!("  2) Tencent");
    println!("  3) Aliyun");
    println!("  4) Google");
    println!("  5) Custom");
    println!("Enter choice [1-5] within {}s: ", RESOLVER_TIMEOUT_SECS);

    let input = read_line_with_timeout(Duration::from_secs(RESOLVER_TIMEOUT_SECS))?;
    let choice = input.unwrap_or_default();
    let trimmed = choice.trim();
    if trimmed.is_empty() {
        return Ok(default_value.to_string());
    }

    match trimmed {
        "1" => Ok(RESOLVER_CLOUDFLARE.to_string()),
        "2" => Ok(RESOLVER_TENCENT.to_string()),
        "3" => Ok(RESOLVER_ALI.to_string()),
        "4" => Ok(RESOLVER_GOOGLE.to_string()),
        "5" => {
            let custom = prompt_value("Custom resolver (space-separated)", false)?;
            if custom.trim().is_empty() {
                Ok(default_value.to_string())
            } else {
                Ok(custom)
            }
        }
        _ => Ok(default_value.to_string()),
    }
}

fn read_line_with_timeout(timeout: Duration) -> Result<Option<String>, String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        let _ = tx.send(input);
    });

    match rx.recv_timeout(timeout) {
        Ok(input) => Ok(Some(input)),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(_) => Err("Failed to read input".to_string()),
    }
}

fn prompt_value(label: &str, sensitive: bool) -> Result<String, String> {
    if sensitive {
        let prompt = format!("{}: ", label);
        rpassword::prompt_password(prompt).map_err(|e| format!("Prompt failed: {e}"))
    } else {
        let mut stdout = io::stdout();
        write!(stdout, "{}: ", label).map_err(|e| format!("Prompt failed: {e}"))?;
        stdout.flush().map_err(|e| format!("Prompt failed: {e}"))?;
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Prompt failed: {e}"))?;
        Ok(input.trim().to_string())
    }
}
