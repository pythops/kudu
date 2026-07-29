pub mod debian;
pub mod ubuntu;

use debian::DebianRelease;
use ubuntu::UbuntuRelease;

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use anyhow::Result;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{
    Arch,
    event::{DownloadEvent, Event},
    get_kudu_data_dir,
};

#[derive(Debug, Clone, Copy, strum_macros::Display, Deserialize, Serialize)]
#[non_exhaustive]
pub enum LinuxDistro {
    #[strum(to_string = "Arch Linux (btw)")]
    ArchLinux,
    Debian(DebianRelease),
    Ubuntu(UbuntuRelease),
}

impl Default for LinuxDistro {
    fn default() -> Self {
        LinuxDistro::Debian(DebianRelease::Trixie)
    }
}

impl LinuxDistro {
    pub fn get_file_path(&self, arch: Arch) -> PathBuf {
        let mut path = get_kudu_data_dir().join("storage");

        match self {
            LinuxDistro::Debian(release) => {
                path.push(format!(
                    "debian-{}-{}.qcow2",
                    release.to_string().to_lowercase(),
                    arch.to_string().to_lowercase()
                ));
            }
            LinuxDistro::Ubuntu(release) => {
                path.push(format!(
                    "ubuntu-{}-{}.qcow2",
                    release.to_string().to_lowercase(),
                    arch.to_string().to_lowercase()
                ));
            }
            LinuxDistro::ArchLinux => {
                path.push("arch.qcow2");
            }
        }

        path
    }
    pub fn download(&self, arch: Arch, sender: Sender<Event>, vm_id: uuid::Uuid) -> Result<()> {
        let path = self.get_file_path(arch);

        let url = match self {
            LinuxDistro::Debian(release) => release.get_url(arch),
            LinuxDistro::Ubuntu(release) => release.get_url(arch),
            LinuxDistro::ArchLinux => {
                "https://geo.mirror.pkgbuild.com/images/latest/Arch-Linux-x86_64-cloudimg.qcow2"
                    .to_string()
            }
        };

        if !path.exists() {
            let client = Client::new();
            let mut response = client.get(url).send()?.error_for_status()?;

            let file_size: usize = response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|len| len.to_str().ok())
                .and_then(|len| len.parse().ok())
                .filter(|&size| size > 0)
                .ok_or(anyhow::anyhow!("Content-Length is missing or zero"))?;

            let mut content = Vec::new();
            let mut buffer = [0; 4096];

            let mut start = Instant::now();

            loop {
                match response.read(&mut buffer) {
                    Ok(n) => {
                        if !content.is_empty() && n == 0 {
                            let _ = sender
                                .send(Event::Download((vm_id, DownloadEvent::Progress(100u8))));
                            fs::write(path, content)?;
                            break;
                        }

                        content.extend_from_slice(&buffer[0..n]);

                        let now = Instant::now();
                        let diff = now.duration_since(start);

                        if diff >= Duration::from_secs(1) {
                            if let Some(rate) = (content.len() * 100).checked_div(file_size) {
                                let rate = rate as u8;
                                let _ = sender
                                    .send(Event::Download((vm_id, DownloadEvent::Progress(rate))));
                            }

                            start = now;
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(e));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn is_available(&self, arch: Arch) -> bool {
        let path = self.get_file_path(arch);
        path.exists()
    }

    pub fn get_from_os_release() -> Result<Option<String>> {
        let os_release_path = Path::new("/etc/os-release");

        if os_release_path.exists() {
            let content = fs::read_to_string(os_release_path)?;
            for line in content.lines() {
                if line.starts_with("ID")
                    && let Some((_, os)) = line.split_once('=')
                {
                    return Ok(Some(os.to_string()));
                }
            }
        }

        Ok(None)
    }
}
