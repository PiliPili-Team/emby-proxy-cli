mod modules;

use clap::Parser;
use modules::cli::{Cli, Commands, IssueCertArgs, WriteProxyArgs};
use modules::commands::{issue_cert, print_params_table, write_nginx_default, write_proxy_config};

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let env_overrides = modules::env::to_env_map(&cli.env_overrides);

    match cli.command {
        Commands::IssueCert {
            cf_token,
            cf_account_id,
            cf_zone_id,
            domain,
            wildcard_domain,
            acme_bin,
            acme_home,
            cert_dir,
            cert_dir_name,
            cert_output_path,
            key_output_path,
            nginx_bin,
            reload_nginx,
            dry_run,
        } => issue_cert(
            &env_overrides,
            IssueCertArgs {
                cf_token,
                cf_account_id,
                cf_zone_id,
                domain,
                wildcard_domain,
                acme_bin,
                acme_home,
                cert_dir,
                cert_dir_name,
                cert_output_path,
                key_output_path,
                nginx_bin,
            },
            reload_nginx,
            dry_run,
        ),
        Commands::WriteNginxDefault {
            cert_path,
            key_path,
            cert_dir_name,
            domain,
            output_path,
            dry_run,
        } => write_nginx_default(
            &env_overrides,
            cert_path,
            key_path,
            cert_dir_name,
            domain,
            output_path,
            dry_run,
        ),
        Commands::WriteProxyConfig {
            proxy_domain,
            backend_url,
            cert_path,
            key_path,
            cert_dir_name,
            cert_dir,
            output_dir,
            resolver,
            dry_run,
        } => write_proxy_config(
            &env_overrides,
            WriteProxyArgs {
                proxy_domain,
                backend_url,
                cert_path,
                key_path,
                cert_dir_name,
                cert_dir,
                output_dir,
                resolvers: resolver,
            },
            dry_run,
        ),
        Commands::PrintParams => print_params_table(),
    }
}
