use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        long = "env",
        value_parser = crate::modules::env::parse_key_val,
        help = "Provide environment overrides as KEY=VALUE (repeatable)"
    )]
    pub env_overrides: Vec<(String, String)>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug)]
pub struct IssueCertArgs {
    pub cf_token: Option<String>,
    pub cf_account_id: Option<String>,
    pub cf_zone_id: Option<String>,
    pub domain: Option<String>,
    pub wildcard_domain: Option<String>,
    pub acme_bin: Option<PathBuf>,
    pub acme_home: Option<PathBuf>,
    pub cert_dir: Option<PathBuf>,
    pub cert_dir_name: Option<String>,
    pub cert_input_path: Option<PathBuf>,
    pub key_input_path: Option<PathBuf>,
    pub cert_output_path: Option<PathBuf>,
    pub key_output_path: Option<PathBuf>,
    pub nginx_bin: Option<PathBuf>,
}

#[derive(Debug)]
pub struct WriteProxyArgs {
    pub proxy_domain: Option<String>,
    pub backend_url: Option<String>,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub cert_dir_name: Option<String>,
    pub cert_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub resolvers: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Setup {
        #[arg(long, default_value_t = true)]
        install_zsh: bool,
        #[arg(long, default_value_t = true)]
        install_cron: bool,
        #[arg(long, default_value_t = true)]
        install_nginx: bool,
        #[arg(long)]
        dry_run: bool,
    },
    IssueCert {
        #[arg(long)]
        cf_token: Option<String>,
        #[arg(long)]
        cf_account_id: Option<String>,
        #[arg(long)]
        cf_zone_id: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        wildcard_domain: Option<String>,
        #[arg(long)]
        acme_bin: Option<PathBuf>,
        #[arg(long)]
        acme_home: Option<PathBuf>,
        #[arg(long)]
        cert_dir: Option<PathBuf>,
        #[arg(long)]
        cert_dir_name: Option<String>,
        #[arg(long)]
        cert_input_path: Option<PathBuf>,
        #[arg(long)]
        key_input_path: Option<PathBuf>,
        #[arg(long)]
        cert_output_path: Option<PathBuf>,
        #[arg(long)]
        key_output_path: Option<PathBuf>,
        #[arg(long)]
        nginx_bin: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        reload_nginx: bool,
        #[arg(long)]
        dry_run: bool,
    },
    WriteNginxDefault {
        #[arg(long)]
        cert_path: Option<PathBuf>,
        #[arg(long)]
        key_path: Option<PathBuf>,
        #[arg(long)]
        cert_dir_name: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        output_path: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    WriteProxyConfig {
        #[arg(long)]
        proxy_domain: Option<String>,
        #[arg(long)]
        backend_url: Option<String>,
        #[arg(long)]
        cert_path: Option<PathBuf>,
        #[arg(long)]
        key_path: Option<PathBuf>,
        #[arg(long)]
        cert_dir_name: Option<String>,
        #[arg(long)]
        cert_dir: Option<PathBuf>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long)]
        resolver: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    PrintParams,
}
