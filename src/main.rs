mod store;
mod totp;

use std::io::{IsTerminal, Read};
use std::path::Path;

use anyhow::{Context, bail};
use clap::Parser;

use crate::store::Store;
use crate::totp::{current_code, parse_secret_input};

#[derive(Parser)]
#[command(
    name = "2fa",
    version,
    about = "Generate TOTP 2FA codes",
    after_help = "\
Examples:
  2fa add github                 Add an account (prompts for secret)
  pbpaste | 2fa add github       Add from clipboard (otpauth:// or data:image QR)
  2fa add github qr.png          Add from a QR image file
  2fa                            Show codes for all accounts
  2fa github                     Show the current code
  2fa github -c                  Show and copy the code
  2fa list                       List accounts and TOTP parameters
  2fa rm github                  Remove an account
  2fa completions bash           Print bash Tab completion for ~/.bashrc

`add` accepts a base32 secret, otpauth:// URL, data:image/...;base64 QR, or a PNG/JPEG QR file.

Enable Tab completion in ~/.bashrc:
  eval \"$(2fa completions bash)\""
)]
struct Cli {
    /// Copy the code to the clipboard
    #[arg(short = 'c', long = "copy")]
    copy: bool,

    /// add, list, rm, completions, or an account name
    action: Option<String>,

    /// Account name for add/rm
    name: Option<String>,

    /// Secret, otpauth:// URL, data URI, or path to a QR image
    payload: Option<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    if std::env::var_os("TWOFA_COMPLETE").is_some() {
        return print_account_names();
    }

    let cli = Cli::parse();
    match (
        cli.action.as_deref(),
        cli.name.as_deref(),
        cli.payload.as_deref(),
    ) {
        (None, None, None) => show(None, cli.copy),
        (Some("add"), Some(name), payload) => {
            reject_copy(cli.copy)?;
            add(name, payload)
        }
        (Some("add"), None, _) => bail!("usage: 2fa add <name> [secret|otpauth|data-uri|qr.png]"),
        (Some("list"), None, None) => {
            reject_copy(cli.copy)?;
            list()
        }
        (Some("list"), _, _) => bail!("usage: 2fa list"),
        (Some("rm"), Some(name), None) => {
            reject_copy(cli.copy)?;
            remove(name)
        }
        (Some("rm"), _, _) => bail!("usage: 2fa rm <name>"),
        (Some("completions"), None, None) | (Some("completions"), Some("bash"), None) => {
            reject_copy(cli.copy)?;
            print_bash_completions()
        }
        (Some("completions"), _, _) => bail!("usage: 2fa completions bash"),
        (Some(name), None, None) => show(Some(name), cli.copy),
        _ => bail!("usage: 2fa [add|list|rm|completions] [name]"),
    }
}

fn print_bash_completions() -> anyhow::Result<()> {
    print!("{}", include_str!("../completions/2fa.bash"));
    Ok(())
}

fn print_account_names() -> anyhow::Result<()> {
    let store = Store::load()?;
    for name in store.names() {
        println!("{name}");
    }
    Ok(())
}

fn reject_copy(copy: bool) -> anyhow::Result<()> {
    if copy {
        bail!("--copy / -c is only valid when showing a code");
    }
    Ok(())
}

fn add(name: &str, payload: Option<&str>) -> anyhow::Result<()> {
    let raw = read_secret(payload)?;
    let credential = parse_secret_input(&raw)?;
    let mut store = Store::load()?;
    store.add(name, &credential)?;
    println!("added {name} ({})", credential.summary());
    Ok(())
}

fn read_secret(payload: Option<&str>) -> anyhow::Result<String> {
    match payload {
        Some("-") => read_stdin(),
        Some(value) if Path::new(value).is_file() => Ok(value.to_string()),
        Some(value) => Ok(value.to_string()),
        None if !std::io::stdin().is_terminal() => read_stdin(),
        None => rpassword::prompt_password("Secret: ").context("failed to read secret"),
    }
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buf)
        .context("failed to read stdin")?;
    Ok(buf)
}

fn list() -> anyhow::Result<()> {
    let store = Store::load()?;
    if store.names().is_empty() {
        eprintln!("no accounts; add one with `2fa add <name>`");
        return Ok(());
    }
    let width = store
        .names()
        .iter()
        .map(|name| name.len())
        .max()
        .unwrap_or(0);
    for name in store.names() {
        let credential = store.credential(name)?;
        println!("{name:width$}  {}", credential.summary());
    }
    Ok(())
}

fn remove(name: &str) -> anyhow::Result<()> {
    let mut store = Store::load()?;
    store.remove(name)?;
    println!("removed {name}");
    Ok(())
}

fn show(name: Option<&str>, copy: bool) -> anyhow::Result<()> {
    let store = Store::load()?;
    let names: Vec<&str> = match name {
        Some(name) => vec![name],
        None => store.names().iter().map(String::as_str).collect(),
    };

    if names.is_empty() {
        if copy {
            bail!("no accounts to copy; add one with `2fa add <name>`");
        }
        eprintln!("no accounts; add one with `2fa add <name>`");
        return Ok(());
    }

    if copy && name.is_none() && names.len() != 1 {
        bail!("specify an account name to copy, e.g. `2fa github -c`");
    }

    let show_name = name.is_none();
    let width = names.iter().map(|n| n.len()).max().unwrap_or(0);
    let mut copied = None;

    for account in names {
        let credential = store.credential(account)?;
        let code = current_code(&credential)?;
        if show_name {
            println!("{account:width$}  {}  {}s", code.code, code.ttl);
        } else {
            println!("{}  {}s", code.code, code.ttl);
        }
        copied = Some(code.code);
    }

    if copy {
        let code = copied.context("no code to copy")?;
        copy_to_clipboard(&code)?;
        eprintln!("copied to clipboard");
    }

    Ok(())
}

fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("failed to open clipboard")?;
    clipboard
        .set_text(text.to_owned())
        .context("failed to copy to clipboard")?;
    Ok(())
}
