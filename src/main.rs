use anyhow::{anyhow, Result};
use clap::{command, Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    // If user provides a Dockerfile, use it
    let dockerfile_path = if let Some(path) = dockerfile {
        path
    } else {
        // Otherwise, write embedded template to a temp file
        let dockerfile_content = include_str!("Dockerfile.template");
        let tmp_path = std::env::temp_dir().join("Dockerfile.safecrate");
        fs::write(&tmp_path, dockerfile_content)?;
        tmp_path
    };

    let status = Command::new("docker")
        .args(["build", "-t", "safecrate_default", "-f"])
        .arg(&dockerfile_path)
        .arg(".")
        .status()?;

    if !status.success() {
        eprint!("Docker build failed!");
    }

    println!("\n✅ Built the base image!");
    println!("⚠️  WARNING: Running untrusted code in Docker is NOT 100% secure.");
    println!("\tDocker escape is still possible. For maximum safety, run inside a full VM (e.g., VMWare, VirtualBox, QEMU).");
    println!("\nUsage:");
    println!("\t$> safecrate open UNTRUSTED_CODE_DIR");

    Ok(())
}

/// Run container with isolated encironment and mount the given directory.
fn open(dir: PathBuf, cmd: String, keep_container: bool, no_network: bool) -> Result<()> {
    let abs_dir = std::fs::canonicalize(&dir)?;
    let project_name = abs_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid directory name"))?;

    let container_name = format!("{}_isolated", project_name);

    let mut docker_args = vec![String::from("run"), String::from("-it")];
    if !keep_container {
        docker_args.push(String::from("--rm"));
    }
    docker_args.push(String::from("--name"));
    docker_args.push(container_name);

    if !no_network {
        docker_args.push(String::from("--network"));
        docker_args.push(String::from("bridge"));
    }
    docker_args.push(String::from("-v"));
    docker_args.push(format!("{}:/workspace", abs_dir.display()));
    docker_args.push(String::from("-w"));
    docker_args.push(String::from("/workspace"));
    docker_args.push(String::from("safecrate_default"));

    // Split cmd into words (space-separated)
    docker_args.extend(cmd.split_whitespace().map(str::to_string));

    let status = Command::new("docker").args(docker_args).status()?;
    if !status.success() {
        return Err(anyhow!("Failed to open container"));
    }

    Ok(())
}

/// Resume a previously created container for the given directory.
fn resume(dir: PathBuf) -> Result<()> {
    let abs_dir = std::fs::canonicalize(&dir)?;
    let project_name = abs_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid directory name"))?;
    let container_name = format!("{}_isolated", project_name);

    // Check if container exists
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

    // Attach interactively
    let status = Command::new("docker")
        .args(["start", "-ai", &container_name])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to resume container"));
    }

    Ok(())
}

fn remove(dir: PathBuf, force: bool) -> Result<()> {
    let abs_dir = std::fs::canonicalize(&dir)?;
    let project_name = abs_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid directory name"))?;
    let container_name = format!("{}_isolated", project_name);

    let mut args = vec!["rm"];
    if force {
        args.push("-f");
    }
    args.push(&container_name);

    let status = Command::new("docker").args(&args).status()?;

    if !status.success() {
        return Err(anyhow!("Failed to remove container"));
    }

    println!("✅ Removed container {}", container_name);
    Ok(())
}
