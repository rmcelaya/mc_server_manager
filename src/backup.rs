use crate::*;
use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::{fs::File, fs::OpenOptions, io, path::Path};

pub fn backup(config: &Config) -> GenericResult<()> {
   let mut i: u8 = 0;
   loop {
      let file_path = config.generate_backup_name(i);
      if let Err(err) = compress_directory(&config.backups_target.as_path(), file_path.as_path()) {
         if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(err.into());
         }
      } else {
         return Ok(());
      }
      if i == 255 {
         return Err("Limit of 256 daily backups exceeded".into());
      }
      i = i + 1;
   }
}

fn compress_directory(origin_path: &Path, destination_path: &Path) -> Result<(), std::io::Error> {
   let tar_gz = OpenOptions::new()
      .write(true)
      .create_new(true)
      .open(destination_path)?;
   let enc = GzEncoder::new(tar_gz, Compression::default());
   let mut tar = tar::Builder::new(enc);
   let name = Path::new(origin_path)
      .file_name()
      .expect("Encoding error in the path");
   tar.append_dir_all(origin_path, name)?;
   return Ok(());
}

pub fn compute_hash(f_path: &Path) -> GenericResult<Vec<u8>> {
   let mut file = File::open(f_path)?;
   let mut sha256 = Sha256::new();
   io::copy(&mut file, &mut sha256)?;
   let hash = sha256.result();
   let hash = Vec::from(hash.as_slice());
   return Ok(hash);
}
