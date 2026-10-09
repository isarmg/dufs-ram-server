use anyhow::{Context, Result, anyhow};
use log::{error, info, warn};
use std::{
    io::{self, Write},
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use xcss_server_runtime::{
    BoundListeners, HttpServer, ProcessSignals, ProductDescriptor, ServerRuntime, health_check,
};
use xczs::args::{Args, build_cli};
use xczs::auth::hash_password;
use xczs::{logger, server::Server};

fn main() -> std::process::ExitCode {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    let json = raw_args.iter().any(|argument| argument == "--json");
    let result = if raw_args.len() == 2 && raw_args[1] == "web-assets" {
        xczs::server::web_assets_manifest().map(|manifest| print!("{manifest}"))
    } else {
        run(raw_args)
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let envelope = if let Some(error) = error.downcast_ref::<xcss_config::ConfigError>() {
                error.envelope()
            } else if let Some(error) = error.downcast_ref::<xcss_server_cli::CliError>() {
                error.0.clone()
            } else if let Some(error) = error.downcast_ref::<xcss_state_file::Error>() {
                xcss_server_cli::state_error(error)
            } else if let Some(error) = error.downcast_ref::<xcss_server_cli::SnapshotError>() {
                xcss_server_cli::snapshot_error(error)
            } else {
                if !json {
                    eprintln!("{error:#}");
                }
                xcss_server_cli::ErrorEnvelope::with_code(
                    xcss_server_cli::ErrorCode::new("current_state_invalid").unwrap(),
                    "The command could not validate or operate on the current configuration and data.",
                )
            };
            let exit = if envelope.code.as_str() == "invalid_cli_input" {
                2
            } else {
                1
            };
            xcss_server_cli::report_error(&envelope, json, exit)
        }
    }
}

#[tokio::main]
async fn run(raw_args: Vec<std::ffi::OsString>) -> Result<()> {
    let cmd = build_cli();
    let matches = match cmd.try_get_matches_from(raw_args) {
        Ok(matches) => matches,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            error.print()?;
            return Ok(());
        }
        Err(_) => {
            return Err(
                xcss_server_cli::CliError(xcss_server_cli::ErrorEnvelope::with_code(
                    xcss_server_cli::ErrorCode::new("invalid_cli_input").unwrap(),
                    "Command arguments do not satisfy the current CLI contract; use --help.",
                ))
                .into(),
            );
        }
    };
    if matches.subcommand_matches("hash-password").is_some() {
        let password = rpassword::prompt_password("Password: ")?;
        validate_cli_password(&password)?;
        let confirmation = rpassword::prompt_password("Confirm password: ")?;
        if password != confirmation {
            anyhow::bail!("Passwords do not match");
        }
        println!("{}", hash_password(&password)?);
        return Ok(());
    }
    let mode = matches.subcommand_name().unwrap_or("run").to_owned();
    let json = matches.get_flag("json");
    let config_path = matches
        .get_one::<std::path::PathBuf>("config")
        .map(|path| xczs::args::normalize_config_path(path))
        .transpose()?;
    let args = Args::parse(matches)?;
    match mode.as_str() {
        "init" => {
            Server::initialize(args)?;
            xcss_log::LogRecord::common("xczs", xcss_log::CommonEvent::InitializationCompleted)?
                .emit_stderr()?;
            println!(
                "{}",
                serde_json::json!({"status":"initialized", "ready":false})
            );
            return Ok(());
        }
        "config" => {
            let schema = Server::validate_current(&args)?;
            let mut paths = vec![
                args.state_dir.as_ref().unwrap().clone(),
                args.serve_path.clone(),
            ];
            if let Some(path) = config_path {
                paths.push(path.canonicalize()?);
            }
            println!(
                "{}",
                serde_json::json!({"status":"valid", "schema_identity":schema, "sources":args.config_sources, "state_paths":paths})
            );
            return Ok(());
        }
        "status" => {
            let report =
                xcss_server_cli::query_status(SocketAddr::new(args.addrs[0], args.port), "xczs")
                    .await
                    .map_err(xcss_server_cli::CliError)?;
            if !report.ready {
                return Err(
                    xcss_server_cli::CliError(xcss_server_cli::ErrorEnvelope::with_code(
                        xcss_server_cli::ErrorCode::new("service_not_ready").unwrap(),
                        "The service answered but its business readiness checks failed.",
                    ))
                    .into(),
                );
            }
            xcss_server_cli::print_report(&report, json)?;
            return Ok(());
        }
        _ => {}
    }
    let result = run_server(args, json).await;
    if let Err(error) = &result {
        error!("Server failed: {error:#}");
        log::logger().flush();
    }
    result
}

