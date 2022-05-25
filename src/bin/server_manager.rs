use server_manager::{config::Config, io::*, jobs::*, server_handler::*, *};
use std::{path::Path, process};

const CONFIG_FILE: &str = "./config.ini";

fn main() {
   let config = Config::new(&Path::new(CONFIG_FILE)).unwrap_or_else(|e| {
      println!("Error reading the configuration: {}", e);
      process::exit(-1);
   });

   let jobs_handler = JobManager::start_jobs(&config);

   let out = get_output_sender();
   let input = get_input_receiver();

   infoln!(
      out,
      "Server manager jobs have started. Starting Minecraft Server UwU. New fresh version"
   );

   let mut handler = match ServerHandler::start_server(&config) {
      Ok(s) => s,
      Err(err) => {
         errorln!(out, "Error starting the server: {}", err);
         jobs_handler.terminate_jobs();
         process::exit(-1);
      }
   };

   'main: loop {
      let s = input.recv().unwrap();
      match s.as_str().trim() {
         "stop" => {
            handler.stop_server();
            break 'main;
         }
         "backup" => {
            handler.backup(&config);
            handler = match ServerHandler::start_server(&config) {
               Ok(s) => s,
               Err(err) => {
                  errorln!(out, "Error starting the server: {}", err);
                  jobs_handler.terminate_jobs();
                  process::exit(-1);
               }
            }
         }
         _ => {
            handler.send(s.as_bytes()).unwrap_or_else(|err| {
               errorln!(out, "Error: {}", err);
            });
         }
      }
   }

   infoln!(out, "Exiting...");
   jobs_handler.terminate_jobs();
}
