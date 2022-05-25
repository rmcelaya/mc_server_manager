use chrono::Local;
use ini::Ini;
use std::{
   ffi::CString,
   path::{Path, PathBuf},
};

use crate::error::*;

pub struct Config {
   pub server_directory: CString,
   pub executable_name: CString,
   pub args: Vec<CString>,

   pub backups_directory: PathBuf,
   pub backups_target: PathBuf,
   pub backups_file_format: String,

   pub telegram_api_token: String,
   pub telegram_user_id: i64,
}

pub struct CheckedConfig {
   pub server_directory: Option<CString>,
   pub executable_name: Option<CString>,
   pub args: Vec<CString>,

   pub backups_directory: Option<PathBuf>,
   pub backups_target: Option<PathBuf>,
   pub backups_file_format: Option<String>,

   pub telegram_api_token: Option<String>,
   pub telegram_user_id: Option<i64>,
}

impl CheckedConfig {
   fn new_empty() -> CheckedConfig {
      CheckedConfig {
         server_directory: None,
         executable_name: None,
         args: Vec::new(),

         backups_directory: None,
         backups_target: None,
         backups_file_format: None,

         telegram_api_token: None,
         telegram_user_id: None,
      }
   }
   fn check(&self) -> bool {
      self.server_directory.is_none()
         || self.executable_name.is_none()
         || self.backups_directory.is_none()
         || self.backups_target.is_none()
         || self.backups_file_format.is_none()
         || self.telegram_api_token.is_none()
         || self.telegram_user_id.is_none()
   }

   fn to_config(self) -> Config {
      Config {
         server_directory: self.server_directory.unwrap(),
         executable_name: self.executable_name.unwrap(),
         args: self.args,

         backups_directory: self.backups_directory.unwrap(),
         backups_target: self.backups_target.unwrap(),
         backups_file_format: self.backups_file_format.unwrap(),

         telegram_api_token: self.telegram_api_token.unwrap(),
         telegram_user_id: self.telegram_user_id.unwrap(),
      }
   }
}

impl Config {
   pub fn new(config_p: &Path) -> GenericResult<Config> {
      let mut config = CheckedConfig::new_empty();

      let conf_file = Ini::load_from_file(config_p)?;

      for (sec, prop) in conf_file.iter() {
         if sec.is_none() {
            return Err(GenericError::Error);
         }
         match sec.unwrap() {
            "General" => {
               for (key, val) in prop.iter() {
                  match key {
                     "server_directory" => {
                        config.server_directory = Some(CString::new(val).unwrap())
                     }
                     "executable_name" => config.executable_name = Some(CString::new(val).unwrap()),
                     "arg" => config.args.push(CString::new(val).unwrap()),
                     _ => (),
                  }
               }
            }
            "Backups" => {
               for (key, val) in prop.iter() {
                  match key {
                     "backups_directory" => config.backups_directory = Some(PathBuf::from(val)),
                     "backups_target" => config.backups_target = Some(PathBuf::from(val)),
                     "backups_file_format" => config.backups_file_format = Some(String::from(val)),
                     _ => (),
                  }
               }
            }

            "Telegram" => {
               for (key, val) in prop.iter() {
                  match key {
                     "api_token" => config.telegram_api_token = Some(String::from(val)),
                     "user_id" => {
                        config.telegram_user_id = Some(String::from(val).parse::<i64>().unwrap())
                     }
                     _ => (),
                  }
               }
            }
            _ => (),
         }
      }

      if config.check() {
         return Err(GenericError::Error);
      }
      //typical file format
      //Backup_%Y-%m-%d-%a

      Ok(config.to_config())
   }

   pub fn generate_backup_name(&self, i: u8) -> PathBuf {
      let time = Local::today();
      let mut tmp = String::new();
      tmp = tmp + &self.backups_file_format + "_" + &i.to_string() + ".tar.gz";
      let file_name = time.format(tmp.as_str());
      let file_name = format!("{}", file_name);
      let file_path = Path::new(&self.backups_directory).join(file_name);

      file_path
   }
}
