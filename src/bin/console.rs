use std::net::TcpStream;
use albatrice::{Split, ToBinary, FromBinary, input};
fn main() {
    let (mut read, mut write) = TcpStream::connect("127.0.0.1:5287").unwrap().split().unwrap();
    std::thread::spawn(move || {
        while let Ok(_) = input().to_binary(&mut write) {}
    });
    while let Ok(string) = String::from_binary(&mut read) {
        println!("{string}");
    }
}
