#![cfg(target_os = "windows")]

use mt_logger::*;

use message_io::node::{self};
use message_io::network::{NetEvent, Transport};
use std::ffi::OsString;
use winptyrs::{PTY, PTYArgs, MouseMode, AgentConfig};
use std::thread;
use std::sync::{Arc, Mutex};


pub fn main() {
    let pty_args = PTYArgs {
        cols: 80,
        rows: 25,
        mouse_mode: MouseMode::WINPTY_MOUSE_MODE_NONE,
        timeout: 10000,
        agent_config: AgentConfig::WINPTY_FLAG_COLOR_ESCAPES
    };

    // Initialize a pseudoterminal.
    let pty = PTY::new(&pty_args).unwrap();
    let pty = Arc::new(Mutex::new(pty));


    // Create a node, the main message-io entity. It is divided in 2 parts:
    // The 'handler', used to make actions (connect, send messages, signals, stop the node...)
    // The 'listener', used to read events from the network or signals.
    let (handler, listener) = node::split::<()>();

    // Listen for TCP, UDP and WebSocket messages at the same time.
    handler.network().listen(Transport::Ws, "0.0.0.0:7703").unwrap();
    let handler = Arc::new(handler);

    // Read incoming network events.
    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _) => unreachable!(),
        NetEvent::Accepted(endpoint, _listener) => {
            let cmd = OsString::from("c:\\windows\\system32\\cmd.exe").to_owned();

            // Spawn a process inside the pseudoterminal.
            pty.lock().unwrap().spawn(cmd, None, None, None).unwrap();

            let pty_clone = Arc::clone(&pty);
            let handler_clone = Arc::clone(&handler);
            thread::spawn(move || {
                loop {
                    let output = pty_clone.lock().unwrap().read(1024, false).unwrap();
                    let a = output.into_string();
                    let b = a.unwrap();
                    let c = b.as_bytes();

                    // if len(c) > 1
                    if c.len() > 0 {
                        handler_clone.network().send(endpoint, &c[1..]);
                    }
                }
            });

            println!("Client connected")
        },
        NetEvent::Message(endpoint, data) => {
            // convert data to string
            let str_data = std::str::from_utf8(&data).unwrap();
            let to_write = OsString::from(str_data);
            let num_bytes = pty.lock().unwrap().write(to_write).unwrap();

            handler.network().send(endpoint, data);
        },
        NetEvent::Disconnected(_endpoint) => mt_log!(Level::Info, "Disconnected"),
    });
}