async fn run_server(args: Args, json: bool) -> Result<()> {
    let data_dir = args.state_dir.as_ref().context("data_dir is required")?;
    xcss_server_cli::runtime_allowed(data_dir).map_err(xcss_server_cli::CliError)?;
    Server::validate_current(&args)?;
    let directory = xcss_state_file::PrivateStateDirectory::open(data_dir)?;
    let _common_lock = directory.try_instance_lock()?;
    xcss_server_cli::runtime_allowed(data_dir).map_err(xcss_server_cli::CliError)?;
    Server::validate_current(&args)?;
    logger::init(args.log_file.clone(), data_dir)
        .map_err(|e| anyhow!("Failed to init logger, {e}"))?;
    info!(target:"common.config.loaded", "configuration loaded");
    xcss_server_runtime::install_panic_hook();
    let print_addrs = args.addrs.clone();
    let max_connections = args.max_connections;
    let mut signals = ProcessSignals::install()?;
    let listeners = BoundListeners::bind(
        args.addrs
            .iter()
            .map(|address| SocketAddr::new(*address, args.port)),
    )
    .context("Failed to bind all configured listen addresses")?;
    let port = listeners.addresses()[0].port();
    let runtime_handle = tokio::runtime::Handle::current();
    let mut startup = tokio::task::spawn_blocking(move || {
        let _entered = runtime_handle.enter();
        Server::builder(args).build()
    });
    let product = tokio::select! {
        biased;
        signal = signals.recv() => {
            info!("Shutdown requested during startup reason={}", signal?);
            final_exit(0);
        },
        result = &mut startup => Arc::new(result.context("Server startup task failed")??),
    };
    let server = product.server().clone();
    let health_server = server.clone();
    let runtime = ServerRuntime::builder(ProductDescriptor {
        id: "xczs".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        foundation_revision: env!("XCSS_FOUNDATION_REVISION").into(),
        profile: "server-filesystem".into(),
        capabilities: vec![
            "embedded-web".into(),
            "admin-static".into(),
            "memory-sessions".into(),
            "server-runtime".into(),
            "server-health".into(),
            "filesystem-root".into(),
            "linux-openat2".into(),
            "durable-operations".into(),
        ],
    })
    .with_schema_identity(server.schema_identity()?)
    .register_health_check(
        "filesystem",
        health_check(move || {
            let server = health_server.clone();
            async move { server.probe_readiness().await }
        }),
    )
    .register_background_task(
        "shutdown-log",
        xcss_server_runtime::TaskCriticality::Degrading,
        |mut shutdown| async move {
            if !*shutdown.borrow() {
                let _ = shutdown.changed().await;
            }
            info!(target:"common.runtime.shutdown_started", "shutdown started");
            Ok(())
        },
    )
    .build()
    .await?;
    let service = server.http_service(runtime.handle())?;
    let mut transport = HttpServer::new(listeners, signals);
    transport.limits.max_connections = max_connections;
    transport.participant = Some(product);
    info!(target:"common.runtime.started", "runtime started");
    let listening = if json {
        serde_json::json!({"service":"xczs", "state":"listening", "port":port}).to_string() + "\n"
    } else {
        print_listening(&print_addrs, port)
    };
    if let Err(error) = write_listening(std::io::stdout().lock(), &listening) {
        warn!("Failed to write listening address to stdout: {error}");
    }
    match runtime.serve(transport, service).await {
        Ok(report) => {
            info!(target:"common.runtime.stopped", "runtime stopped");
            info!(
                "Graceful shutdown complete clean={} phase={:?}",
                report.clean, report.phase
            );
            final_exit(0);
        }
        Err(error) => {
            error!("Server shutdown failed: {error}");
            // Keep the error's business-state retention owner alive until the
            // executable terminates; never let runtime Drop release stuck I/O.
            final_exit(1);
        }
    }
}

fn final_exit(code: i32) -> ! {
    log::logger().flush();
    std::process::exit(code)
}

fn validate_cli_password(password: &str) -> Result<()> {
    xcss_admin_auth::validate_password(password)
        .context("Password violates the current administrator policy")
}

fn print_listening(print_addrs: &[IpAddr], port: u16) -> String {
    let mut output = String::new();
    let urls = print_addrs
        .iter()
        .map(|addr| {
            let addr = match addr {
                IpAddr::V4(_) => format!("{addr}:{port}"),
                IpAddr::V6(_) => format!("[{addr}]:{port}"),
            };
            format!("http://{addr}/")
        })
        .collect::<Vec<_>>();

    if urls.len() == 1 {
        output.push_str(&format!("Listening on {}", urls[0]))
    } else {
        let info = urls
            .iter()
            .map(|v| format!("  {v}"))
            .collect::<Vec<String>>()
            .join("\n");
        output.push_str(&format!("Listening on:\n{info}\n"))
    }

    output
}

fn write_listening(mut output: impl Write, listening: &str) -> io::Result<()> {
    writeln!(output, "{listening}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cli_password_validation_uses_the_foundation_administrator_policy() {
        assert!(validate_cli_password("").is_err());
        for password in [
            "p".repeat(xcss_admin_auth::PASSWORD_MIN_BYTES),
            "p".repeat(xcss_admin_auth::PASSWORD_MAX_BYTES),
            "é".repeat(xcss_admin_auth::PASSWORD_MAX_BYTES / "é".len()),
        ] {
            assert!(validate_cli_password(&password).is_ok());
        }
        for password in [
            "p".repeat(xcss_admin_auth::PASSWORD_MIN_BYTES - 1),
            "valid-password\n".to_string(),
            "p".repeat(xcss_admin_auth::PASSWORD_MAX_BYTES + 1),
            format!(
                "a{}",
                "é".repeat(xcss_admin_auth::PASSWORD_MAX_BYTES / "é".len())
            ),
        ] {
            assert!(validate_cli_password(&password).is_err());
        }
    }
}
