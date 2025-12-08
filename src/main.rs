use borg_timemachine::{BorgBackup, Config};
use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "borg-timemachine")]
#[command(about = "Time Machine-style backups using Borg", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Borg repository
    Init,

    /// Run a backup cycle (create, prune, compact)
    Backup,

    /// List all archives in the repository
    List,

    /// Mount the repository for browsing
    Mount {
        /// Mount point directory
        #[arg(value_name = "MOUNT_POINT")]
        mount_point: String,
    },

    /// Generate an example configuration file
    GenerateConfig {
        /// Output path for the example config
        #[arg(value_name = "OUTPUT", default_value = "borg-config.yaml")]
        output: String,
    },

    /// Check repository integrity
    Check,

    /// Show repository info
    Info,
}

fn main() {
    let cli = Cli::parse();

    // Handle generate-config separately since it doesn't need a config file
    if let Commands::GenerateConfig { output } = cli.command {
        if let Err(e) = BorgBackup::generate_example_config(&output) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        return;
    }

    // Load configuration
    let config = match Config::load_or_default(cli.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            eprintln!("\nGenerate an example config with:");
            eprintln!("  borg-timemachine generate-config");
            process::exit(1);
        }
    };

    // Load passphrase from file
    let passphrase = match std::fs::read_to_string(&config.security.passphrase_file) {
        Ok(p) => p.trim().to_string(),
        Err(e) => {
            eprintln!(
                "Error reading passphrase file {}: {}",
                config.security.passphrase_file, e
            );
            eprintln!("\nCreate the passphrase file with:");
            eprintln!(
                "  echo 'your-strong-passphrase' > {}",
                config.security.passphrase_file
            );
            eprintln!("  chmod 600 {}", config.security.passphrase_file);
            process::exit(1);
        }
    };

    // Set passphrase environment variable for borg
    std::env::set_var("BORG_PASSPHRASE", passphrase);

    // Create BorgBackup instance
    let mut backup = match BorgBackup::new(config) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Execute command
    let result = match cli.command {
        Commands::Init => backup.init_repository(),
        Commands::Backup => backup.run_backup_cycle(),
        Commands::List => backup.list_archives(),
        Commands::Mount { mount_point } => backup.mount_repository(&mount_point),
        Commands::Check => backup.check_repository(),
        Commands::Info => {
            use std::process::Command;
            let repo_path = backup.get_repo_path();
            Command::new("borg")
                .args(["info", &repo_path])
                .status()
                .map_err(|e| format!("Failed to run borg info: {}", e))
                .and_then(|status| {
                    if !status.success() {
                        Err("borg info failed".to_string())
                    } else {
                        Ok(())
                    }
                })
        }
        Commands::GenerateConfig { .. } => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
