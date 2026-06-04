//! Claude Haha - Rust-based tool executor with sandboxing, VCR, and verification

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tool_executor::{ToolExecutor, ToolExecutorConfig};
use vcr::{Vcr, VcrConfig, VcrMode};

#[derive(Parser)]
#[command(name = "claude-haha-rust")]
#[command(about = "Rust tool executor with sandboxing and VCR")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a command with sandbox limits
    Exec {
        #[arg()]
        program: String,

        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        #[arg(long, short = 'd', default_value = ".")]
        cwd: PathBuf,

        #[arg(long, short = 't')]
        timeout_secs: Option<u64>,
    },

    /// Start a simple REPL for command execution
    Repl {
        #[arg(long, short = 'd', default_value = ".")]
        cwd: PathBuf,
    },

    /// Check version and configuration
    Info,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Exec { program, args, cwd, timeout_secs } => {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut config = ToolExecutorConfig::default();
                    config.working_dir = cwd;
                    if let Some(t) = timeout_secs {
                        config.time_limit = Some(std::time::Duration::from_secs(t));
                    }

                    let executor = ToolExecutor::new(config);
                    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    executor.execute(&program, &args_ref, None).await
                });

            if result.success {
                print!("{}", result.output.stdout);
                eprint!("{}", result.output.stderr);
            } else {
                if let Some(err) = &result.error {
                    eprintln!("Error: {}", err);
                }
                eprint!("{}", result.output.stderr);
                std::process::exit(1);
            }
        }

        Commands::Repl { cwd } => {
            println!("claude-haha-rust REPL (type 'exit' or 'quit' to leave)");
            println!("Working directory: {}", cwd.display());
            println!();

            let mut config = ToolExecutorConfig::default();
            config.working_dir = cwd;
            config.time_limit = Some(std::time::Duration::from_secs(30));

            let executor = ToolExecutor::new(config);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let mut line = String::new();
            loop {
                print!("> ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                line.clear();
                match std::io::stdin().read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Read error: {}", e);
                        break;
                    }
                }

                let line = line.trim();
                if line.is_empty() || line == "exit" || line == "quit" {
                    break;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                let program = parts[0];
                let args: Vec<&str> = parts[1..].to_vec();

                let result = rt.block_on(async {
                    executor.execute(program, &args, None).await
                });

                if result.success {
                    print!("{}", result.output.stdout);
                    eprint!("{}", result.output.stderr);
                } else {
                    if let Some(err) = &result.error {
                        eprintln!("Error: {}", err);
                    }
                    eprint!("{}", result.output.stderr);
                }

                println!("[exit code: {:?}, duration: {}ms]", result.output.exit_code, result.duration_ms);
            }

            println!("\nBye!");
        }

        Commands::Info => {
            println!("claude-haha-rust v{}", env!("CARGO_PKG_VERSION"));
            println!("Rust tool executor with sandboxing, VCR, and verification");
            println!();

            let sandbox_config = sandbox::SandboxConfig::default();
            println!("Sandbox seccomp: {:?}", sandbox_config.seccomp);
            println!("Sandbox max memory: {:?}", sandbox_config.resources.max_memory_bytes);
            println!("Sandbox max processes: {:?}", sandbox_config.resources.max_processes);

            let vcr_config = VcrConfig::default();
            println!("VCR fixtures dir: {}", vcr_config.fixtures_dir.display());
            println!("VCR mmap: {}", vcr_config.use_mmap);

            // Verify we can create a VCR instance
            match Vcr::new(vcr_config, VcrMode::Playback) {
                Ok(vcr) => println!("VCR initialized, mode: {:?}", vcr.mode()),
                Err(e) => eprintln!("VCR init failed: {}", e),
            }
        }
    }
}
