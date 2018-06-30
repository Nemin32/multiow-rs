#[cfg(windows)] extern crate winapi;
use std::net::TcpStream;
use std::io::ErrorKind;

fn into_os(msg: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(msg).encode_wide().chain(once(0)).collect()
}

fn main() {
    use winapi::shared::minwindef::*;
    use winapi::um::winuser::FindWindowW;
    use winapi::um::winuser::GetWindowThreadProcessId;
    use winapi::um::winnt::*;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::memoryapi::ReadProcessMemory;
    
    use std::mem::size_of;
    use std::{thread, time};
    use std::ptr::null_mut;
    
    use std::io::Write;
    use std::io::Read;
    
    let mut name: String = String::new();
    std::io::stdin().read_line(&mut name).unwrap();
    name.pop();
    name.pop();
    
    let mut connection = TcpStream::connect("84.3.238.244:2536").unwrap();
    connection.set_read_timeout(Some(time::Duration::from_millis(100))).unwrap();

    unsafe {
        let oddapp = FindWindowW(null_mut(), into_os("Oddworld Abe's Exoddus").as_ptr());
        let mut proc: DWORD = 0;
        let access: DWORD = PROCESS_VM_READ | PROCESS_QUERY_INFORMATION;

        GetWindowThreadProcessId(oddapp, &mut proc);
        
        let handle = OpenProcess(access, 0, proc);
        let mut read: u16 = 0;
        let mut old: u16 = 1;
        let readp: *mut u16 = &mut read;
        
        let mut pos: [u16; 2] = [0; 2];
        let mut prevpos: [u16; 2] = [0; 2];
        let posp: *mut [u16; 2] = &mut pos;
        
        loop {
            ReadProcessMemory(handle, 0x5C1BC2 as LPCVOID, readp as LPVOID, size_of::<u16>(), null_mut());
            ReadProcessMemory(handle, 0x5C3030 as LPCVOID, posp as LPVOID, size_of::<u16>() * 2, null_mut());
            
            if old != read || pos[0] != prevpos[0] || pos[1] != prevpos[1] {
                println!("{:?}", pos);
                
                prevpos[0] = pos[0];
                prevpos[1] = pos[1];
                old = read;
                
                let payload = format!("{}|{}|{}|{}", name, read, pos[0],pos[1]);
                println!("Currently rescued {} Mudokons. Location: {:?} (name: '{}')", read, pos, name);
                connection.write_all(payload.as_bytes()).unwrap();
            }

            let mut buffer = vec![0;256];
            match connection.read(&mut buffer) {
                Ok(_) => {
                    let msg = String::from_utf8(buffer).unwrap();
                    let msg = msg.trim_matches(char::from(0));
                    //let mut msg_rp = str::replace(&msg, "{", "");
                    //msg_rp.pop();
                    
                    let msg_rp = msg.split(",");
                    //msg.pop();
                    //msg.pop();
                    
                    
                    for line in msg_rp {
                        println!("{}", str::replace(line, "\"", ""));
                    }
                    
                    println!("");
                },
                _ => {}
            };
            
            thread::sleep(time::Duration::from_millis(1000));
        }
    }
}