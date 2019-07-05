use super::*;
use std::io::prelude::*;
use oh_my_rust::*;
use std::time::Duration;

type Callback = Box<dyn FnMut() -> bool + Send>;

/// A piper starts a background thread that do the piping works via mio
/// note: currently we only asyncly listen on read, while assuming write should not block. This is untrue.
struct Piper {
    poll: mio::Poll,
    callback_table: std::sync::Mutex<CallbackTable>
}

// todo: directly save the pointer to the function inside Token? thus we can avoid the dict
// note: dyn pointers are not usize: they are two usizes. Thus we cannot just point to the closure, instead we need to point to a box which contains the fat pointer. This is two indirections, but still better than hashmap looking
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
                // long lock: get_mut hold a reference to the table, this is because modifying the table may reallocate
                // long lock here is fine because adding callbacks do not actually change anything before the next poll
                // to change to short lock, we need to directly refer to the content of box rather than refer to the box which holds the whole hashmap
                let table = &mut piper_copy.callback_table.lock().unwrap().table;
                for event in &events {
                    let f = table.get_mut(&event.token()).unwrap();
                    if f() { // finished
                        table.remove(&event.token());
                    }
                }
            }
        });

        piper
    }

    pub fn register<R, W, P, E>(&self, mut from: R, mut to: W, pipe: P, end: E)
        where R: ReadExt + mio::Evented + Send + 'static,
              W: Write + Send + 'static,
              P: Fn(&[u8], &mut W) + Send + 'static,
              E: Fn(&mut W) + Send + 'static
    {
        let handle = &from as *const R; // save the pointer before we transfer the ownership to the closure
        let token = self.callback_table.lock().unwrap().insert(Box::new(move || {
            let mut buf = [0; 1024];
            loop {
                match from.read(&mut buf) {
                    Ok(0) => { // closed
                        end(&mut to);
                        return true
                    },
                    Ok(n) => {
                        pipe(&buf[..n], &mut to);
                        return false
                    },
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            return false // run out of bytes, waiting for next event.
                        }
                        panic!("err={:?}", e)
                    }
                }
            }
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