use std::net::{TcpListener, TcpStream};
use std::io::{Write, Error, self, BufRead, BufReader};
use std::thread;
use polling::{Event, Poller};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc::TryRecvError;

fn main() {
    let port = std::env::args().nth(1).unwrap_or("".to_string());
    let ip = std::env::args().nth(2).unwrap_or("".to_string());
    
    if ip.chars().count() > 0 && port.chars().count() > 0 {
        client(port, ip);
    } else if port.chars().count() > 0 {
        server(port)  
    } else {
        println!("No port given!");
    }

}

fn server(port: String){
    let listener = TcpListener::bind(format!("0.0.0.0:{}",port)).expect("could not bind");
    for stream in listener.incoming() {
        match stream {
            Err(e) => { eprintln!("failed: {}", e) },
            Ok(stream) => {
                thread::spawn(move || {
                    accept(stream).unwrap_or_else(|error| eprintln!("{:?}", error))
                });
            },
        };
    }

}

fn accept(mut stream: TcpStream) -> Result<(), Error> {
    println!("Incoming connection from: {}", stream.peer_addr()?);
    let mut input = String::new();
    print!("Accept incoming connection (y/n)? ");
    io::stdout().flush().expect("flush failed!");

    io::stdin().read_line(&mut input).expect("could not read input");
    stream.write(input.as_bytes()).expect("Failed to write to client");

    if input.chars().nth(0).unwrap() == 'y' || input.chars().nth(0).unwrap() == 'Y' {
        talk(&mut stream).unwrap_or_else(|error| eprintln!("{:?}", error));
    } else {
        return Ok(());
    }
    return Ok(());
}

fn client(port: String, ip: String){
    let mut stream = TcpStream::connect(format!("{}:{}", ip, port)).expect("could not connect to host");
    println!("Connecting to {ip}:{port} please wait...");
    loop {
        let mut buffer: Vec<u8> = Vec::new();
        let mut response = BufReader::new(&stream);
        response.read_until(b'\n', &mut buffer).expect("could not read into buffer");

        if buffer[0] == b'y' || buffer[0] == b'Y' {
            println!("Connection accepted");
            talk(&mut stream).unwrap_or_else(|error| eprintln!("{:?}", error));
            return
        } else {
            println!("Connection rejected... exiting program");
            std::process::exit(0x1);
        }
    }
}

fn talk(stream: &mut TcpStream) -> Result<(), Error>{
    let poller = Poller::new()?;
    let key = 7;
    stream.set_nonblocking(true)?;
    poller.add(&*stream, Event::readable(key))?;
    let mut events = Vec::new();

    let (tx2, stdin_channel) = spawn_stdin_channel();

    println!("--------------------Transmission Started--------------------");
    loop {
        
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(10)))?;

        let mut buffer = String::new();
        let mut reader = BufReader::new(&*stream);

        for ev in &events {
            if ev.key == key{
                match reader.read_line(&mut buffer){
                    Ok(num_bytes) => {
                        if num_bytes == 0 {
                            println!("---------------------Transmission Ended---------------------");
                            tx2.send(String::new()).expect("Error could not disconnect from stdin");
                            stream.shutdown(std::net::Shutdown::Both).expect("Failed to shutdown stream");
                            return Ok(()); 
                        }
                    },
                    Err(_) => {
                        println!("---------------------Transmission Ended---------------------");
                        tx2.send(String::new()).expect("Error could not disconnect from stdin");
                        stream.shutdown(std::net::Shutdown::Both).expect("Failed to shutdown stream");
                        return Ok(());  
                    },
                };
                buffer.pop().unwrap();
                println!("{}", buffer);
                poller.modify(&*stream, Event::readable(key))?;
            }
        }

        match stdin_channel.try_recv() {
            Ok(key) => {
                stream.write(key.as_bytes()).expect("Failed to write to stream");
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => panic!("stdin channel disconnected"),
        }



    }
}

//figure out how to close thread when connection closed
fn spawn_stdin_channel() -> (Sender<String>, Receiver<String>) {
    let (tx, rx) = mpsc::channel::<String>();
    let (tx2, rx2) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();

        match rx2.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break;
            },
            Err(TryRecvError::Empty) => {},
        };
    }); 
    return (tx2, rx);
}
