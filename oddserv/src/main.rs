use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time;
use std::io::ErrorKind;

struct PlayerInfo {
    savedMuds: usize,
    location: [u16; 2]
}

fn handle_client(mut stream: TcpStream, rescued_muds: Arc<Mutex<HashMap<String, PlayerInfo>>>, have_to: Arc<Mutex<bool>>) {
    use std::io::Read;
    use std::str;
    
    let mut buf = vec![0; 20];
    let mut muds: usize = 1;

    //stream.set_read_timeout(Some(time::Duration::from_millis(100))).unwrap();
    
    loop {
        //buf.clear();
        match stream.read(&mut buf) {
            Ok(_) => {
                let msg = String::from_utf8(buf.clone()).unwrap();
                let split: Vec<&str> = msg.split("|").collect();

                let num = split[1].trim_matches(char::from(0)).parse::<usize>().unwrap();
                let pos0 = split[2].trim_matches(char::from(0)).parse::<u16>().unwrap();
                let pos1 = split[3].trim_matches(char::from(0)).parse::<u16>().unwrap();
                
                if num != muds {
                    rescued_muds.lock().unwrap().insert(split[0].to_string(), PlayerInfo {savedMuds: num, location: [pos0, pos1]});
                    muds = num;
                    
                    *have_to.lock().unwrap() = true;
                    
                    println!("[{}]: {}", split[0], num);
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
            
            while counter < streams_unlocked.len() {
                let mut payload = String::new();
                for (key, val) in &*muds.lock().unwrap() {
                    payload += &format!("{}: [{}/{}] ({}), ", key, val.location[0], val.location[1], val.savedMuds);
                }
                    
                match streams_unlocked[counter].write(payload.as_bytes()) {
                    Ok(_) => {
                        println!("Msg sent to: {}", streams_unlocked[counter].local_addr().unwrap());
                    },
                    Err(e) => {
                        if e.kind() == ErrorKind::ConnectionReset {
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
    
        thread::sleep(time::Duration::from_millis(1000));
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