use clap::Parser;
use std::{error::Error, path::PathBuf, process::Command};

#[derive(Parser, Debug)]
struct Opts {
    // A name of S3 bucket
    #[arg(short = 'b', long = "bucket", env = "GIT_S3_BUCKET")]
    bucket: String,

    /// Prefix used to store snapshots on S3
    #[arg(short = 'p', long = "prefix", env = "GIT_S3_PREFIX")]
    prefix: String,

    /// Path to a git repository. A current directory is used by default.
    #[arg(short = 'r', long = "root", env = "GIT_ROOT")]
    git_root: PathBuf,
}

#[::tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenvy::dotenv();
    if let Ok(dir) = std::env::current_dir() {
        std::env::set_var("GIT_ROOT", dir);
    }

    let opts = Opts::parse();

    let aws_config = aws_config::load_from_env().await;

    let tempdir = tempfile::tempdir()?;

    Command::new("git")
        .arg("clone")
        .arg("--depth=1")
        .arg(&opts.git_root)
        .arg("repo")
        .current_dir(tempdir.path())
        .output()?;

    let repo_path = tempdir.path().join("repo");

    let _ = std::fs::remove_dir_all(repo_path.join(".git"));

    let archive_path = tempdir.path().join("snapshot.tar.gz");

    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg(".")
        .current_dir(&repo_path)
        .output()?;

    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let key = key_with_prefix(&opts.prefix, "snaphot.tar.gz");
    let body = aws_sdk_s3::primitives::ByteStream::from_path(archive_path).await?;
    let _response = s3_client
        .put_object()
        .bucket(&opts.bucket)
        .key(&key)
        .body(body)
        .send()
        .await?;

    println!(
        "Success. Snapshot is published in s3://{}/{}",
        opts.bucket, key
    );

    Ok(())
}

fn key_with_prefix(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}/{key}")
    }
}
