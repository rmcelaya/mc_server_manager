use std::ffi::{CStr, CString};

use crate::*;
use std::os::raw::c_char;

use libc::{c_int, pid_t, size_t, ssize_t, strerror};
use std::{
   io::Read,
   io::Write,
   sync::{
      atomic::{AtomicBool, Ordering},
      Arc,
   },
   thread::{self, JoinHandle},
   time::Duration,
};

/********* C functions*****************/

#[link(name = "c_processes")]
extern "C" {
   fn execute(
      command: *const c_char,
      arguments: *const *const c_char,
      server_directory: *const c_char,
      d: &mut ProcDescriptor,
      e_info: *mut c_int,
      pipe_input: c_int,
      pipe_output: c_int,
      pipe_err: c_int,
   ) -> c_int;
   fn c_write(fd: c_int, command: *const u8, s: size_t, e_info: *mut c_int) -> ssize_t;

   fn c_read(fd: c_int, command: *const u8, s: size_t, e_info: *mut c_int) -> ssize_t;
   fn c_kill(pid: pid_t, level: c_int);
   fn c_wait(pid: pid_t) -> c_int;
   fn c_wait_forever(pid: pid_t);

}

/*
Fields not wrapped in an Arc will be cloned
in different threads and should be read only once
they have been initialized when the server has started.
Thread handlers are not cloned
*/

pub enum KillLevel {
   SIGTERM,
   SIGKILL,
}

#[repr(C)]
struct ProcDescriptor {
   proc_pid: pid_t,
   proc_stdin: c_int,
   proc_stdout: c_int,
   proc_stderr: c_int,
}

pub struct ProcessHandler {
   proc_pid: pid_t,
   stdin_writer: Option<PipeWriter>,
   stdout_reader: Option<PipeReader>,
   stderr_reader: Option<PipeReader>,
   dead: Arc<AtomicBool>,
   dead_waiter_handler: Option<JoinHandle<()>>,
}

impl ProcessHandler {
   pub fn execute<F>(
      command: &CString,
      arguments: &Vec<CString>,
      server_directory: &CString,
      pipe_input: bool,
      pipe_output: bool,
      pipe_err: bool,
      shutdown_routine: F,
   ) -> Result<ProcessHandler, GenericError>
   where
      F: FnOnce() + Send + 'static,
   {
      let mut pd = ProcDescriptor {
         proc_pid: 0,
         proc_stdin: 0,
         proc_stdout: 0,
         proc_stderr: 0,
      };

      let mut serv = ProcessHandler {
         proc_pid: 0,
         stdin_writer: None,
         stdout_reader: None,
         stderr_reader: None,
         dead: Arc::new(AtomicBool::new(true)),
         dead_waiter_handler: None,
      };

      let comm = command.as_ptr();
      let mut args: Vec<*const c_char> = Vec::new();
      args.push(command.as_ptr());
      args.extend(arguments.iter().map(|arg| arg.as_ptr()));
      args.push(std::ptr::null());
      let args = args.as_ptr();

      let e;
      let mut e_info: c_int = 0;
      unsafe {
         e = execute(
            comm,
            args,
            server_directory.as_ptr(),
            &mut pd,
            &mut e_info,
            pipe_input as c_int,
            pipe_output as c_int,
            pipe_err as c_int,
         );
         if e != 0 {
            let e_str = CStr::from_ptr(strerror(e_info))
               .to_str()
               .expect("Converting errno string to UTF8 failed");
            match e {
               -1 => return Err(format!("Error when creating process pipes: {}", e_str).into()),
               -2 => {
                  return Err(
                     format!("Error when trying to fork the process process: {}", e_str).into(),
                  )
               }
               _ => (),
            }
         };
      };
      serv.dead.store(false, Ordering::SeqCst);

      serv.proc_pid = pd.proc_pid;

      if pipe_input {
         serv.stdin_writer = Some(PipeWriter::new(pd.proc_stdin));
      }
      if pipe_output {
         serv.stdout_reader = Some(PipeReader::new(pd.proc_stdout));
      }
      if pipe_err {
         serv.stderr_reader = Some(PipeReader::new(pd.proc_stderr));
      }

      let mut serv2 = serv.clone();
      serv.dead_waiter_handler = Some(thread::spawn(move || {
         serv2.wait(0).unwrap();
         serv2.dead.store(true, Ordering::SeqCst);
         shutdown_routine();
      }));

      Ok(serv)
   }

