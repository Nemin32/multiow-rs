#include <Windows.h>
#include <Wingdi.h>
#include <iostream>

#include <thread>

uint16_t MUDNUM = 0;

void drawOnScreen(HWND win) {
    HDC hdc = GetDC(win);
    RECT r;
    GetWindowRect(win, &r);
    
    r.left += 30;
    r.top += 10;
    
    char buffer[4];
    
    while (true) {
        sprintf(buffer, "%d", MUDNUM);
        DrawText(hdc, buffer, -1, &r, 0);
    }
    
    ReleaseDC(win, hdc);
}

int main() {
    HWND oddapp = FindWindow(NULL, "Oddworld Abe's Exoddus");
    DWORD process = 0;
    
    if (oddapp != NULL) {
        GetWindowThreadProcessId(oddapp, &process);        
        DWORD access = 
               PROCESS_VM_READ |
               PROCESS_QUERY_INFORMATION;
        
        HANDLE proc = OpenProcess(access, false, process);
        uint16_t read = 0;
        uint16_t old = 1;
        SIZE_T bytes = 0;
        
        std::thread t1(drawOnScreen, oddapp);
        
        while (ReadProcessMemory(proc, (void*)0x5C1BC2, &read, sizeof(uint16_t), &bytes)) {
            if (read != old) {
                std::cout << read << "\n";
                old = read;
                
                MUDNUM = read;
            }
            
            Sleep(1000);
        }
        
        t1.join();
        CloseHandle(proc);
    }
    
    return 0;
}