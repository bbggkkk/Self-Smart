use super::GitManager;
use anyhow::Result;
use std::process::Command;

impl GitManager {
    pub fn status(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["status", "--short"])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn diff(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["diff"])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn add_all(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to stage files");
        }
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to commit: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn tag(&self, name: &str, message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["tag", "-a", name, "-m", message])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create tag: {}", stderr);
        }
        Ok(())
    }

    pub fn push(&self, remote: &str, branch: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["push", remote, branch, "--tags"])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to push: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn log(&self, count: usize) -> Result<String> {
        let output = Command::new("git")
            .args(["log", "--oneline", &format!("-{}", count)])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(
                self.repo
                    .workdir()
                    .unwrap_or_else(|| self.repo.path()),
            )
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
