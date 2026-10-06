use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "2fa";

#[derive(Default, Serialize, Deserialize)]
struct AccountsFile {
    accounts: Vec<String>,
}

pub struct Store {
    path: PathBuf,
    accounts: Vec<String>,
}

impl Store {
    pub fn load() -> anyhow::Result<Self> {
        let path = accounts_path()?;
        let accounts = if path.exists() {
            let data = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let file: AccountsFile = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            file.accounts
        } else {
            Vec::new()
        };
        Ok(Self { path, accounts })
    }

    pub fn names(&self) -> &[String] {
        &self.accounts
    }

    pub fn add(&mut self, name: &str, credential: &crate::totp::Credential) -> anyhow::Result<()> {
        let name = validate_name(name)?;
        if self.accounts.iter().any(|existing| existing == name) {
            bail!("account '{name}' already exists");
        }

        let entry = Entry::new(SERVICE, name).context("failed to open macOS Keychain")?;
        entry
            .set_password(&credential.to_storage()?)
            .with_context(|| format!("failed to store secret for '{name}' in Keychain"))?;

        self.accounts.push(name.to_string());
        if let Err(err) = self.save() {
            let _ = entry.delete_credential();
            return Err(err);
        }
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> anyhow::Result<()> {
        let Some(index) = self.accounts.iter().position(|existing| existing == name) else {
            bail!("account '{name}' not found");
        };

        let entry = Entry::new(SERVICE, name).context("failed to open macOS Keychain")?;
        match entry.delete_credential() {
            Ok(()) => {}
            Err(keyring::Error::NoEntry) => {}
            Err(err) => {
                return Err(err).context(format!("failed to delete Keychain item for '{name}'"));
            }
        }

        self.accounts.remove(index);
        self.save()
    }

    pub fn credential(&self, name: &str) -> anyhow::Result<crate::totp::Credential> {
        if !self.accounts.iter().any(|existing| existing == name) {
            bail!("account '{name}' not found");
        }
        let entry = Entry::new(SERVICE, name).context("failed to open macOS Keychain")?;
        let stored = entry
            .get_password()
            .with_context(|| format!("failed to read secret for '{name}' from Keychain"))?;
        crate::totp::Credential::from_storage(&stored)
    }

    fn save(&self) -> anyhow::Result<()> {
        let dir = self.path.parent().context("invalid config path")?;
        fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
        set_mode(dir, 0o700)?;

        let data = serde_json::to_string_pretty(&AccountsFile {
            accounts: self.accounts.clone(),
        })?;
        fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        set_mode(&self.path, 0o600)?;
        Ok(())
    }
}

fn accounts_path() -> anyhow::Result<PathBuf> {
    let dirs = directories::BaseDirs::new().context("home directory not found")?;
    Ok(dirs.home_dir().join(".config/2fa/accounts.json"))
}

fn validate_name(name: &str) -> anyhow::Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        bail!("account name is empty");
    }
    if matches!(name, "add" | "list" | "rm" | "completions") {
        bail!("'{name}' is a reserved command name");
    }
    Ok(name)
}

fn set_mode(path: &Path, mode: u32) -> anyhow::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}
