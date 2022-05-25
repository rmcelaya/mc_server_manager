use lazy_static::*;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

type OutputPacketType = OutputPacket;
type InputPacketType = String;

lazy_static! {
    static ref OUT: Mutex<ChannelProvider<OutputPacketType>> = Mutex::new(ChannelProvider::new());
    static ref IN: Mutex<ChannelProvider<InputPacketType>> = Mutex::new(ChannelProvider::new());
}

pub fn get_input_sender() -> Sender<InputPacketType> {
    let tmp = IN.lock().unwrap();
    tmp.get_sender()
}

pub fn get_input_receiver() -> Receiver<InputPacketType> {
    let mut tmp = IN.lock().unwrap();
    tmp.get_receiver()
}

pub fn get_output_sender() -> Sender<OutputPacketType> {
    let tmp = OUT.lock().unwrap();
    tmp.get_sender()
}

pub fn get_output_receiver() -> Receiver<OutputPacketType> {
    let mut tmp = OUT.lock().unwrap();
    tmp.get_receiver()
}

struct ChannelProvider<T> {
    sender: Sender<T>,
    receiver: Option<Receiver<T>>,
}

impl<T> ChannelProvider<T> {
    fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender: sender,
            receiver: Some(receiver),
        }
    }

    fn get_receiver(&mut self) -> Receiver<T> {
        self.receiver.take().unwrap()
    }

    fn get_sender(&self) -> Sender<T> {
        self.sender.clone()
    }
}

//Non memory optimum structure (two level labeled struct), but more readable

pub enum OutputPacket {
    Message {
        level: OutputMessageType,
        message: String,
    },
    Terminate,
}

pub enum OutputMessageType {
    Error,
    Warning,
    Info,
    Debug,
    Raw,
}

#[macro_export]
macro_rules! debug {
    ($out:expr, $($arg:tt)+) => {
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Debug,
            message: format!($($arg)*)
        }).unwrap()
    };
}

#[macro_export]
macro_rules! info{
    ($out:expr, $($arg:tt)+) => {
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Info,
            message: format!($($arg)*)
        }).unwrap()
    };
}

#[macro_export]
macro_rules! error{
    ($out:expr, $($arg:tt)+) => {
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Error,
            message: format!($($arg)*)
        }).unwrap()
    };
}

#[macro_export]
macro_rules! warn {
    ($out:expr, $($arg:tt)+) => {
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Warning,
            message: format!($($arg)*)
        }).unwrap()
    };
}

#[macro_export]
macro_rules! raw {
    ($out:expr, $($arg:tt)+) => {
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Raw,
            message: format!($($arg)*)
        }).unwrap()
    };
}

#[macro_export]
macro_rules! debugln {
    ($out:expr, $($arg:tt)+) => {
        let mut m = format!($($arg)*);
        m.push_str("\n");
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Debug,
            message: m
        }).unwrap()
    };
}

#[macro_export]
macro_rules! infoln{
    ($out:expr, $($arg:tt)+) => {
        let mut m = format!($($arg)*);
        m.push_str("\n");
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Info,
            message: m
        }).unwrap();
    };
}

#[macro_export]
macro_rules! errorln{
    ($out:expr, $($arg:tt)+) => {
        let mut m = format!($($arg)*);
        m.push_str("\n");
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Error,
            message: m
        }).unwrap()
    };
}

#[macro_export]
macro_rules! warnln {
    ($out:expr, $($arg:tt)+) => {
        let mut m = format!($($arg)*);
        m.push_str("\n");
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Warning,
            message: m
        }).unwrap()
    };
}

#[macro_export]
macro_rules! rawln {
    ($out:expr, $($arg:tt)+) => {
        let mut m = format!($($arg)*);
        m.push_str("\n");
        $out.send(OutputPacket::Message{
            level: OutputMessageType::Raw,
            message: m
        }).unwrap()
    };
}
