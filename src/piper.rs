use super::*;
use std::io::prelude::*;
use oh_my_rust::*;
use std::time::Duration;

struct Task {
    from: Box<dyn ReadExt + Send>,
    to: Box<dyn Write + Send>,
    pipe: fn(&mut dyn ReadExt, &mut dyn Write),
    end: fn(&mut dyn ReadExt, &mut dyn Write)
}

/// A piper starts a background thread that do the piping work via mio
#[allow(deprecated)]
struct Piper {
    tx: mio::channel::Sender<Task>
}

impl Piper {
    fn new() -> Piper {
        #[allow(deprecated)]
        let (tx, rx) = mio::channel::channel();

        std::thread::spawn(move || {
            let poll = match mio::Poll::new() {
                Ok(poll) => poll,
                Err(e) => panic!("failed to create Poll instance; err={:?}", e),
            };
            
            let mut events = mio::Events::with_capacity(1024);

            let token_table = std::collections::BTreeMap::<mio::Token, Task>::new();

            poll.register(&rx, mio::Token(0), mio::Ready::readable(), mio::PollOpt::edge());

            let mut token_id = 1; // the zero-th token is for the channel receiver

            loop {
                poll.poll(&mut events, None).unwrap();
                for event in &events {
                    match event.token() {
                        mio::Token(0) => unimplemented!(),
                        token => unimplemented!()
                    }
                }
            }
        });

        Piper { tx }
    }
}
