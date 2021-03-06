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

// PLAYERSTATES = We should send the PlayerInfo-s to the players.
// ANNOUNCEMENT = There is a message we want to send.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
enum MessageType {
  PLAYERSTATES,
  ANNOUNCEMENT(String)
}

// You might say "But Nemin... This is not how chapters follow each other!" and you'd be right.
// However, for sume unknown reason, this is how the levels are numbered.
// I was dumbfounded too, but I guess we just have to accept this...
const LEVEL_NAMES: &[&str] = &[
  "Necrum Mines",
  "Necrum",
  "Mudomo Vault",
  "Mudanchee Vault",
  "FeeCo Depot",
  "Slig Barracks",
  "Mudanchee ender",
  "Bonewerkz",
  "SoulStorm Brewery",
  "Soultorm Brewery ender", // This is named "Game Ender" in-game. Maybe I should use that name too?
  "Mudomo Vault ender",
  "FeeCo Depot ender",
  "Slig Barracks ender",
  "Bonewerkz ender"
];

const HELP_MSG: &'static str = "
Available commands:

shutdown/quit/exit - Shuts the server down.
kick [name] - Kicks a player from the server.
announce - Send a message to all players
states - Manually set the 'send player data' flag.
players - Shows statistics about each connected player.
ips - Shows the IP-s of the connected players.
clear - Clears the console. (Will probably not work using cmd.exe)
";

// This function handles every client.
fn handle_client(name: String, mut stream: TcpStream, streams: Arc<Mutex<HashMap<String, TcpStream>>>, player_list: Arc<Mutex<HashMap<String, PlayerInfo>>>, have_to: Sender<MessageType>) {
  use std::io::Read;

  // We read the data sent by the players into this buffer.
  let mut buf = vec![0; 512];
  
  stream.set_read_timeout(Some(time::Duration::from_millis(400))).unwrap();
  
  loop {
    match stream.read(&mut buf) {
      // Reading Ok(0) means that the connection is closed, so we should terminate the thread.
      Ok(0) => {
        println!("Player '{}' disconnected.", name); 
        player_list.lock().unwrap().remove(&name);
        streams.lock().unwrap().remove(&name);
        return;
      },
      Ok(_) => {
        // We turn the raw bytes sent by the player into a PlayerInfo struct using Serde.
        let plinfo: Result<PlayerInfo, _> = deserialize(&buf[..]);
        if let Ok(plinfo) = plinfo {
          // We lock the player_list mutex, letting us edit the HashMap containing all the players' data.
          let mut plist = player_list.lock().unwrap();

          // We swap the old PlayerInfo with the new one.
          let old_info = plist.insert(plinfo.name.clone(), plinfo.clone());
          if let Some(old) = old_info {
            // If anything changed...
            if old != plinfo {
              // If a player has entered a new chapter...
              if old.location[0] != plinfo.location[0] && plinfo.location[0] != 0 {
                // We send an announcement. "Player has entered chapter name!".
                let _ = have_to.send(MessageType::ANNOUNCEMENT(format!("{} has entered {}!", plinfo.name, LEVEL_NAMES[*&plinfo.location[0] as usize - 1])));
              }
            }
            
            // And we notify the announcer thread, that it's time to send data to the players.
            let _ = have_to.send(MessageType::PLAYERSTATES);
          }
        }
      },
      
      Err(e) => {
        // If the client has disconnected, we should terminate the thread or else it just causes the program to hog up memory.
        if e.kind() != ErrorKind::TimedOut {
          println!("Player '{}' disconnected.", name); 
          player_list.lock().unwrap().remove(&name);
          streams.lock().unwrap().remove(&name);
          return;
        }
      }
    };
  }
}

// This function provides a convenient way to try to send data to a client.
// If the client refuses, we simply drop his connection, because he is probably already disconnected.
fn write_or_drop(streams: &mut HashMap<String, TcpStream>, bytes: Vec<u8>) {
  use std::io::Write;
  use std::net::Shutdown::Both;

  streams.retain(
    |name, stream| {
      match stream.write_all(bytes.as_slice()) {
        Ok(_) => true,          
        Err(e) => {
          if e.kind() != ErrorKind::TimedOut {
            stream.shutdown(Both).unwrap();
            println!("Error. Dropping player {}.", name);
            return false;
          }
          true
        }
      }
    }
  );
}

// This function communicates with all the clients. It's job is to send every players' data to the clients.
fn announcer(streams: Arc<Mutex<HashMap<String, TcpStream>>>, have_to: Receiver<MessageType>, player_infos: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
  loop {
    // recv() will block until it receives a signal. Which means this loop will be on standby, until we request it to do it's job.
    // We do this with 'have_to.send(MessageType)'
    let msgtype = have_to.recv().unwrap();
    
    // We lock the Mutex containing all the TcpStreams, letting us reach it's contents.
    let streams_unlocked = &mut *streams.lock().unwrap();

    write_or_drop(streams_unlocked, serialize(&msgtype).unwrap());
    if msgtype == MessageType::PLAYERSTATES {
        // We turn the HashMap into raw bytes. This will be what we will send to each client.
        let bytes: Vec<u8> = serialize(&*player_infos.lock().unwrap()).unwrap();
        write_or_drop(streams_unlocked, bytes);
    }
  }
}

