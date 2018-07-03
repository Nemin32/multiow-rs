#[cfg(windows)] extern crate winapi;
use std::net::TcpStream;
use std::{thread, time};
use std::io::Write;
use std::io::Read;
use std::process;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;

use winapi::shared::minwindef::*;
use winapi::um::winuser::FindWindowW;
use winapi::um::winuser::GetWindowThreadProcessId;
use winapi::um::winnt::*;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::memoryapi::ReadProcessMemory;

use std::mem::size_of;
use std::ptr::null_mut;

fn into_os(msg: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(msg).encode_wide().chain(once(0)).collect()
}

fn read_line(prompt: &str) -> String {
    let mut retval: String = String::new();
    println!("{} ", prompt);
    std::io::stdin().read_line(&mut retval).unwrap();
    
    retval.pop();
    retval.pop();
    
    retval
}

fn make_connection() -> TcpStream {
    let ip = read_line("Enter the server's IP: ");
    
    println!("Connecting to {}:2536", ip);
    let connection = match TcpStream::connect(format!("{}:2536", ip)) {
        Ok(s) => {println!("Connection successful.\n\n"); s},
        Err(e) => {println!("Connection error: {:?}", e.kind()); process::exit(1);}
    };
    connection.set_read_timeout(Some(time::Duration::from_millis(100))).unwrap();
    
    connection
}

//375 x 260
const ROOM_WIDTH: u16 = 375;
const ROOM_HEIGHT: u16 = 260;

//640 x 480
const PROPORTION_W: f64 = 640.0 / ROOM_WIDTH as f64;
const PROPORTION_H: f64 = 480.0 / ROOM_HEIGHT as f64;

fn render_players(relativexy: Receiver<[u16; 2]>) {
    unsafe {
    let oddapp = FindWindowW(null_mut(), into_os("Oddworld Abe's Exoddus").as_ptr());
    let hdc = winapi::um::winuser::GetDC(oddapp);
    
    let brush = winapi::um::wingdi::CreateSolidBrush(winapi::um::wingdi::RGB(255, 0, 0));
    winapi::um::wingdi::SelectObject(hdc, brush as LPVOID);
    
    loop {
        //for relativexy in poslist.recv().unwrap() {
            let relativexy = relativexy.recv().unwrap();
            winapi::um::wingdi::Rectangle(
                hdc,
                ((relativexy[0] as i16 - 5) as f64 * PROPORTION_W) as i32,
                ((relativexy[1] as i16 - 20) as f64 * PROPORTION_H) as i32,
                ((relativexy[0] + 15) as f64 * PROPORTION_W) as i32,
                ((relativexy[1]) as f64 * PROPORTION_H) as i32);
            };
        //}
        
        //thread::sleep(time::Duration::from_millis(60));
    }
}

