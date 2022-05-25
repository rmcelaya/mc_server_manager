use crate::{io::*, *};

use std::thread;
use std::thread::JoinHandle;

/*
Trait is not used right now. It's kept here to add support
for flexible modularity in the future
*/

trait JobCleaner {
    fn terminate(self: Box<Self>);
}

pub struct JobManager {
    sync_jobs: Vec<Box<dyn JobCleaner>>,

    telegram_cleaner: TelegramManagerCleaner,
    async_thread_handler: JoinHandle<()>,
}

impl JobManager {
    pub fn start_jobs(config: &Config) -> JobManager {
        //Sync jobs
        let mut jobs: Vec<Box<dyn JobCleaner>> = Vec::with_capacity(1);
        jobs.push(Box::new(OutputManagerJob::start(config)));
        jobs.push(Box::new(StdinManagerJob::start()));

        //Async Jobs
        let (telegram_cleaner, telegram_routine) =
            telegram_job(&config.telegram_api_token, config.telegram_user_id);

        //let (discord_cleaner, discord_routine) =
        //    discord_job(&config.telegram_api_token, config.telegram_user_id);

        //Spawning asynchronous background jobs in a different thread
        let async_thread_handler = std::thread::spawn(move || {
            let async_routine = async move {
                tokio::join!(telegram_routine);
            };
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .enable_all()
                .build()
                .expect("Failed to start tokio runtime");
            rt.block_on(async_routine);
        });

        let sync_jobs = jobs;
        JobManager {
            sync_jobs,
            telegram_cleaner,
            async_thread_handler,
        }
    }

    pub fn terminate_jobs(self) {
        for j in self.sync_jobs {
            j.terminate();
        }
        self.telegram_cleaner.terminate();
        self.async_thread_handler.join().unwrap();
    }
}

/********* Telegram receiving job ********/

use futures::Future;
use tokio::sync::oneshot;

fn telegram_job(api_token: &str, authorized_user_id: i64) -> (TelegramManagerCleaner, impl Future) {
    let mut bot = tbot::Bot::new(String::from(api_token)).event_loop();
    bot.text(move |context| async move {
        let out = get_output_sender();
        let input = get_input_sender();
        let data = &context.text.value;
        let user = &context.from.as_ref().unwrap().first_name;
        let user_id = context.from.as_ref().unwrap().id.0;
        infoln!(out, "Telegram message from {}: {}", user, data);
        if user_id == authorized_user_id {
            let mut data = String::from(data);
            data.push_str("\n");
            input.send(data).unwrap();
        } else {
            warnln!(
                out,
                "Telegram user does not have permission to send commands"
            );
            warnln!(out, "User: {}", user_id);
        }
    });

    let (tx_end, rx_end): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();

    let bot_job = bot.polling().start();
    let async_routine = async move {
        tokio::select! {
            _ = bot_job => (),
            _ = rx_end => ()
        };
    };
    let cleaner = TelegramManagerCleaner { tx_end };

    (cleaner, async_routine)
}

struct TelegramManagerCleaner {
    tx_end: oneshot::Sender<()>,
}

impl TelegramManagerCleaner {
    fn terminate(self) {
        self.tx_end.send(()).unwrap();
    }
}

/******** Discord receiving job  *********/
/*
use serenity::{
    model::{channel::Message, gateway::Ready},
    prelude::*
};

struct DiscHandler;

impl EventHandler for DiscHandler {

    fn message(&self, ctx: Context, msg: Message) {
        let input = get_input_sender();
        let m = format!("/say Message from Discord user {}: {}",msg.author.name,msg.content);
        input.send(m).unwrap();
     }

}



fn discord_job(api_token: &str, authorized_user_id: i64) -> (DiscordManagerCleaner, impl Future) {
    let mut client = serenity::client::Client::new(api_token,DiscHandler).expect("Failed to connect a discord");
    let bot_job = async{
        client.star
    };
    let (tx_end, rx_end): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();

    let async_routine = async move {
        tokio::select! {
            _ = bot_job => (),
            _ = rx_end => ()
        };
    };
    let cleaner = DiscordManagerCleaner { tx_end };

    (cleaner, async_routine)
}

struct DiscordManagerCleaner {
    tx_end: oneshot::Sender<()>,
}

impl DiscordManagerCleaner {
    fn terminate(self) {
        self.tx_end.send(()).unwrap();
    }
}
*/

/******** Output managing job ***********/
use crate::telegram::*;

struct OutputManagerJob {
    handle: thread::JoinHandle<()>,
}

impl OutputManagerJob {
    fn start(config: &Config) -> OutputManagerJob {
        let recv = get_output_receiver();
        let mut tel_out =
            TelegramMessageSender::new(&config.telegram_api_token, config.telegram_user_id);

        let handle = std::thread::spawn(move || 'main: loop {
            let s = match recv.recv() {
                Ok(s) => s,
                Err(_e) => break 'main,
            };

            let level_str;
            match s {
                OutputPacket::Terminate => break 'main,

                OutputPacket::Message { level, message } => {
                    match level {
                        OutputMessageType::Error => level_str = "ERROR",
                        OutputMessageType::Warning => level_str = "WARN",
                        OutputMessageType::Info => level_str = "INFO",
                        OutputMessageType::Debug => level_str = "DEBUG",
                        OutputMessageType::Raw => level_str = "MINECRAFT",
                    };
                    let out_s = format!("[{}] {}", level_str, message);
                    print!("{}", out_s);
                    if let Err(e) = tel_out.send_message(&out_s) {
                        println!("[WARN] Could not send log to telegram:{}", e);
                    }
                }
            }
        });
        OutputManagerJob { handle }
    }
}

impl JobCleaner for OutputManagerJob {
    fn terminate(self: Box<Self>) {
        let out = get_output_sender();
        out.send(OutputPacket::Terminate).unwrap();
        self.handle.join().unwrap();
    }
}

/****** Stdin input managing ******/

/*
    Job that is blocked until input is received,
    and thus, its not terminated. It will end when
    the process exits

    Stdin manager job is kept as a separated job in a different thread,
    because that would be what would happen if it was spawned as an asynchronous
    task in tokio under the hood.

    IDEA: Implemented using tokio io. Even working the same way under the hood,
    we would be able to cancel the thread whenever needed sending a signal. Maybe
    not needed
*/

//We keep the handle for future compat
struct StdinManagerJob {
    _handle: thread::JoinHandle<()>,
}

impl StdinManagerJob {
    fn start() -> StdinManagerJob {
        let _handle = thread::spawn(|| {
            let sender = get_input_sender();
            let out = get_output_sender();
            let mut input = String::new();
            'main: loop {
                input.clear();
                if let Err(e) = std::io::stdin().read_line(&mut input) {
                    error!(out, "Stdin input error: {}", e);
                };
                if let Err(_e) = sender.send(input.clone()) {
                    break 'main;
                };
            }
        });
        StdinManagerJob { _handle }
    }
}

impl JobCleaner for StdinManagerJob {
    fn terminate(self: Box<Self>) {}
}
