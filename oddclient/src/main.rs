#[cfg(windows)] extern crate winapi;
use winapi::ctypes::c_void;
use winapi::shared::basetsd::LONG_PTR;
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
use std::collections::HashMap;

#[macro_use]
extern crate serde_derive;
extern crate bincode;
use bincode::{serialize, deserialize};

// Every screen is 375 x 260.
const ROOM_WIDTH: u16 = 375;
const ROOM_HEIGHT: u16 = 260;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct PlayerInfo {
  name: String,
  saved_muds: u16,
  location: [u16; 3], //LVL ID, Path ID, CAM ID
  position: [u16; 2] // X, Y
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
enum MessageType {
  PLAYERSTATES,
  ANNOUNCEMENT(String)
}

// I don't really know how this works. I copied this from here: https://github.com/retep998/winapi-rs
fn into_os(msg: &str) -> Vec<u16> {
  use std::ffi::OsStr;
  use std::iter::once;
  use std::os::windows::ffi::OsStrExt;

  OsStr::new(msg).encode_wide().chain(once(0)).collect()
}

// Prompts the user and returns their answer, stripped of newline
fn read_line(prompt: &str) -> String {
  let mut retval: String = String::new();
  println!("{} ", prompt);
  std::io::stdin().read_line(&mut retval).unwrap();
  
  retval.pop();
  retval.pop();
  
  retval
}

fn read_name() -> String {
  loop {
    let candidate = read_line("Enter your name:");
    
    // I have no problems with creative names, but imagine if a guy named "úőéúőéőé éőúéééúpúpéópü" comes up to your server. 
    // How would you do anything to them? Sure, you could probably copy-paste the name, but the enter is still there.
    // To circumvent this, the rules are the following:
    // - ASCII only
    // - No whitespace
    // - Longer than 2 characters, but shorter than 9.
    if candidate.chars().all(|c| {c.is_ascii() && !c.is_whitespace()}) && candidate.len() >= 3 && candidate.len() <= 8 {
      return candidate;
    } else {
      println!("The name you entered is invalid.\r\nPlease only use ASCII characters and no spaces.\r\nThe name's length should be between 3 and 8 characters.");
    }
  }
}

// Connects to the server. Currently the port is fixed, but this would be easy to fix.
// TODO: Make the server port agnostic.
fn make_connection(name: String) -> TcpStream {
  let ip = read_line("Enter the server's IP: ");
  
  /* Uncomment to prompt for port */
  let port_inp = ""; //read_line("Enter port (or press enter for default: 2536): "); 
  let port = match port_inp.as_ref() {
    "" => "2536",
    port => &port
  };
  
  println!("Connecting to {}:{}", ip, port);
  let connection = match TcpStream::connect(format!("{}:{}", ip, port)) {
    Ok(mut s) => {
      println!("Connection successful. Sending '{}' as name.", name); 
      let _ = s.write(&serialize(&name).unwrap());
      s
    },
    Err(e) => {println!("Connection error: {:?}", e.kind()); process::exit(1);}
  };
     
  connection.set_read_timeout(Some(time::Duration::from_millis(400))).unwrap();
  connection
}

macro_rules! into_proportion {
  ($val:expr, $offset:expr, $prop:ident) => {(($val as i16 + $offset) as f64 * $prop) as i32};
}
// Now this is where things get a bit more interesting. This function renders names in white boxes for each player.
fn render_players(player_data: Receiver<([u16; 2], String)>) {
  use winapi::shared::windef::*;
  use winapi::um::winuser::*;
  use winapi::um::wingdi::*;

  unsafe {
    let oddapp = FindWindowW(null_mut(), into_os("Test").as_ptr()); // HWND to the game's window.

    let mut winrect = RECT {left: 0, top: 0, right: 0, bottom: 0};
    let hdc = winapi::um::winuser::GetDC(oddapp);

    loop {
      // When the main thread sends us coordinates, we receive them into this array.
      
      
      // We query the window to get it's dimensions. Should be about 600x800, but it varies.
      GetWindowRect(oddapp, &mut winrect); 
      let proportion_w: f64 = (winrect.right - winrect.left) as f64 / ROOM_WIDTH as f64;
      let proportion_h: f64 = (winrect.bottom - winrect.top) as f64 / ROOM_HEIGHT as f64;
      
      for (relativexy, name) in player_data.try_iter() { 
        if relativexy[0] != 0 && relativexy[1] != 0 {
          let name = into_os(&name);
          
          TextOutW(
            hdc,
            into_proportion!(relativexy[0], -10, proportion_w), 
            into_proportion!(relativexy[1], -30, proportion_h), 
            name.as_ptr(), name.len() as i32 - 1
          );
        }
      }
    }
  }
}

// Contribs: agashlin, WindowsBunnyOne

unsafe extern "system" fn wnd_proc(hwnd: winapi::shared::windef::HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
  use winapi::shared::windef::*;
  use winapi::um::winuser::*;
  use winapi::um::wingdi::*;

  let mut ps = PAINTSTRUCT{
    hdc: null_mut(),
    fErase: 0,
    rcPaint: RECT {left: 0, top: 0, right: 0, bottom: 0},
    fRestore: 0,
    fIncUpdate: 0,
    rgbReserved: [0; 32]
  };

  let data: *mut Vec<[u16; 2]> = GetPropW(hwnd, into_os("pos").as_ptr()) as *mut Vec<[u16; 2]>;
  

  match msg {
      WM_CLOSE => {return DestroyWindow(hwnd) as isize;},
      WM_DESTROY => {RemovePropW(hwnd, into_os("pos").as_ptr()); PostQuitMessage(0); return 0;},
      WM_PAINT => {
        let dc: HDC = BeginPaint(hwnd, &mut ps);
        
        let mut brush = CreateSolidBrush(RGB(0,0,0));
        let mut winrect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut winrect); 
        let proportion_w: f64 = (winrect.right - winrect.left) as f64 / ROOM_WIDTH as f64;
        let proportion_h: f64 = (winrect.bottom - winrect.top) as f64 / ROOM_HEIGHT as f64;

        SelectObject(dc, brush as LPVOID);
        Rectangle(dc, 0,0, 800, 600);
        SetBkColor(dc, RGB(244, 244, 244));

        if data != null_mut() {
          let text = into_os(&format!("Data: {}", (*data).len()));
          TextOutW(dc, 20, 20, text.as_ptr(), text.len() as i32);

          for pos in &*data { 
            let text = into_os(&format!("Hello world, {:?}", pos));
            TextOutW(dc, into_proportion!(pos[0], -10, proportion_w), into_proportion!(pos[1], -30, proportion_h), text.as_ptr(), text.len() as i32);
          }
        }

        DeleteObject(brush as LPVOID);
        return EndPaint(hwnd, &ps) as isize;
      },
      _ => {return DefWindowProcW(hwnd, msg, wparam, lparam);}
  }
}


