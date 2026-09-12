use anyhow::{Context, Result, anyhow};
use dufs::args::{Args, build_cli};
use dufs::auth::hash_password;
use dufs::{logger, server::Server};
use log::{error, info, warn};
use sarmg_server_runtime::{
    BoundListeners, HttpServer, ProcessSignals, ProductDescriptor, ServerRuntime, health_check,
};
use std::{
    io::{self, Write},
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

fn main() -> Result<()> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    run(raw_args)
}

#[tokio::main]
async fn run(raw_args: Vec<std::ffi::OsString>) -> Result<()> {
    let cmd = build_cli();
    let matches = cmd.get_matches_from(raw_args);
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
    let args = Args::parse(matches)?;
    logger::init(args.log_file.clone()).map_err(|e| anyhow!("Failed to init logger, {e}"))?;
    let result = run_server(args).await;
    if let Err(error) = &result {
        error!("Server failed: {error:#}");
        log::logger().flush();
    }
    result
}

async fn run_server(args: Args) -> Result<()> {
    sarmg_server_runtime::install_panic_hook();
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
        id: "dufs-ram".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        foundation_revision: "80c49f2f811d8d47dbbcdb6c9521924de7d3184e".into(),
        profile: "server-filesystem".into(),
        capabilities: vec![
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
    .build()
    .await?;
    let service = server.http_service(runtime.handle())?;
    let mut transport = HttpServer::new(listeners, signals);
    transport.limits.max_connections = max_connections;
    transport.participant = Some(product);
    let listening = print_listening(&print_addrs, port);
    if let Err(error) = write_listening(std::io::stdout().lock(), &listening) {
        warn!("Failed to write listening address to stdout: {error}");
    }
    match runtime.serve(transport, service).await {
        Ok(report) => {
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
    sarmg_admin_auth::validate_password(password)
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
            "p".repeat(sarmg_admin_auth::PASSWORD_MIN_BYTES),
            "p".repeat(sarmg_admin_auth::PASSWORD_MAX_BYTES),
            "é".repeat(sarmg_admin_auth::PASSWORD_MAX_BYTES / "é".len()),
        ] {
            assert!(validate_cli_password(&password).is_ok());
        }
        for password in [
            "p".repeat(sarmg_admin_auth::PASSWORD_MIN_BYTES - 1),
            "valid-password\n".to_string(),
            "p".repeat(sarmg_admin_auth::PASSWORD_MAX_BYTES + 1),
            format!(
                "a{}",
                "é".repeat(sarmg_admin_auth::PASSWORD_MAX_BYTES / "é".len())
            ),
        ] {
            assert!(validate_cli_password(&password).is_err());
        }
    }
}
