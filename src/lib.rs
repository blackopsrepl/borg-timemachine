use chrono::{Datelike, Local};
use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const DEFAULT_CONFIG: &str = include_str!("../borg-config.yaml");

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub repository: Repository,
    pub jobs: Vec<BackupJob>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    pub compression: String,
    pub options: Options,
    pub retention: Retention,
    pub notifications: Notifications,
    pub logging: Logging,
    pub maintenance: Maintenance,
    pub security: Security,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Repository {
    pub path: String,
    pub encryption: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BackupJob {
    pub name: String,
    pub source: String,
    pub destination: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone)]
pub struct Options {
    pub one_file_system: bool,
    pub exclude_caches: bool,
    pub show_progress: bool,
    pub show_stats: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Retention {
    pub within: String,
    pub hourly: u32,
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
    pub yearly: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Notifications {
    pub enabled: bool,
    pub email: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Logging {
    pub log_file: String,
    pub lock_file: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Maintenance {
    pub check_day: u32,
    pub auto_compact: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Security {
    pub passphrase_file: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {}", path, e))?;

        serde_yaml::from_str(&contents).map_err(|e| format!("Failed to parse config file: {}", e))
    }

    pub fn load_or_default(path: Option<&str>) -> Result<Self, String> {
        if let Some(config_path) = path {
            Self::load(config_path)
        } else {
            serde_yaml::from_str(DEFAULT_CONFIG)
                .map_err(|e| format!("Failed to parse default config: {}", e))
        }
    }
}

pub struct BorgBackup {
    config: Config,
    log_handle: Option<fs::File>,
    hostname: String,
}

impl BorgBackup {
    pub fn new(config: Config) -> Result<Self, String> {
        let hostname = Self::get_hostname()?;

        Ok(Self {
            config,
            log_handle: None,
            hostname,
        })
    }

    pub fn get_repo_path(&self) -> String {
        self.config.repository.path.clone()
    }

    fn get_hostname() -> Result<String, String> {
        let output = Command::new("hostname")
            .arg("-s")
            .output()
            .map_err(|e| format!("Failed to get hostname: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn init_repository(&self) -> Result<(), String> {
        println!(
            "Initializing Borg repository at: {}",
            self.config.repository.path
        );

        // Check if repository already exists
        let check = Command::new("borg")
            .args(["info", &self.config.repository.path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if check.is_ok() && check.unwrap().success() {
            return Err(format!(
                "Repository already exists at {}. Remove it first or use a different path.",
                self.config.repository.path
            ));
        }

        let status = Command::new("borg")
            .args([
                "init",
                &format!("--encryption={}", self.config.repository.encryption),
                &self.config.repository.path,
            ])
            .status()
            .map_err(|e| format!("Failed to run borg init: {}", e))?;

        if !status.success() {
            return Err("borg init failed".to_string());
        }

        println!("Repository initialized successfully!");
        println!("\nIMPORTANT: Export and backup your encryption key:");
        println!(
            "  borg key export {} ~/borg-key-backup.txt",
            self.config.repository.path
        );
        println!(
            "  borg key export --paper {} borg-key-qr.html",
            self.config.repository.path
        );

        Ok(())
    }

    pub fn check_lock(&self) -> Result<(), String> {
        if Path::new(&self.config.logging.lock_file).exists() {
            return Err(format!(
                "Lock file exists at {}. Another backup may be running.",
                self.config.logging.lock_file
            ));
        }
        Ok(())
    }

    pub fn create_lock(&self) -> Result<(), String> {
        fs::write(&self.config.logging.lock_file, "")
            .map_err(|e| format!("Failed to create lock file: {}", e))
    }

    pub fn remove_lock(&self) {
        let _ = fs::remove_file(&self.config.logging.lock_file);
    }

    fn log(&mut self, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_line = format!("[{}] {}\n", timestamp, message);

        // Print to stdout
        print!("{}", log_line);
        let _ = io::stdout().flush();

        // Write to log file if handle exists
        if let Some(ref mut handle) = self.log_handle {
            let _ = handle.write_all(log_line.as_bytes());
        }
    }

    pub fn open_log(&mut self) -> Result<(), String> {
        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.logging.log_file)
            .map_err(|e| format!("Failed to open log file: {}", e))?;

        self.log_handle = Some(log_file);
        Ok(())
    }

    pub fn load_passphrase(&self) -> Result<String, String> {
        fs::read_to_string(&self.config.security.passphrase_file)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("Failed to read passphrase file: {}", e))
    }

    pub fn create_backup(&mut self) -> Result<(), String> {
        let archive_name = format!(
            "{}-{}",
            self.hostname,
            Local::now().format("%Y-%m-%d-%H%M%S")
        );

        self.log(&format!("Starting backup: {}", archive_name));

        // Build borg create command
        let mut cmd = Command::new("borg");
        cmd.arg("create");

        if self.config.options.show_stats {
            cmd.arg("--stats");
        }
        if self.config.options.show_progress {
            cmd.arg("--progress");
        }
        if self.config.options.one_file_system {
            cmd.arg("--one-file-system");
        }
        if self.config.options.exclude_caches {
            cmd.arg("--exclude-caches");
        }

        cmd.arg(format!("--compression={}", self.config.compression));

        // Add global exclusions
        for pattern in &self.config.exclusions {
            cmd.arg("--exclude").arg(pattern);
        }

        // Build archive path with jobs
        let archive_path = format!("{}::{}", self.config.repository.path, archive_name);
        cmd.arg(&archive_path);

        // Add all enabled job sources
        for job in &self.config.jobs {
            if job.enabled {
                cmd.arg(&job.source);

                // Add job-specific exclusions
                for pattern in &job.exclude {
                    cmd.arg("--exclude").arg(pattern);
                }
            }
        }

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to run borg create: {}", e))?;

        // Borg exit codes:
        // 0 = success
        // 1 = warning (backup completed but some files couldn't be read)
        // 2+ = error (backup failed)
        let exit_code = status.code().unwrap_or(2);
        if exit_code >= 2 {
            return Err(format!("borg create failed with exit code {}", exit_code));
        }

        if exit_code == 1 {
            self.log("Backup created with warnings (some files may have been skipped)");
        } else {
            self.log("Backup created successfully");
        }
        Ok(())
    }

    pub fn prune_backups(&mut self) -> Result<(), String> {
        self.log("Pruning old backups...");

        let mut cmd = Command::new("borg");
        cmd.arg("prune")
            .arg("--list")
            .arg(format!("--prefix={}-", self.hostname))
            .arg(format!("--keep-within={}", self.config.retention.within))
            .arg(format!("--keep-hourly={}", self.config.retention.hourly))
            .arg(format!("--keep-daily={}", self.config.retention.daily))
            .arg(format!("--keep-weekly={}", self.config.retention.weekly))
            .arg(format!("--keep-monthly={}", self.config.retention.monthly))
            .arg(format!("--keep-yearly={}", self.config.retention.yearly))
            .arg(&self.config.repository.path);

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to run borg prune: {}", e))?;

        let exit_code = status.code().unwrap_or(2);
        if exit_code >= 2 {
            return Err(format!("borg prune failed with exit code {}", exit_code));
        }

        self.log("Prune completed");
        Ok(())
    }

    pub fn compact_repository(&mut self) -> Result<(), String> {
        if !self.config.maintenance.auto_compact {
            return Ok(());
        }

        self.log("Compacting repository...");

        let status = Command::new("borg")
            .args(["compact", &self.config.repository.path])
            .status()
            .map_err(|e| format!("Failed to run borg compact: {}", e))?;

        let exit_code = status.code().unwrap_or(2);
        if exit_code >= 2 {
            return Err(format!("borg compact failed with exit code {}", exit_code));
        }

        self.log("Compact completed");
        Ok(())
    }

    pub fn check_repository(&mut self) -> Result<(), String> {
        // Only run on the configured day
        let today = Local::now().weekday().num_days_from_monday() + 1;
        if self.config.maintenance.check_day == 0 || today != self.config.maintenance.check_day {
            return Ok(());
        }

        self.log("Running weekly integrity check...");

        let status = Command::new("borg")
            .args(["check", &self.config.repository.path])
            .status()
            .map_err(|e| format!("Failed to run borg check: {}", e))?;

        let exit_code = status.code().unwrap_or(2);
        if exit_code >= 2 {
            return Err(format!(
                "Repository integrity check failed with exit code {}",
                exit_code
            ));
        }

        self.log("Integrity check passed");
        Ok(())
    }

    pub fn send_failure_notification(&self, error: &str) {
        if !self.config.notifications.enabled {
            return;
        }

        let subject = format!("Backup Failure on {}", self.hostname);
        let body = format!("Borg backup failed: {}", error);

        let _ = Command::new("mail")
            .args(["-s", &subject, &self.config.notifications.email])
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(body.as_bytes())?;
                }
                child.wait()
            });
    }

    pub fn run_backup_cycle(&mut self) -> Result<(), String> {
        self.check_lock()?;
        self.create_lock()?;

        let result = self.run_backup_cycle_inner();

        if let Err(ref e) = result {
            self.log(&format!("ERROR: {}", e));
            self.send_failure_notification(e);
        }

        self.remove_lock();
        result
    }

    fn run_backup_cycle_inner(&mut self) -> Result<(), String> {
        self.open_log()?;

        // Run backup
        self.create_backup()?;

        // Prune old backups
        self.prune_backups()?;

        // Compact repository
        self.compact_repository()?;

        // Check repository (if scheduled)
        self.check_repository()?;

        self.log("Backup cycle complete");
        Ok(())
    }

    pub fn list_archives(&self) -> Result<(), String> {
        let status = Command::new("borg")
            .args(["list", &self.config.repository.path])
            .status()
            .map_err(|e| format!("Failed to run borg list: {}", e))?;

        if !status.success() {
            return Err("borg list failed".to_string());
        }

        Ok(())
    }

    pub fn mount_repository(&self, mount_point: &str) -> Result<(), String> {
        println!("Mounting repository to {}", mount_point);

        let status = Command::new("borg")
            .args(["mount", &self.config.repository.path, mount_point])
            .status()
            .map_err(|e| format!("Failed to run borg mount: {}", e))?;

        if !status.success() {
            return Err("borg mount failed".to_string());
        }

        println!("Mounted successfully!");
        println!("Browse backups: ls {}", mount_point);
        println!("Unmount with: fusermount -u {}", mount_point);

        Ok(())
    }

    pub fn generate_example_config(output_path: &str) -> Result<(), String> {
        fs::write(output_path, DEFAULT_CONFIG)
            .map_err(|e| format!("Failed to write example config: {}", e))?;

        println!("Example configuration written to: {}", output_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_config() {
        let config = Config::load_or_default(None);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.repository.path, "/tmp/borg");
        assert_eq!(config.compression, "lz4");
        assert!(!config.jobs.is_empty());
    }

    #[test]
    fn test_config_jobs() {
        let config = Config::load_or_default(None).unwrap();
        assert!(config.jobs.iter().any(|j| j.name == "system-config"));
        assert!(config.jobs.iter().any(|j| j.name == "user-homes"));
    }

    #[test]
    fn test_retention_policy() {
        let config = Config::load_or_default(None).unwrap();
        assert_eq!(config.retention.within, "24H");
        assert_eq!(config.retention.hourly, 24);
        assert_eq!(config.retention.daily, 7);
    }

    #[test]
    fn test_backup_job_enabled_default() {
        let config = Config::load_or_default(None).unwrap();
        for job in &config.jobs {
            assert!(job.enabled);
        }
    }
}
