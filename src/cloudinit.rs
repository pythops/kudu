use anyhow::Result;
use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub struct Cloudinit;

impl Cloudinit {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let mut dir = env::temp_dir();

        dir.push("user-data");
        let user_data_path = dir.clone();
        dir.pop();

        fs::copy(path, &user_data_path)?;

        dir.push("meta-data");
        let meta_data_path = dir.clone();
        dir.pop();

        File::create(&meta_data_path)?;

        dir.push("cloudinit.iso");
        let output_path = dir.clone();

        let mut command = Command::new("xorriso");
        command
            .arg("-as")
            .arg("genisoimage")
            .arg("-output")
            .arg(&output_path)
            .arg("-volid")
            .arg("CIDATA")
            .arg("-joliet")
            .arg("-quiet")
            .arg("-rock")
            .arg(user_data_path)
            .arg(meta_data_path);

        command.stdout(Stdio::null()).stderr(Stdio::null());
        let _ = command.output()?;
        Ok(output_path)
    }
}
