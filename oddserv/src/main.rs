use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time;
use std::io::ErrorKind;

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

fn handle_client(mut stream: TcpStream, player_list: Arc<Mutex<HashMap<String, PlayerInfo>>>, have_to: Arc<Mutex<bool>>) {
    use std::io::Read;
    //use std::str;
    
    let mut buf = vec![0; 256];

    //stream.set_read_timeout(Some(time::Duration::from_millis(100))).unwrap();
    
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
                    *have_to.lock().unwrap() = true;
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

fn announcer(streams: Arc<Mutex<Vec<TcpStream>>>, have_to: Arc<Mutex<bool>>, muds: Arc<Mutex<HashMap<String, PlayerInfo>>>) {
    use std::io::Write;

    loop {
        if *have_to.lock().unwrap() {
            let streams_unlocked = &mut *streams.lock().unwrap();
        
            println!("Writing to {} hosts.", streams_unlocked.len());
            let mut counter = 0;
            
            let mut payload = Vec::new();
            
            for (_, vals) in &*muds.lock().unwrap() {
                payload.push(vals.clone());
            }
            
            println!("{:?}", payload);
            
            let bytes: Vec<u8> = serialize(&payload).unwrap();
            
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
            
            println!("Lock disengaged.");
            *have_to.lock().unwrap() = false;
        }
    
        thread::sleep(time::Duration::from_millis(200));
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2536").unwrap();
    let rescued_muds = HashMap::new();
    
    let mud_mutex = Arc::new(Mutex::new(rescued_muds));
    let streams = Arc::new(Mutex::new(Vec::new()));
    let have_to_announce = Arc::new(Mutex::new(true));
    
    let (sclone, htclone, mclone) = (streams.clone(), have_to_announce.clone(), mud_mutex.clone());
    thread::spawn(move || {announcer(sclone, htclone, mclone)});
    
    // accept connections and process them serially
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            println!("Accepted connection.");
            
            streams.lock().unwrap().push(stream.try_clone().expect("Oof"));
            
            let mud_mutex = mud_mutex.clone();
            let have_to_announce = have_to_announce.clone();
            thread::spawn(move || {handle_client(stream, mud_mutex, have_to_announce)});
        }
    }
    Ok(())
}