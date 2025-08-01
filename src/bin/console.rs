use albatrice::{FromBinary, Split, ToBinary, input};
use std::net::TcpStream;
fn main() {
    let (mut read, mut write) = TcpStream::connect("127.0.0.1:5287")
        .unwrap()
        .split()
        .unwrap();
    println!("Connection made");
    std::thread::spawn(move || while input().to_binary(&mut write).is_ok() {});
    while let Ok(string) = String::from_binary(&mut read) {
        println!("{string}");
    }
}
