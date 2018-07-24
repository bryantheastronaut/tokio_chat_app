extern crate tokio;
#[macro_use]
extern crate futures;
extern crate bytes;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use futures::sync::mpsc;
use futures::future::{self, Either};
use bytes::{BytesMut, Bytes, BufMut};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Shorthand for the transmit half of the message channel
type Tx = mpsc::UnboundedSender<Bytes>;

/// Shorthand for the receive half of the message channel
type Rx = mpsc::UnboundedReceiver<Bytes>;

struct Shared {
    peers: HashMap<SocketAddr, Tx>,
}

impl Shared {
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }
}

struct Lines {
    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}

impl Lines {
    fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }
    fn fill_read_buf(&mut self) -> Result<Async<()>, io::Error> {
        loop {
            // ensure read buffer has capacity
            self.rd.reserve(1024);
            // read data into buffer
            // read_buf provided by AsyncRead
            let n = try_ready!(self.socket.read_buf(&mut self.rd));
            if n == 0 {
                return Ok(Async::Ready(()))
            }
        }
    }
    // write half
    fn buffer(&mut self, line: &[u8]) {
        // push the line onto the end of hte write buffer
        // the put fn is from BufMut trait
        self.wr.put(line);
    }

    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // as long as there is buffered data to write, try to write it
        while !self.wr.is_empty() {
            // try to write some bytes to the socket
            let n = try_ready!(self.socket.poll_write(&self.wr));
            // as long as wr is not empty, n should never be 0 bytes
            assert!(n > 0);
            // this discards the first n bytes from the buffer
            let _ = self.wr.split_to(n);
        }
        Ok(Async::Ready(()))
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        // first read any new data that may have come off the socket
        // track if the socket is closed here and use that to inform the
        // return value below
        let sock_closed = self.fill_read_buf()?.is_ready();
        // now try finding lines
        let pos = self.rd.windows(2)
            .position(|bytes| bytes == b"\r\n");
        if let Some(pos) = pos {
            // remove from read buffer and set to "line"
            let mut line = self.rd.split_to(pos + 2);
            // remove the trailing \r\n
            line.split_off(pos);
            // return the line
            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {
    // wrap the socket with the Lines Codec we wrote above
    let lines = Lines::new(socket);
    // the first line is treated as the clients name. The client is
    // not added to the set of connected peers until this line is
    // recieved.
    // We use the `into_future` combinator to extract the first item
    // from the lines stream. `into_future` takes a stream and converts it
    // into a future of `(first, rest)` where rest is the original stream instance
    let connection = lines.into_future()
        // `into_future` doesnt have the right error type, but we can map it to make it work
        .map_err(|(e, _)| e)
        // process the first line recieved as the clients name
        .and_then(|(name, lines)| {
            let name = match name {
                Some(name) => name,
                None => {
                    // TODO: handle a client that disconnects early
                    unimplemented!();
                }
            }
            // TODO: rest of the process function
        });
    let task = unimplemented!();
    // spawn the task
    tokio::spawn(task);
}

fn main() {
    // create state instance which will be moved into the tasks that
    // accepts incoming connections
    let state = Arc::new(Mutex::new(Shared::new()));
    let addr = "127.0.0.1:6142".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();
    let server = listener.incoming().for_each(move |socket| {
        process(socket, state.clone());
        Ok(())
    })
    .map_err(|e| eprintln!("accept error = {:?}", e));

    println!("server running on localhost:6142");

    // Start the server which:
    // 1. starts the Tokio runtime
    // 2. spawns server on that runtime
    // 3. blocks current thread until runtime becomes idle
    //      (all spawns tasks have completed)
    tokio::run(server);
}