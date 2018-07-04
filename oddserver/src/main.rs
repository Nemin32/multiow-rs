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

// We use the same PlayerInfo struct as in the client. See it's main.rs for details.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct PlayerInfo {
    name: String,
    saved_muds: u16,
    location: [u16; 3],
    position: [u16; 2]
}

// This function handles every client.
fn handle_client(mut stream: TcpStream, player_list: Arc<Mutex<HashMap<String, PlayerInfo>>>, have_to: Sender<()>) {
    use std::io::Read;
    
    // We read the data sent by the players into this buffer.
    let mut buf = vec![0; 512];
    
    loop {
        match stream.read(&mut buf) {
            Ok(_) => {
                // We turn the raw bytes sent by the player into a PlayerInfo struct using Serde.
                let plinfo: PlayerInfo = deserialize(&buf[..]).unwrap();
                
                // We lock the player_list mutex, letting us edit the HashMap containing all the players' data.
                let mut plist = player_list.lock().unwrap();
                
                // We read the previous entry's state.
                let (prev_muds, prev_loc, prev_pos) = match plist.get(&plinfo.name) {
                    Some(info) => (info.saved_muds, info.location, info.position),
                    None => (0, [0,0,0], [0,0])
                };
                
                // If anything changed, we update the values.
                if plinfo.position != prev_pos || plinfo.location != prev_loc || plinfo.saved_muds != prev_muds {
                    plist.insert(plinfo.name.clone(), plinfo);
                    
                    // And we notify the announcer thread, that it's time to send data to the players.
                    let _ = have_to.send(());
                }
            },
            
            Err(e) => {
                // If the client has disconnected, we should terminate the thread or else it just causes the program to hog up memory.
                if e.kind() == ErrorKind::ConnectionReset {
                    return;
                }
            }
        };
    }
}

// This function communicates with all the clients. It's job is to send every players' data to the clients.
fn announcer(streams: Arc<Mutex<Vec<TcpStream>>>, have_to: Receiver<()>, player_infos: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
    use std::io::Write;

    loop {
        // recv() will block until it receives a signal. Which means this loop will be on standby, until we request it to do it's job.
        // We do this with 'have_to.send(())'
        have_to.recv().unwrap();
        
        // We lock the Mutex containing all the TcpStreams, letting us reach it's contents.
        let streams_unlocked = &mut *streams.lock().unwrap();
    
        //println!("Writing to {} hosts.", streams_unlocked.len());
        
        // This counter will count how many TcpStreams there are.
        let mut counter = 0;
        
        // We turn the HashMap into raw bytes. This will be what we will send to each client.
        let bytes: Vec<u8> = serialize(&*player_infos.lock().unwrap()).unwrap();
        
        // We loop through every client.
        while counter < streams_unlocked.len() {
            match streams_unlocked[counter].write_all(bytes.as_slice()) {
                Ok(_) => {
                    // If we successfully sent data to this client then we're done with this client.
                    println!("Msg sent to: {}", streams_unlocked[counter].peer_addr().unwrap());
                },
                
                Err(e) => {
                    // But if we haven't, it's time to drop this client, or else we just hog up memory.
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

// This function serves as a kind of "console". We can use it to issue some commands to the server.
fn console(streams: Arc<Mutex<Vec<TcpStream>>>, have_to: Sender<()>, muds: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
    use std::io::{self, BufRead, Write};
    use std::process;
    
    let stdin = io::stdin();
    
    loop {
        // These four lines will prompt the player with "> " 
        // and then split their input at each space, trim the newlines and return the entire thing in a Vec.
        let mut line = String::new();
        print!("> ");
        let _ = io::stdout().flush();
        let _ = stdin.lock().read_line(&mut line);
        let split: Vec<&str> = line.split(" ").map(|x| x.trim()).collect();

        // We check which command has the player entered.
        match split[0] {
            "shutdown" | "quit" | "q" | "exit" => {process::exit(0);},
            "kick" => {if split.len() == 2 {println!("{} kicked.", split[1]);} else {println!("Usage: kick [name]");}},
            "announce" => {let _ = have_to.send(()); println!("Flag set. Sending data...");},
            "help" => {println!("shutdown/quit/exit - Shuts the server down.\r\nkick [name] - Kicks a player from the server.\r\nannounce - Manually set the 'send player data' flag.");},
            "" => {}, // If the player hasn't entered anything, we should just loop.
            _ => {println!("Unrecognized command. Use 'help' to see the list of commands.");}
        }
    }
}

fn main() -> std::io::Result<()> {
    // Binding to 0.0.0.0 means that the system will assign a free IP for us.
    let listener = TcpListener::bind("0.0.0.0:2536").unwrap();
    
    // This HashMap contains all the players' data. See PlayerInfo struct.
    let player_infos = HashMap::new();
    let infos_mutex = Arc::new(Mutex::new(player_infos));
    
    // This Vec holds all the active connections.
    let streams = Arc::new(Mutex::new(Vec::new()));
    
    // We use this channel to communicate with the 'announcer' thread.
    let (sender, have_to) = channel();
    
    // We move a reference of the streams Vec and the PlayerInfo Mutex into the newly started 'announcer' thread.
    let (sclone, mclone) = (streams.clone(), infos_mutex.clone());
    thread::spawn(move || {announcer(sclone, have_to, mclone)});
    
    // We print a welcoming message.
    println!("--- [NEMIN'S MultiOW SERVER] ---\r\nRunning on port {}.\r\nEnter 'help' for the list of commands.\r\nHave fun!", listener.local_addr().unwrap().port());
    
    // We do the same as what we did with the 'announcer' thread for the 'console' thread.
    let (sclone, htclone, mclone) = (streams.clone(), sender.clone(), infos_mutex.clone());
    thread::spawn(move || {console(sclone, htclone, mclone)});
    
    // We accept connections and process them serially.
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            println!("Accepted connection.");
            
            // We clone the stream and put it into the streams Vec.
            streams.lock().unwrap().push(stream.try_clone().expect("Stream could not have been cloned."));
            
            // We spawn a new thread, running the 'handle_client' function. See it's comments for details.
            let infos_mutex = infos_mutex.clone();
            let have_to_announce = sender.clone();
            thread::spawn(move || {handle_client(stream, infos_mutex, have_to_announce)});
        }
    }
    Ok(())
}