unsafe fn create_layered(rec: Receiver<([u16; 2], String)>) {
  use winapi::shared::windef::{HWND, POINT, RECT};
  use winapi::um::winuser::*;

  let wc = WNDCLASSW {
    style : CS_OWNDC|CS_VREDRAW|CS_HREDRAW,
    lpfnWndProc : Some( wnd_proc ),
    hInstance : null_mut(),
    lpszClassName : into_os("MyWin").as_ptr(),
    cbClsExtra : 0,
    cbWndExtra : 0,
    hIcon: null_mut(),
    hCursor: null_mut(),
    hbrBackground: null_mut(),
    lpszMenuName: null_mut(),
  };

  RegisterClassW(&wc);

  let oddapp = FindWindowW(null_mut(), into_os("Oddworld Abe's Exoddus").as_ptr());
  let mut pos: RECT = RECT {left: 0, top: 0, right: 0, bottom: 0};
  GetWindowRect(oddapp, &mut pos);

  let win: HWND = CreateWindowExW(
    WS_EX_LAYERED, 
    into_os("MyWin").as_ptr(), 
    into_os("Test").as_ptr(), 
    WS_POPUP, 
    pos.left + 5, pos.top+30, pos.right-pos.left - 5, pos.bottom-pos.top-30,
    oddapp, 
    null_mut(), null_mut(), null_mut()
  );

  if win == null_mut() {
    println!("Angery");
    std::process::exit(0);
  }

  let mut msg = MSG {
    hwnd: null_mut(),
    message: 0,
    wParam: 0,
    lParam: 0,
    time: 0,
    pt: POINT {x: 0, y: 0},
  };

  
  SetLayeredWindowAttributes(win, winapi::um::wingdi::RGB(0,0,0), 0, LWA_COLORKEY);
  ShowWindow(win, SW_SHOWNORMAL);

  //let dataz = vec![[20u16, 40u16], [60u16, 70u16]];
  //SetWindowLongPtrW(win, GWLP_USERDATA, Box::into_raw(Box::new(dataz)) as LONG_PTR);
  //SetWindowPos(win, null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);

  loop {
    let vals = rec.try_iter().map(|vals| vals.0).collect::<Vec<[u16;2]>>();

    if vals.len() != 0 {
      SetPropW(win, into_os("pos").as_ptr(), Box::into_raw(Box::new(vals)) as HANDLE);
      //RedrawWindow(win, null_mut(), null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASENOW);
      InvalidateRect(win, null_mut(), 1);
      UpdateWindow(win);
    }

    PeekMessageW(&mut msg, win, 0, 0, PM_REMOVE);
    TranslateMessage(&msg);
    DispatchMessageW(&msg);
  }
}

