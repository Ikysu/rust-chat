use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, Error};
use std::env;
use tokio::sync::mpsc::{Sender, Receiver};

struct Client {
    addr: String,
    sock: TcpStream
}

async fn handle_client(tx: Sender<String>, stream: &mut TcpStream)-> Result<(), Error> {
    let commands = vec![
        "/help - Помощь",
        "/name [nickname] - Сменить ник",
        "/whoami - Кто я?"
    ];

    println!("incoming connection from: {}", stream.peer_addr()?);
    let mut name = stream.peer_addr()?.to_string();
    let mut buf = [0;512];

    tx.send(format!("Server: {} connected to chat!^", name)).await.unwrap();

    loop {
        let bytes_read = stream.read(&mut buf)?;
        if bytes_read == 0 {return Ok(())}
        let tmp = format!("{}", String::from_utf8_lossy(&buf).trim()).replace("\0", "").replace("^", "#");
        buf.iter_mut().for_each(|x| *x = 0);
        if tmp.len()>0 && tmp.len() < 100 {
            if tmp.chars().next() == Some('/') {
                let command_data = tmp[1..].split(" ").collect::<Vec<&str>>();
                match command_data[0] {
                    "help" => {
                        stream.write_all(format!("System: {}^",commands.join(" | ")).as_bytes())?;
                    },
                    "name" => {
                        if tmp.len()>1 && command_data.len()>1 {
                            let newname = command_data[1..].join(" ");
                            tx.send(format!("Server: {} changed to {}^", name, newname)).await.unwrap();
                            name = newname;
                        }
                    },
                    "whoami" => {
                        stream.write_all(format!("System: Your name is {}^", name).as_bytes())?;
                    },


                    _ => {
                        stream.write_all("System: Неизвестная команда^".as_bytes())?;
                    }
                }
            }else{
                tx.send(format!("{}: {}^", name,tmp)).await.unwrap();
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let listener = TcpListener::bind("0.0.0.0:26537").unwrap();
    listener.set_nonblocking(true).expect("Cannot set non-blocking");
    println!("Server listening");
    let mut clients: Vec<Client> = vec![];
    let (tx, mut rx): (Sender<String>, Receiver<String>) = tokio::sync::mpsc::channel(32);

    loop {
        match rx.try_recv() {
            Ok(raw_msg) => {
                for rawdt in raw_msg.split("^") {
                    if rawdt.len() > 0 {
                        for i in 0..clients.len() {
                            if let Some(elem) = clients.get_mut(i) {
                                match elem.sock.write_all(format!("{}^", rawdt).as_bytes()) {
                                    Ok(_)=>{
                                        println!("Sended ({}): {}", elem.addr, rawdt);
                                    },
                                    Err(_)=>{}
                                };
                            }
                        }
                    }
                    
                }
            },
            Err(_)=>{}
        }

        match listener.accept() {
            Ok((stream, address)) => {
                let st = stream.try_clone().expect("listener clone failed...");
                let newclient = Client {
                    addr:address.to_string(),
                    sock:st
                };
                clients.push(newclient);
                let mut stream_clone = stream.try_clone().expect("stream clone failed...");
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    match handle_client(tx_clone,&mut stream_clone).await {
                        Ok(_)=>{},
                        Err(_)=>{println!("Connection closed")}
                    }
                });
            },
            Err(_) => {},
        }
    };
    
    // close the socket server
    //drop(listener);
}