   pub fn is_dead(&mut self) -> bool {
      if self.dead.load(Ordering::SeqCst) {
         return true;
      }
      unsafe {
         match c_wait(self.proc_pid) {
            0 => false,
            _ => {
               self.dead.store(true, Ordering::SeqCst);
               true
            }
         }
      }
   }

   //For instant wait use method is_dead
   pub fn wait(&mut self, time: u64) -> Result<(), GenericError> {
      if time == 0 {
         unsafe { c_wait_forever(self.proc_pid) };
         return Ok(());
      }

      //IDEA: implement asynchronously with a thread that
      //takes care of all the timers
      let timeout = Arc::new(AtomicBool::new(false));
      let timeout_clone = timeout.clone();
      thread::spawn(move || {
         thread::sleep(Duration::from_secs(time));
         timeout_clone.store(true, Ordering::SeqCst);
      });

      while !self.is_dead() {
         if timeout.load(Ordering::SeqCst) {
            return Err(GenericError::Error);
         }
      }
      Ok(())
   }

   pub fn kill(&mut self, lev: &KillLevel) {
      //Direct cast could be made but this way things are more clear
      let c: c_int = match lev {
         KillLevel::SIGTERM => 0,
         KillLevel::SIGKILL => 1,
      };

      unsafe {
         c_kill(self.proc_pid, c);
      }
   }

   pub fn force_kill(&mut self) {
      loop {
         if self.is_dead() {
            break;
         } else {
            self.kill(&KillLevel::SIGKILL);
         }
      }
   }

   pub fn get_stdin_writer(&mut self) -> PipeWriter {
      self
         .stdin_writer
         .take()
         .expect("Tried to take stdin and it doesn't exist")
   }

   pub fn get_stdout_reader(&mut self) -> PipeReader {
      self
         .stdout_reader
         .take()
         .expect("Tried to take stdout reader and it doesn't exist")
   }

   pub fn get_stderr_reader(&mut self) -> PipeReader {
      self
         .stderr_reader
         .take()
         .expect("Tried to take stdout reader and it doesn't exist")
   }

   fn join_dead_waiter(&mut self) {
      if self.dead_waiter_handler.is_some() {
         self
            .dead_waiter_handler
            .take()
            .unwrap()
            .join()
            .expect("Panic! at the dead waiter thread");
      }
   }
}

/*
   Not implemenenting the Clone trait and to avoid making the clone method
   public. Don't want myself to clone this struct outside this file by mistake
   in the future.
*/
impl ProcessHandler {
   fn clone(&self) -> ProcessHandler {
      ProcessHandler {
         proc_pid: self.proc_pid,
         stdin_writer: None,
         stdout_reader: None,
         stderr_reader: None,
         dead: self.dead.clone(),
         dead_waiter_handler: None,
      }
   }
}

impl Drop for ProcessHandler {
   fn drop(&mut self) {
      self.force_kill();
      self.join_dead_waiter();
   }
}

//PipeReader and PipeWriter will not implement cloning because the c functions that make it work
//are not thread safe. To safely use PipeReader or PipeWriter across multiple threads use
//Arc from the standard library

pub struct PipeReader {
   fd: c_int,
}

impl PipeReader {
   fn new(fd: c_int) -> PipeReader {
      PipeReader { fd }
   }
}

impl Read for PipeReader {
   fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
      unsafe {
         let mut e_info = 0;
         let e = c_read(self.fd, buf.as_ptr(), buf.len(), &mut e_info);
         if e == -4 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, ""));
         } else if e == -5 {
            return Err(std::io::Error::from_raw_os_error(e_info));
         }
         Ok(e as usize)
      }
   }
}

pub struct PipeWriter {
   fd: c_int,
}
impl PipeWriter {
   fn new(fd: c_int) -> PipeWriter {
      PipeWriter { fd }
   }
}

impl PipeWriter {
   pub fn write_all(&mut self, buf: &[u8]) -> GenericResult<()> {
      let e = self.write(buf)?;
      if e != buf.len() {
         return Err("Could not write the whole command to the pipe".into());
      }
      Ok(())
   }
}

impl Write for PipeWriter {
   fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
      unsafe {
         let mut e_info = 0;
         let e = c_write(self.fd, buf.as_ptr(), buf.len(), &mut e_info);
         if e == -3 {
            return Err(std::io::Error::from_raw_os_error(e_info));
         }
         Ok(e as usize)
      }
   }

   fn flush(&mut self) -> std::io::Result<()> {
      Ok(())
   }
}