fn main() {

    
    unsafe {
        let oddapp = FindWindowW(null_mut(), into_os("Oddworld Abe's Exoddus").as_ptr());
        let mut proc: DWORD = 0;
        let access: DWORD = PROCESS_VM_READ | PROCESS_QUERY_INFORMATION;

        if oddapp == null_mut() /*false*/ { //Remove comment to turn on checking.
            println!("You need to run Exoddus.exe before starting this program.");
            process::exit(1);
        } 
        
        GetWindowThreadProcessId(oddapp, &mut proc);
        
        let handle = OpenProcess(access, 0, proc);
        let mut read: u16 = 0;
        let mut old: u16 = 1;
        let readp: *mut u16 = &mut read;
        
        let mut pos: [u16; 3] = [0; 3];
        let mut prevpos: [u16; 3] = [0; 3];
        //let posp: *mut [u16; 3] = &mut pos;
        
        let mut heroxy: [u16; 3] = [0; 3];
        let mut prevhero: [u16; 3] = [0; 3];
        //let heroxyp: *mut [u16; 3] = &mut heroxy;
        
        //let name = read_line("Enter your name: ");
        //let mut connection = make_connection();

        //let (sender, receiver) = channel();
        
        //thread::spawn(move || {render_players( receiver)});
        
        use winapi::um::psapi::MODULEINFO;
        use winapi::um::psapi::GetModuleInformation;
        
        let mut module: HMODULE = null_mut();
        winapi::um::psapi::EnumProcessModules(handle, &mut module, 8, null_mut());
        
        let mut mi: MODULEINFO = MODULEINFO {lpBaseOfDll: null_mut(), SizeOfImage: 0, EntryPoint: null_mut()};
        GetModuleInformation(handle, module, &mut mi, 24);

        let base_pointer = (mi.lpBaseOfDll as u32 + 0x1C1230) as *mut u8;

        let mut pointer: u32 = 0;
        let pp: *const u32 = &pointer;
        
        ReadProcessMemory(handle, base_pointer as LPVOID, pp as LPVOID, size_of::<u32>(), null_mut());

        let pos_base = (pointer + 0x68) as *mut u32;

        let xpos: u16 = 0;
        let xp: *const u16 = &xpos;
        ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
        ReadProcessMemory(handle, (pointer + 0xBA) as LPCVOID, xp as LPVOID, size_of::<u16>(), null_mut());
        println!("{:?}", xpos);
        
        let ypos: u16 = 0;
        let yp: *const u16 = &ypos;
        ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
        ReadProcessMemory(handle, (pointer + 0xBE) as LPCVOID, yp as LPVOID, size_of::<u16>(), null_mut());
        println!("{:?}", ypos);
        
        
        loop {
            //0x5C1BC2 DWORD number of rescued Mudokons
            //0x5C3030 - 0x5C3034 3xDWORD = LVL ID, PATH ID, CAM ID

            ReadProcessMemory(handle, 0x5C1BC2 as LPCVOID, readp as LPVOID, size_of::<u16>(), null_mut()); 
            ReadProcessMemory(handle, 0x5C3030 as LPCVOID, pos.as_mut_ptr() as LPVOID, size_of::<u16>() * 3, null_mut());
            
            
            ReadProcessMemory(handle, (0x5C1B68 + 0xB8) as LPCVOID, heroxy.as_mut_ptr() as LPVOID, size_of::<u16>() * 3, null_mut());

            let relativexy: [u16; 2] = [heroxy[0] % ROOM_WIDTH, heroxy[2] % ROOM_HEIGHT];      
            sender.send(relativexy);
            
            println!("{:?}", heroxy);
            
            if old != read || prevpos != pos || prevhero != heroxy {
                prevpos = pos;
                old = read;
                prevhero = heroxy;
                
                
                
               // println!("{:?} | {:?}", heroxy, relativexy);
                /*let payload = format!("{}|{}|{}|{}|{}|{}|{}", name, read, pos[0],pos[1],pos[2], relativexy[0],relativexy[1]);
                println!("Currently rescued {} Mudokons. Location: {:?} Position: {:?} (name: '{}')", read, pos, relativexy, name);
                connection.write_all(payload.as_bytes()).unwrap();*/
            }

            
            let mut buffer = vec![0;256];
            match connection.read(&mut buffer) {
                Ok(_) => {
                    let msg = String::from_utf8(buffer).unwrap();
                    let msg = msg.trim_matches(char::from(0));
                    let msg_rp = msg.split(", ");

                    for line in msg_rp {
                        println!("{}", line);
                        
                        let line_split: Vec<&str> = line.split("|").collect();
                        if line_split.len() > 1 {
                            sender.send([
                                line_split[5].trim_matches(char::from(0)).parse::<u16>().unwrap(),
                                line_split[6].trim_matches(char::from(0)).parse::<u16>().unwrap()
                            ]).unwrap();
                        }
                    }
                    
                    println!("");
                },
                _ => {}
            };
            
            thread::sleep(time::Duration::from_millis(500));
        }
        
    }
    
}
