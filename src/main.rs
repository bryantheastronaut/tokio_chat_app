extern crate tokio;

use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

fn main() {
    let addr = "127.0.0.1:6142".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();
    let server = listener.incoming().for_each(|socket| {
        println!("Accepted socket. addr = {:?}", socket.peer_addr().unwrap());

        let connection = io::write_all(socket, "hello, world\n")
            .then(|res| {
                println!("Wrote message; success = {:?}", res.is_ok());
                Ok(())
            });
        tokio::spawn(connection);
        Ok(())
    })
    .map_err(|e| eprintln!("accept error = {:?}", e));
    println!("Server is running on localhost:6142");
    tokio::run(server);
}