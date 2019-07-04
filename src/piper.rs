use super::*;
use std::io::prelude::*;
use oh_my_rust::*;
use std::time::Duration;

type Callback = Box<dyn FnMut() + Send>;

/// A piper starts a background thread that do the piping work via mio
struct Piper {
    poll: mio::Poll,
    callback_table: std::sync::Mutex<CallbackTable>
}

struct CallbackTable {
    table: std::collections::BTreeMap<mio::Token, Callback>,
    id: usize
}

impl Piper {
    // create and leak a Piper
    pub fn new() -> &'static Piper {
        let piper = Piper {
            poll: match mio::Poll::new() {
                Ok(poll) => poll,
                Err(e) => panic!("failed to create Poll instance; err={:?}", e),
            },
            callback_table: std::sync::Mutex::new(CallbackTable {
                table: std::collections::BTreeMap::new(),
                id: 0
            }),
        };

        let piper = Box::leak(Box::new(piper));

        let piper_copy = &*piper;
        std::thread::spawn(move || {
            let mut events = mio::Events::with_capacity(1024);

            loop {
                piper_copy.poll.poll(&mut events, None).unwrap();
                for event in &events {
                    let table = &mut piper_copy.callback_table.lock().unwrap().table;
                    let f = table.get_mut(&event.token()).unwrap();
                    f()
                }
            }
        });

        piper
    }

    pub fn register<R, W>(&self, mut from: R, mut to: W, pipe: impl Fn(&[u8], &mut W), end: impl Fn(&mut W))
        where R: ReadExt + mio::Evented + Send + 'static,
              W: Write + Send + 'static
    {
        let handle = &from as *const R;
        let token = self.callback_table.lock().unwrap().insert(Box::new(move || {
            std::io::copy(&mut from, &mut to);
        }));
        // unsafe: we are sure that the function won't be used before we register `from` to the poll
        self.poll.register(unsafe { &*handle }, token, mio::Ready::readable(), mio::PollOpt::edge()).unwrap();
    }
}

impl CallbackTable {
    fn insert(&mut self, f: Callback) -> mio::Token {
        let token = mio::Token(self.id);
        self.id += 1;
        self.table.insert(token, f);
        token
    }
}