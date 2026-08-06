use anyhow::Result;
use std::{
    fs::{self, File},
    path::Path,
    process::{Command, Stdio},
};

pub struct Cloudinit;

impl Cloudinit {
    pub fn from_path(user_data_path: &Path, output_path: &Path) -> Result<()> {
        let path = std::env::temp_dir().join("kudu_cloudinit");

        fs::create_dir(&path)?;

        let meta_data_path = path.clone().join("meta-data");
        File::create(&meta_data_path)?;

        let data_path = path.clone().join("user-data");
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

        fs::remove_dir_all(path)?;

        Ok(())
    }
}
