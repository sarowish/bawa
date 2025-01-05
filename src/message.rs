use crate::event::Event;
use anyhow::Error;
use std::ops::Deref;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

pub enum Kind {
    Info,
    Error,
    Warning,
}

pub struct Message {
    message: String,
    pub kind: Kind,
    token: CancellationToken,
    tx: UnboundedSender<Event>,
}

impl Message {
    pub fn new(tx: UnboundedSender<Event>) -> Self {
        Message {
            message: String::new(),
            kind: Kind::Info,
            token: CancellationToken::new(),
            tx,
        }
    }

    pub fn set_info(&mut self, message: &str) {
        self.token.cancel();

        if !message.is_empty() {
            self.message = message.to_string();
            self.kind = Kind::Info;
            self.token = CancellationToken::new();
        }
    }

    pub fn set_message_with_timeout(&mut self, message: &str, duration: u64) {
        self.set_info(message);
        self.clear_timeout(duration);
    }

    pub fn set_error(&mut self, error: &Error) {
        self.set_info(&error.to_string());
        self.kind = Kind::Error;
        self.clear_timeout(10);
    }

    pub fn set_error_from_str(&mut self, message: &str) {
        self.set_info(message);
        self.kind = Kind::Error;
        self.clear_timeout(10);
    }

    pub fn set_warning(&mut self, message: &str) {
        self.set_info(message);
        self.kind = Kind::Warning;
        self.clear_timeout(10);
    }

    pub fn clear(&mut self) {
        self.message.clear();
        self.token.cancel();
    }

    fn clear_timeout(&mut self, duration: u64) {
        let token = self.token.clone();
        let tx = self.tx.clone();

        tokio::task::spawn(async move {
            tokio::select! {
            () = token.cancelled() => {},
            () = tokio::time::sleep(std::time::Duration::from_secs(duration)) => tx.send(Event::ClearMessage).unwrap() }
        });
    }
}

impl Deref for Message {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}
