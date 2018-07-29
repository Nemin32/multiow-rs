#include <windows.h>
#include <Psapi.h>
#include <iostream>
#include <string>

int main() {
    HWND oddapp = FindWindow(NULL, "Oddworld Abe's Exoddus");
    DWORD process = 0;
    
    if (oddapp != NULL) {
        GetWindowThreadProcessId(oddapp, &process);        
        DWORD access = 
               PROCESS_ALL_ACCESS;
        
        HANDLE proc = OpenProcess(access, false, process);
        
        
        HMODULE mod;
        EnumProcessModules(proc, &mod, sizeof(HMODULE), NULL);

        
        //Abe_x --- Exoddus.exe+0x1C1230, 0x68, 0xBA
        //Abe_y --- Exoddus.exe+0x1C1230, 0x68, 0xBE
        
        if (mod != NULL) {
            MODULEINFO mi;
            GetModuleInformation(proc, mod, &mi, sizeof(mi));
            char* base_pointer = ((char*)mi.lpBaseOfDll + 0x1C1230);
            
            DWORD pointer;
            ReadProcessMemory(proc, base_pointer, &pointer, sizeof(pointer), NULL);
            DWORD pos_base = (pointer + 0x68);
            
            uint16_t xpos;
            ReadProcessMemory(proc, (DWORD*)pos_base, &pointer, sizeof(pointer), NULL);
            ReadProcessMemory(proc, (DWORD*)(pointer + 0xBA), &xpos, sizeof(xpos), NULL);
            std::cout << xpos << "\n";

            uint16_t ypos;
            ReadProcessMemory(proc, (DWORD*)pos_base, &pointer, sizeof(pointer), NULL);
            ReadProcessMemory(proc, (DWORD*)(pointer + 0xBE), &ypos, sizeof(ypos), NULL);
            std::cout << ypos << "\n";
            
        }
    }
}