pub mod ops;

use anyhow::Result;
use gix::Repository;
use std::path::Path;

pub struct GitManager {
    repo: Repository,
}

impl GitManager {
    pub fn new(workdir: &str) -> Result<Self> {
        let repo = gix::open(workdir).map_err(|e| anyhow::anyhow!("Failed to open repo: {}", e))?;
        Ok(Self { repo })
    }

    pub fn init(workdir: &str) -> Result<Self> {
        let path = Path::new(workdir);
        let repo = gix::init(path).map_err(|e| anyhow::anyhow!("Failed to init repo: {}", e))?;
        Ok(Self { repo })
    }

    pub fn repository(&self) -> &Repository {
        &self.repo
    }
}
