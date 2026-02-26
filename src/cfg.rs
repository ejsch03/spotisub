use std::path::PathBuf;

use clap::Parser;

use crate::prelude::*;

fn local_addr() -> SocketAddr {
    let ip = local_ip_address::list_afinet_netifas()
        .ok()
        .and_then(|addrs| {
            addrs.into_iter().find_map(|(name, ip)| match ip {
                IpAddr::V4(v4)
                    if !v4.is_loopback()
                        && !v4.is_link_local()
                        && ["Ethernet", "Wi-Fi"].contains(&name.as_str()) =>
                {
                    Some(ip)
                }
                _ => None,
            })
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    SocketAddr::new(ip, 4040)
}

#[derive(Debug, serde::Deserialize)]
struct CredentialsConfig {
    user: String,
    pass: String,
    client_id: String,
    client_secret: String,
}

#[derive(Clone, Debug)]
pub struct Account {
    user: String,
    pass: String,
}

impl Account {
    pub const fn user(&self) -> &str {
        self.user.as_str()
    }

    pub const fn pass(&self) -> &str {
        self.pass.as_str()
    }
}

#[derive(Clone, Debug)]
pub struct Dev {
    client_id: String,
    client_secret: String,
}

impl Dev {
    pub const fn client_id(&self) -> &str {
        self.client_id.as_str()
    }

    pub const fn client_secret(&self) -> &str {
        self.client_secret.as_str()
    }
}

#[derive(Clone, Debug)]
pub struct Credentials {
    account: Account,
    dev: Dev,
}

impl Credentials {
    pub const fn account(&self) -> &Account {
        &self.account
    }

    pub const fn dev(&self) -> &Dev {
        &self.dev
    }
}

#[derive(Debug, Parser)]
pub struct ArgsConfig {
    #[arg(short, long, default_value_t = local_addr())]
    addr: SocketAddr,

    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Config {
    addr: SocketAddr,
    cred: Credentials,
}

impl Config {
    pub fn new() -> Result<Self> {
        let args = ArgsConfig::parse();

        // obtain config path
        let config_path = args
            .config_path
            .map(Ok::<_, anyhow::Error>)
            .unwrap_or_else(|| {
                let mut home = std::env::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to obtain home directory."))?;
                home.push(format!("{}.json", env!("CARGO_PKG_NAME")));
                Ok(home)
            })?;

        // open the config file
        let rdr = std::fs::File::open(&config_path).map_err(|e| {
            if let std::io::ErrorKind::NotFound = e.kind() {
                anyhow!("Failed to open '{}'.", config_path.display())
            } else {
                e.into()
            }
        })?;
        // read and deserialize from file
        let CredentialsConfig {
            user,
            pass,
            client_id,
            client_secret,
        } = serde_json::from_reader::<_, CredentialsConfig>(rdr)?;

        let addr = args.addr;
        let cred = Credentials {
            account: Account { user, pass },
            dev: Dev {
                client_id,
                client_secret,
            },
        };
        Ok(Self { addr, cred })
    }

    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn cred(&self) -> Credentials {
        self.cred.clone()
    }
}