fn main() {
  unsafe {
    let oddapp = FindWindowW(null_mut(), into_os("Oddworld Abe's Exoddus").as_ptr()); // HWND to the window.
    let mut proc: DWORD = 0;
    
    // Since we aren't actually writing anything into the game's memory, these are all the privieges we need.
    let access: DWORD = PROCESS_VM_READ | PROCESS_QUERY_INFORMATION; 

    if oddapp == null_mut() { // If you want to debug stuff just set this line to 'false' to avoid the check.
      println!("You need to run Exoddus.exe before starting this program.");
      process::exit(1);
    } 
    
    GetWindowThreadProcessId(oddapp, &mut proc); 
    
    let handle = OpenProcess(access, 0, proc); // We obtain the HANDLE to the underlying process.
    
    // You might be wondering about this pattern where I make explicit pointers to the variables.
    // It's not my sillyness, Rust demands you to be this precise. This results in a few unnecessary lines of code.
    
    // Intentionally set to one, so the client updates as soon as possible.
    let mut saved_muds: u16 = 0;
    let mut previously_muds: u16 = 1; 
    let saved_mudsp: *mut u16 = &mut saved_muds;
    
    let mut pos: [u16; 3] = [0; 3];
    let mut prevpos: [u16; 3] = [0; 3];
    
    let mut prevhero: [u16; 2] = [0; 2];
    
    let (sender, receiver) = channel();
    thread::spawn(|| create_layered(receiver));

    let name = read_name();
    let mut connection = make_connection(name.clone());

    
    
    
    //thread::spawn(move || {render_players(receiver)});
    
    
    ////// This is some confusing mess. //////
    
    /* Writing this was a real pain. Mostly I pointer juggled, until finally I understood how it should be done.
    If you want to understand how this works, I suggest that you read through 'second.cpp' 
    and insert the values below this into something like Cheat Engine.
    For CE to work, you'll need a Pointer, with two offsets and you have to read 2 bytes. The first (lower) offset is always 0x68.
    But as I said, this was mostly shooting in the dark, so if by any chance, you're struggling, it's ok mate. I did too. */
    
    // Abe_x --- Exoddus.exe+0x1C1230, 0x68, 0xBA
    // Abe_y --- Exoddus.exe+0x1C1230, 0x68, 0xBE
    
    use winapi::um::psapi::MODULEINFO;
    use winapi::um::psapi::GetModuleInformation;
    
    // Actually this is a hack, since EnumProcessModules returns an array, in which we should have to manually search for the appropriate module.
    // But in AE's case, the first module is Exoddus.exe itself, so we can load it like this no problem.
    let mut module: HMODULE = null_mut();
    winapi::um::psapi::EnumProcessModules(handle, &mut module, 8, null_mut()); 
    
    // This whole thing is only necessary to get the base address in a format we can actually do calculations with.
    let mut mi: MODULEINFO = MODULEINFO {lpBaseOfDll: null_mut(), SizeOfImage: 0, EntryPoint: null_mut()};
    GetModuleInformation(handle, module, &mut mi, 24);
    let base_pointer = (mi.lpBaseOfDll as u32 + 0x1C1230) as *mut u8; // Exoddus.exe+0x1C1230

    // This pointer is actually holding ANOTHER pointer, which we need to offset by either 0xBA or 0xBE to get the X or Y coordinates.
    let pointer: u32 = 0;
    let pp: *const u32 = &pointer;
    ReadProcessMemory(handle, base_pointer as LPVOID, pp as LPVOID, size_of::<u32>(), null_mut());
    let pos_base = (pointer + 0x68) as *mut u32; 

    let xpos: u16 = 0;
    let xp: *const u16 = &xpos;
    
    let ypos: u16 = 0;
    let yp: *const u16 = &ypos;
    ////// Confusing mess ends here. //////
    
    // This HashMap contains the players' data. See PlayerInfo struct.
    let mut players: HashMap<String, PlayerInfo> = HashMap::new();
    let mut announcement = String::new();
    let mut announcer_counter = 0;
    
    loop {
      // 0x5C1BC2 DWORD number of rescued Mudokons
      // 0x5C3030 - 0x5C3034 3xDWORD = LVL ID, PATH ID, CAM ID

      // Reading the number of saved Mudokons.
      ReadProcessMemory(handle, 0x5C1BC2 as LPCVOID, saved_mudsp as LPVOID, size_of::<u16>(), null_mut()); 
      
      // Reading the current LVL/Path/CAM ID-s.
      ReadProcessMemory(handle, 0x5C3030 as LPCVOID, pos.as_mut_ptr() as LPVOID, size_of::<u16>() * 3, null_mut());
      
      // As the message states this is a really painful thing, but what could I do?
      // OWI, I like you, but ALIVE is messed up.
      if pos[0] == 0 {
        use std::net::Shutdown::Both;
        println!("\r\nBecause of an engine limitation returning\r\nto the main menu messes up the player position tracker.");
        println!("For this reason the app will now exit.\r\nPlease restart it when you entered a map. Sorry for this!");
        connection.shutdown(Both).unwrap();
        process::exit(1);
      }

      // We offset the pointer by 0xBA and thus we can read the X coordinate.
      ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
      ReadProcessMemory(handle, (pointer + 0xBA) as LPCVOID, xp as LPVOID, size_of::<u16>(), null_mut());
      
      // We offset by 0xBE and like before we read the Y coordinate.
      ReadProcessMemory(handle, pos_base as LPCVOID, pp as LPVOID, size_of::<u32>(), null_mut());
      ReadProcessMemory(handle, (pointer + 0xBE) as LPCVOID, yp as LPVOID, size_of::<u16>(), null_mut());

      // The player coordinates are absolute values. The top left of the *entire map* is [0, 0], not the current room.
      // To turn the coordinates into the format we need 
      // We modulo the xpos and the ypos we get a value between [0, 0] and [ROOM_WIDTH, ROOM_HEIGHT].
      let relativexy = [xpos % ROOM_WIDTH, ypos % ROOM_HEIGHT];    

      

      // If anything changed (Abe moved, Mudokons were saved, Abe left the screen), we update the variables and we send the data to the server.
      // ˘Title screen check˘: If we're on the title screen, we don't want to send data.
      if relativexy != [0,0] && previously_muds != saved_muds || prevpos != pos || prevhero != relativexy {
        prevpos = pos;
        previously_muds = saved_muds;
        prevhero = relativexy;
        
        // This will be sent to the server.
        let payload = PlayerInfo {name: name.clone(), saved_muds: saved_muds, location: pos, position: relativexy};
        let bytes: Vec<u8> = serialize(&payload).unwrap(); // We use the Serde Bincode crate for this.
        
        match connection.write_all(bytes.as_slice()) {
          Ok(_) => {},
          Err(e) => {
            use std::io::ErrorKind;
            if e.kind() == ErrorKind::ConnectionAborted {
              println!("Lost connection. Please reconnect!");
              process::exit(0);
            }
          }
        };
      }

      
      // This buffer contains the raw data the server sends us.
      let mut buffer = vec![0;512];
      match connection.read(&mut buffer) {
        Ok(_) => {
          if let Ok(m) = deserialize(&buffer[..]) {
            match m {
              MessageType::ANNOUNCEMENT(inner) => {
                println!("The server sent this message \"{}\"", &inner.trim());
                announcement = inner;

                // One "unit" is 200 milliseconds currently. (See the thread::sleep at the end).
                // So currently this message is displayed for 4000 milliseconds or 4 seconds.
                // TODO: Maybe this should have a better interface. A macro perhaps? Or just a constant?
                announcer_counter = 20; 
              },

              MessageType::PLAYERSTATES => {
                let mut buffer = vec![0;512];
                if let Ok(_) = connection.read(&mut buffer) {
                  if let Ok(infos) = deserialize(&buffer[..]) {
                    let infos: HashMap<String, PlayerInfo> = infos;
                    for (name, plinfo) in infos {
                      players.insert(name.clone(), plinfo.clone());
                    }
                  }
                }
              }
            }
          };
        },
        Err(e) => {
          use std::io::ErrorKind;
          if e.kind() == ErrorKind::ConnectionAborted {
            println!("Lost connection. Please reconnect!");
            process::exit(0);
          }
        }
      };
      
      // If our character is on the same screen as some other player, we send their location for the renderer thread.
      for (_, vals) in &players { if vals.location == pos && pos[0] != 0 {sender.send((vals.position, vals.name.clone())).unwrap();}}
      
      // If there is a current announcement, we send it to the rendering thread and then decrement the counter.
      if announcer_counter != 0 {
        // The coordinates were chosen pretty arbitarily, but it looks good, so I kept it.
        sender.send(([25, 40], announcement.clone())).unwrap();
        announcer_counter -= 1;
      }
      
      // Finally, we sleep to be less straining on the PC.
      thread::sleep(time::Duration::from_millis(200));
    }
  }
}
