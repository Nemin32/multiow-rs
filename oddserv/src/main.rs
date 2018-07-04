use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time;
use std::io::ErrorKind;

use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};

#[macro_use]
extern crate serde_derive;
extern crate bincode;
use bincode::{serialize, deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct PlayerInfo {
    name: String,
    saved_muds: u16,
    location: [u16; 3],
    position: [u16; 2]
}

fn handle_client(mut stream: TcpStream, player_list: Arc<Mutex<HashMap<String, PlayerInfo>>>, have_to: Sender<()>) {
    use std::io::Read;
    
    let mut buf = vec![0; 256];
    
    loop {
        match stream.read(&mut buf) {
            Ok(_) => {               
                let plinfo: PlayerInfo = deserialize(&buf[..]).unwrap();
                let mut plist = player_list.lock().unwrap();
                let (prev_muds, prev_loc, prev_pos) = match plist.get(&plinfo.name) {
                    Some(info) => (info.saved_muds, info.location, info.position),
                    None => (0, [0,0,0], [0,0])
                };
                
                if plinfo.position != prev_pos || plinfo.location != prev_loc || plinfo.saved_muds != prev_muds {
                    plist.insert(plinfo.name.clone(), plinfo);
                    
                    //*have_to.lock().unwrap() = true;
                    let _ = have_to.send(());
                }
            },
            
            Err(e) => {
                if e.kind() == ErrorKind::ConnectionReset {
                    return;
                }
            }
        };
    }
}

fn announcer(streams: Arc<Mutex<Vec<TcpStream>>>, have_to: Receiver<()>, muds: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
    use std::io::Write;

    loop {
        have_to.recv().unwrap();
        let streams_unlocked = &mut *streams.lock().unwrap();
    
        println!("Writing to {} hosts.", streams_unlocked.len());
        let mut counter = 0;
        
        let bytes: Vec<u8> = serialize(&*muds.lock().unwrap()).unwrap();
        
        while counter < streams_unlocked.len() {
            match streams_unlocked[counter].write_all(bytes.as_slice()) {
                Ok(_) => {
                    println!("Msg sent to: {}", streams_unlocked[counter].peer_addr().unwrap());
                },
                
                Err(e) => {
                    if e.kind() != ErrorKind::TimedOut {
                        println!("Dropping host: {}", streams_unlocked[counter].local_addr().unwrap());
                        streams_unlocked.remove(counter);
                        continue;
                    }
                }
            }
        
            counter = counter + 1;
        }
        
        thread::sleep(time::Duration::from_millis(200));
    }
}

fn console(streams: Arc<Mutex<Vec<TcpStream>>>, have_to: Sender<()>, muds: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
    use std::io::{self, BufRead, Write};
    use std::process;
    
    
    let stdin = io::stdin();
    
    loop {
        let mut line = String::new();
        print!("> ");
        let _ = io::stdout().flush();
        let _ = stdin.lock().read_line(&mut line);
        let split: Vec<&str> = line.split(" ").map(|x| x.trim()).collect();

        match split[0] {
            "shutdown" | "quit" | "q" | "exit" => {process::exit(0);},
            "kick" => {if split.len() == 2 {println!("{} kicked.", split[1]);} else {println!("Usage: kick [name]");}},
            "announce" => {let _ = have_to.send(()); println!("Flag set. Sending data...");},
            "help" => {println!("shutdown/quit/exit - Shuts the server down.\r\nkick [name] - Kicks a player from the server.\r\nannounce - Manually set the 'send player data' flag.");},
            "" => {},
            _ => {println!("Unrecognized command. Use 'help' to see the list of commands.");}
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2536").unwrap();
    let rescued_muds = HashMap::new();
    
    let mud_mutex = Arc::new(Mutex::new(rescued_muds));
    let streams = Arc::new(Mutex::new(Vec::new()));
    
    let (sender, have_to) = channel();
    
    let (sclone, mclone) = (streams.clone(), mud_mutex.clone());
    thread::spawn(move || {announcer(sclone, have_to, mclone)});
    
    println!("--- [NEMIN'S MultiOW SERVER] ---\r\nRunning on port {}.\r\nEnter 'help' for the list of commands.\r\nHave fun!", listener.local_addr().unwrap().port());
    
    let (sclone, htclone, mclone) = (streams.clone(), sender.clone(), mud_mutex.clone());
    thread::spawn(move || {console(sclone, htclone, mclone)});
    
    // accept connections and process them serially
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            println!("Accepted connection.");
            
            streams.lock().unwrap().push(stream.try_clone().expect("Oof"));
            
            let mud_mutex = mud_mutex.clone();
            let have_to_announce = sender.clone();
            thread::spawn(move || {handle_client(stream, mud_mutex, have_to_announce)});
        }
    }
    Ok(())
}