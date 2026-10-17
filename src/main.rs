mod api;
mod app;
mod config;
mod error;
mod security;
mod update;
mod widgets;

use std::process::ExitCode;

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use rand::RngCore;

use crate::config::{Config, Launch};

fn main() -> ExitCode {
    let launch = match Config::launch() {
        Ok(launch) => launch,
        Err(error) => {
            eprintln!("homepage: {error}");
            return ExitCode::FAILURE;
        }
    };
    match launch {
        Launch::HashPassword => hash_password(),
        Launch::Update { command, state_dir } => {
            let result = tokio::runtime::Runtime::new()
                .map_err(crate::error::AppError::from)
                .and_then(|runtime| runtime.block_on(update::run_command(command, &state_dir)));
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("homepage: update failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Launch::CheckConfig(config) => {
            init_tracing(&config.log_level);
            match app::build_auth(&config) {
                Ok(_) => {
                    println!(
                        "configuration OK: {} services, {} devices",
                        config.services.len(),
                        config.devices.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("homepage: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Launch::Serve(config) => {
            init_tracing(&config.log_level);
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("homepage: cannot start runtime: {error}");
                    return ExitCode::FAILURE;
                }
            };
            match runtime.block_on(app::run(*config)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    tracing::error!(%error, "server stopped");
                    eprintln!("homepage: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn init_tracing(level: &str) {
    let filter =
        tracing_subscriber::EnvFilter::try_new(format!("homepage={level},tower_http=warn"))
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn hash_password() -> ExitCode {
    let read = || -> std::io::Result<String> {
        use std::io::{BufRead, IsTerminal};
        if !std::io::stdin().is_terminal() {
            // Scripted use: `echo "$PASSWORD" | homepage hash-password`
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            let password = line.trim_end_matches(['\r', '\n']).to_owned();
            if password.chars().count() < 12 {
                return Err(std::io::Error::other("use at least 12 characters"));
            }
            return Ok(password);
        }
        let first = rpassword::prompt_password("New password: ")?;
        let second = rpassword::prompt_password("Repeat password: ")?;
        if first != second {
            return Err(std::io::Error::other("passwords do not match"));
        }
        if first.chars().count() < 12 {
            return Err(std::io::Error::other("use at least 12 characters"));
        }
        Ok(first)
    };
    let password = match read() {
        Ok(password) => password,
        Err(error) => {
            eprintln!("homepage: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut salt = [0_u8; 16];
    rand::rng().fill_bytes(&mut salt);
    let hash = SaltString::encode_b64(&salt)
        .map_err(|error| error.to_string())
        .and_then(|salt| {
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|error| error.to_string())
        });
    match hash {
        Ok(hash) => {
            println!("password_hash = '{hash}'");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("homepage: {error}");
            ExitCode::FAILURE
        }
    }
}
