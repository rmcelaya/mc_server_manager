use crate::backup::*;
use crate::io::*;
use crate::processes::*;
use crate::*;

use std::{
    io::{BufRead, BufReader},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};

pub const STOP_COMMAND: &[u8] = b"stop\n";
pub const SAVE_COMMAND: &[u8] = b"save-all\n";

const WAITING_TIME: u64 = 60 * 5;

pub struct ServerHandler {
    process_handler: ProcessHandler,
    wanted_dead: Arc<AtomicBool>,
    stdin_writer: PipeWriter,
    jobs: Vec<thread::JoinHandle<()>>,
}

impl ServerHandler {
    pub fn start_server(config: &Config) -> GenericResult<Self> {
        let wanted_dead = Arc::new(AtomicBool::new(false));
        let wanted_dead_c = wanted_dead.clone();

        let shutdown_clos = move || {
            let out = get_output_sender();
            if !wanted_dead_c.load(std::sync::atomic::Ordering::SeqCst) {
                errorln!(out, "Server went brrr");
                warnln!(
                    out,
                    "Some threads could panic as the process is gonna be closed without waiting"
                );
                thread::sleep(time::Duration::from_millis(100));
                process::exit(-1);
            }
        };

        let mut process_handler = ProcessHandler::execute(
            &config.executable_name,
            &config.args,
            &config.server_directory,
            true,
            true,
            true,
            shutdown_clos,
        )?;

        let stdin_writer = process_handler.get_stdin_writer();
        let mut stdout_reader = BufReader::new(process_handler.get_stdout_reader());
        let mut stderr_reader = BufReader::new(process_handler.get_stderr_reader());

        let stdout_clos = move || {
            let out = get_output_sender();
            let mut buf = String::new();
            loop {
                if let Err(_e) = stdout_reader.read_line(&mut buf) {
                    break;
                }
                raw!(out, "{}", buf);
                buf.clear();
            }
        };

        let stderr_clos = move || {
            let out = get_output_sender();
            let mut buf = String::new();
            loop {
                if let Err(_e) = stderr_reader.read_line(&mut buf) {
                    break;
                }
                raw!(out, "{}", buf);
                buf.clear();
            }
        };
        let stdout_thread = thread::spawn(stdout_clos);
        let stderr_thread = thread::spawn(stderr_clos);

        let mut jobs = Vec::with_capacity(2);
        jobs.push(stdout_thread);
        jobs.push(stderr_thread);

        Ok(Self {
            process_handler,
            wanted_dead,
            stdin_writer,
            jobs,
        })
    }

    pub fn stop_server(mut self) {
        let out = get_output_sender();
        self.wanted_dead.store(true, Ordering::SeqCst);
        if let Err(e) = self.stdin_writer.write_all(STOP_COMMAND) {
            if self.process_handler.is_dead() {
                warnln!(out, "Server is already dead!");
            } else {
                errorln!(out, "{}", e);
                warnln!(out, "Forcing server to stop");
            }
        }

        if let Err(_e) = self.process_handler.wait(WAITING_TIME) {
            warnln!(
                out,
                "Server didn't stop after {} seconds. Forcing it to stop.",
                WAITING_TIME
            );
        };

        self.process_handler.force_kill();
        for j in self.jobs {
            j.join().expect("Error when joining threads");
        }
    }

    pub fn send(&mut self, command: &[u8]) -> GenericResult<()> {
        return self.stdin_writer.write_all(command);
    }

    pub fn sendln(&mut self, command: &[u8]) -> GenericResult<()> {
        let mut tmp: Vec<u8> = Vec::from(command);
        tmp.push('\n' as u8);
        return self.send(&tmp);
    }

    pub fn backup(mut self, config: &Config) {
        let out = get_output_sender();

        infoln!(out, "Saving and closing the server");

        self.wanted_dead.store(true, Ordering::SeqCst);
        if let Err(e) = self.stdin_writer.write_all(SAVE_COMMAND) {
            errorln!(out, "Error: {}", e);
            warnln!(out, "Forcing server to stop");
        }
        if let Err(e) = self.stdin_writer.write_all(STOP_COMMAND) {
            errorln!(out, "Error 2: {}", e);
            warnln!(out, "Forcing server to stop");
        }
        if let Err(_e) = self.process_handler.wait(WAITING_TIME) {
            warnln!(
                out,
                "Server didn't stop after {} seconds. Forcing it to stop",
                WAITING_TIME
            );
        };

        self.process_handler.force_kill();

        for j in self.jobs {
            j.join().expect("Error when joining threads");
        }

        infoln!(out, "Creating backup...");
        if let Err(e) = backup(&config) {
            errorln!(out, "Could not make backup. Error: {}", e);
        };
    }
}
