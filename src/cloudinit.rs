use anyhow::Result;
use chrono::Utc;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub struct Cloudinit;

impl Cloudinit {
    pub fn from_path(user_data_path: &Path, output_path: &Path) -> Result<()> {
        let path = std::env::temp_dir().join("kudu_cloudinit");

        if !path.exists() {
            fs::create_dir(&path)?;
        }

        let root_dir = path.join(format!("{}", Utc::now()));
        fs::create_dir(&root_dir)?;

        let meta_data_path = root_dir.clone().join("meta-data");
        File::create(&meta_data_path)?;

        let data_path = root_dir.clone().join("user-data");
        fs::copy(user_data_path, &data_path)?;

        let mut command = Command::new("xorriso");
        command
            .arg("-as")
            .arg("genisoimage")
            .arg("-output")
            .arg(output_path)
            .arg("-volid")
            .arg("CIDATA")
            .arg("-joliet")
            .arg("-quiet")
            .arg("-rock")
            .arg(data_path)
            .arg(meta_data_path);

        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;

        fs::remove_dir_all(root_dir)?;

        Ok(())
    }

    pub fn create_userdata(username: String, password: String) -> Result<PathBuf> {
        let path = std::env::temp_dir().join("kudu_cloudinit");

        if !path.exists() {
            fs::create_dir(&path)?;
        }

        let path = path.join(format!("user-data-{}", Utc::now().timestamp()));

        let data = format!(
            r#"#cloud-config
ssh_pwauth: true
users:
  - name: {}
    shell: /usr/bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
    plain_text_passwd: {}
    lock_passwd: false
"#,
            username, password
        );

        fs::write(&path, data)?;

        Ok(path)
    }
}