// This function serves as a kind of "console". We can use it to issue some commands to the server.
fn console(streams: Arc<Mutex<HashMap<String, TcpStream>>>, have_to: Sender<MessageType>, muds: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
  use std::io::{self, BufRead, Write};
  use std::process;
  use  std::net::Shutdown;
  
  let stdin = io::stdin();
  
  loop {
    // These four lines will prompt the player with "> " 
    // and then split their input at each space, trim the newlines and return the entire thing in a Vec.
    let mut line = String::new();
    print!("> ");
    let _ = io::stdout().flush(); // Rust is line-buffered. We need to flush STDOUT for the "> " to show.
    let _ = stdin.lock().read_line(&mut line);
    let split: Vec<&str> = line.split(" ").map(|x| x.trim()).collect();

    // We check which command has the player entered.
    match split[0] {
      "shutdown" | "quit" | "q" | "exit" => {process::exit(0);},
      "kick" => {
        if split.len() == 2 {
          if muds.lock().unwrap().contains_key(split[1]) {
            let mut line = String::new();
            
            print!("Are you sure you want to kick {}? (NO/yes) ", split[1]);
            let _ = io::stdout().flush();
            let _ = stdin.lock().read_line(&mut line);
            
            match line.as_str().trim() {
              "yes" | "y" | "Y" | "Yes" | "YES" => {
                //First we remove their PlayerInfo struct from the HashMap.
                muds.lock().unwrap().remove(split[1]);
                
                //Then we shut their connection off.
                //TODO: Send "You are disconnected" signal here?
                if let Some(shut) = streams.lock().unwrap().get(split[1]) {
                  shut.shutdown(Shutdown::Both).unwrap();
                }
                
                //Finally we remove the now defunct connection from the HashMap and notify the server's owner.
                streams.lock().unwrap().remove(split[1]);
                println!("{} kicked.", split[1]);
              },
              "" | "no" | "n" |"N" | "No" | "NO" => {},
              _ => println!("Please answer using (Y)es or (N)o.")
            };
          } else {
            println!("There is no such player.");
          }
        } else {
          println!("Usage: kick [name]");
        }
      },
      "states" => {let _ = have_to.send(MessageType::PLAYERSTATES); println!("Flag set. Sending data...");},
      "clear" | "c" => {
        //Sucks to be this guy ˘
        println!("If this doesn't clean your console then you're using an outdated console. Please use PowerShell.{}[2;J", 27 as char);
      }
      "announce" => {
        //'msg' will contain everything after the first space. So if the input was "announce this is a test message",
        //then msg == "this is a test message".
        let msg = line.clone().split_off(split[0].len() + 1);
        println!("{}", msg);
        let _ = have_to.send(MessageType::ANNOUNCEMENT(msg));
      },
      "players" => {
        let locked = &*muds.lock().unwrap();
        
        println!("There are {} connected players.{}", locked.len(), (if locked.len() != 0 {"\r\n---"} else {""}));
        
        for player in locked.values() {
          println!("[{}]:\nRescued Mudokons: {}\nLocation: {:?}\nPosition:\nX: {} Y: {}\n---", player.name, player.saved_muds, player.location, player.position[0], player.position[1]);
        }
      },
      "ips" => {
        let locked = &*streams.lock().unwrap();
        
        println!("There are {} connected players.\r\n", locked.len());
        for (name, stream) in locked.iter() {
          println!("[{}]: {}", name, stream.peer_addr().unwrap());
        }
      },
      "help" => {println!("{}", HELP_MSG);},
      "" => {}, // If the player hasn't entered anything, we should just loop.
      _ => {println!("Unrecognized command. Use 'help' to see the list of commands.");}
    }
  }
}

fn main() -> std::io::Result<()> {
  use std::io::Read;
  // Binding to 0.0.0.0 means that the system will assign a free IP for us.
  let listener = TcpListener::bind("0.0.0.0:2536").unwrap();
  
  // This HashMap contains all the players' data. See PlayerInfo struct.
  let player_infos = HashMap::new();
  let infos_mutex = Arc::new(Mutex::new(player_infos));
  
  // This HashMap holds all the active connections.
  let streams = Arc::new(Mutex::new(HashMap::new()));
  
  // We use this channel to communicate with the 'announcer' thread.
  let (sender, have_to) = channel();
  
  // We move a reference of the streams HashMap and the PlayerInfo Mutex into the newly started 'announcer' thread.
  let (sclone, mclone) = (streams.clone(), infos_mutex.clone());
  thread::spawn(move || {announcer(sclone, have_to, mclone)});
  
  // We print a welcoming message.
  println!("--- [NEMIN'S MultiOW SERVER] ---\r\nRunning on port {}.\r\nEnter 'help' for the list of commands.\r\nHave fun!", listener.local_addr().unwrap().port());
  
  // We do the same as what we did with the 'announcer' thread for the 'console' thread.
  let (sclone, htclone, mclone) = (streams.clone(), sender.clone(), infos_mutex.clone());
  thread::spawn(move || {console(sclone, htclone, mclone)});
  
  // We accept connections and process them serially.
  for stream in listener.incoming() {
    if let Ok(mut stream) = stream {
      let mut namebuf = [0; 256];
      stream.read(&mut namebuf).unwrap();
      let name: String = deserialize(&namebuf).unwrap();
      println!("Player '{}' joined the game.", name);
      
      // We clone the stream and put it into the streams HashMap.
      streams.lock().unwrap().insert(name.clone(), stream.try_clone().expect("Stream could not have been cloned."));
      
      // We spawn a new thread, running the 'handle_client' function. See it's comments for details.
      let infos_mutex = infos_mutex.clone();
      let have_to_announce = sender.clone();
      let streams = streams.clone();
      thread::spawn(move || {handle_client(name, stream, streams, infos_mutex, have_to_announce)});
    }
  }
  Ok(())
}