#[cfg(windows)] extern crate winapi;
use std::net::TcpStream;
use std::{thread, time};
use std::io::Write;
use std::io::Read;
use std::process;
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

#[macro_use]
extern crate serde_derive;
extern crate bincode;
use bincode::{serialize, deserialize};

//375 x 260
const ROOM_WIDTH: u16 = 375;
const ROOM_HEIGHT: u16 = 260;

//640 x 480
const PROPORTION_W: f64 = 800.0 / ROOM_WIDTH as f64;
const PROPORTION_H: f64 = 640.0 / ROOM_HEIGHT as f64;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct PlayerInfo {
    name: String,
    saved_muds: u16,
    location: [u16; 3],
    position: [u16; 2]
}

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
                ((relativexy[0] as i16 - 4) as f64 * PROPORTION_W) as i32,
                ((relativexy[1] as i16 - 32) as f64 * PROPORTION_H) as i32,
                ((relativexy[0] + 14) as f64 * PROPORTION_W) as i32,
                ((relativexy[1] - 16) as f64 * PROPORTION_H) as i32);
            };
        //}
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
        
        let mut prevhero: [u16; 2] = [0; 2];
        
        let name = read_line("Enter your name: ");
        let mut connection = make_connection();

        let (sender, receiver) = channel();
        
        thread::spawn(move || {render_players( receiver)});
        
        use winapi::um::psapi::MODULEINFO;
        use winapi::um::psapi::GetModuleInformation;
        
        let mut module: HMODULE = null_mut();
        winapi::um::psapi::EnumProcessModules(handle, &mut module, 8, null_mut());
        
        let mut mi: MODULEINFO = MODULEINFO {lpBaseOfDll: null_mut(), SizeOfImage: 0, EntryPoint: null_mut()};
        GetModuleInformation(handle, module, &mut mi, 24);

        let base_pointer = (mi.lpBaseOfDll as u32 + 0x1C1230) as *mut u8;

        let pointer: u32 = 0;
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
            
            ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
            ReadProcessMemory(handle, (pointer + 0xBA) as LPCVOID, xp as LPVOID, size_of::<u16>(), null_mut());
            
            ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
            ReadProcessMemory(handle, (pointer + 0xBE) as LPCVOID, yp as LPVOID, size_of::<u16>(), null_mut());

            let relativexy: [u16; 2] = [xpos % ROOM_WIDTH, ypos % ROOM_HEIGHT];      

            if old != read || prevpos != pos || prevhero != relativexy {
                prevpos = pos;
                old = read;
                prevhero = relativexy;
                
                
                
                if pos[0] != 0 || relativexy != [0,0] { //Title screen check
                    let payload = PlayerInfo {name: name.clone(), saved_muds: read, location: pos, position: relativexy};
                    let bytes: Vec<u8> = serialize(&payload).unwrap();
                    connection.write_all(bytes.as_slice()).unwrap();
                }
            }

            
            let mut buffer = vec![0;512];
            match connection.read(&mut buffer) {
                Ok(_) => {
                    let msg: Vec<PlayerInfo> = deserialize(&buffer[..]).unwrap();
                    
                    for player in msg {
                        //println!("{:?}", player);
                        sender.send(player.position);
                    }
                    
                    println!("");
                },
                _ => {}
            };
            
            //thread::sleep(time::Duration::from_millis(500));
        }
        
    }
    
}
