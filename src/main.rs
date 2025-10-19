use anyhow::{anyhow, Context, Result};
use clap::{command, Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DOCKER_IMAGE_NAME: &str = "safecrate_default";
const CONTAINER_SUFFIX: &str = "isolated";

/// Safecrate — safely open and build untrusted code in isolated Docker sandboxes.
#[derive(Parser)]
#[command(name = "safecrate")]
#[command(about = "Safely open and run untrusted code in isolated environments.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a safecrate base image
    Init {
        /// Custom Dockerfile (overrides default)
        #[arg(long)]
        dockerfile: Option<PathBuf>,
    },

    /// Open a directory in an isolated container
    Open {
        /// Directory to open
        dir: PathBuf,

        /// Command to run inside container (default: nvim)
        #[arg(long, default_value = "nvim .")]
        cmd: String,

        /// Do not remove container after exit
        #[arg(long)]
        keep_container: bool,

        /// Disable network
        #[arg(long)]
        no_network: bool,
    },

    /// Open a previously created container
    Resume {
        /// Project directory to resume container for
        dir: PathBuf,
    },

    /// Remove a previously created container
    Remove {
        /// Project directory whose container to remove
        dir: PathBuf,

        /// Force remove even if running
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dockerfile } => init(dockerfile),
        Commands::Open {
            dir,
            cmd,
            keep_container,
            no_network,
        } => open(dir, cmd, keep_container, no_network),
        Commands::Resume { dir } => resume(dir),
        Commands::Remove { dir, force } => remove(dir, force),
    }
}

/// Build a Docker image for isolated (by default Rust + Neovim) environment.
fn init(dockerfile: Option<PathBuf>) -> Result<()> {
    let dockerfile_path = if let Some(path) = dockerfile {
        path
    } else {
        let dockerfile_content = include_str!("Dockerfile.template");
        let tmp_path = std::env::temp_dir().join("Dockerfile.safecrate");
        fs::write(&tmp_path, dockerfile_content)
            .context("Failed to write temporary Dockerfile")?;
        tmp_path
    };

    let args = &[
        "build",
        "-t",
        DOCKER_IMAGE_NAME,
        "-f",
        dockerfile_path.to_str().unwrap(),
        ".",
    ];

    run_docker_command(args, "Docker build failed")?;

    println!("\n✅ Built the base image!");
    println!("⚠️  WARNING: Running untrusted code in Docker is NOT 100% secure.");
    println!("\tDocker escape is still possible. For maximum safety, run inside a full VM (e.g., VMWare, VirtualBox, QEMU).");
    println!("\nUsage:");
    println!("\t$> safecrate open UNTRUSTED_CODE_DIR");

    Ok(())
}

/// Run container with isolated environment and mount the given directory.
fn open(dir: PathBuf, cmd: String, keep_container: bool, no_network: bool) -> Result<()> {
    let container_name = get_container_name(&dir)?;

    let mut docker_args = vec!["run", "-it"];
    if !keep_container {
        docker_args.push("--rm");
    }
    docker_args.extend(&["--name", &container_name]);

    if !no_network {
        docker_args.extend(&["--network", "bridge"]);
    }

    let abs_dir = std::fs::canonicalize(&dir)?;
    let volume_mapping = format!("{}:/workspace", abs_dir.display());
    docker_args.extend(&["-v", &volume_mapping, "-w", "/workspace"]);
    docker_args.push(DOCKER_IMAGE_NAME);
    docker_args.extend(&["sh", "-c", &cmd]);

    run_docker_command(&docker_args, "Failed to open container")
}

/// Resume a previously created container for the given directory.
fn resume(dir: PathBuf) -> Result<()> {
    let container_name = get_container_name(&dir)?;

    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={}", container_name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;
    let exists = !String::from_utf8_lossy(&output.stdout).trim().is_empty();

    if !exists {
        return Err(anyhow!(
            "No existing container to resume. Run `safecrate open` first with --keep-container."
        ));
    }

    run_docker_command(&["start", "-ai", &container_name], "Failed to resume container")
}

/// Remove a previously created container for the given directory.
fn remove(dir: PathBuf, force: bool) -> Result<()> {
    let container_name = get_container_name(&dir)?;

    let mut args = vec!["rm"];
    if force {
        args.push("-f");
    }
    args.push(&container_name);

    run_docker_command(&args, "Failed to remove container")?;

    println!("✅ Removed container {}", container_name);
    Ok(())
}

/// Helper to get the container name from a directory path.
fn get_container_name(dir: &Path) -> Result<String> {
    let abs_dir = std::fs::canonicalize(dir)?;
    let project_name = abs_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid directory name: {}", dir.display()))?;
    Ok(format!("{}_{}", project_name, CONTAINER_SUFFIX))
}

/// Helper to run a Docker command and provide better error context.
fn run_docker_command(args: &[&str], error_message: &str) -> Result<()> {
    let mut command = Command::new("docker");
    command.args(args).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = command.status().context(format!(
        "Failed to execute docker command. Is docker installed and running?"
    ))?;

    if !status.success() {
        return Err(anyhow!(
            "{}. Docker command exited with non-zero status.",
            error_message
        ));
    }

    Ok(())
}
