#include <iostream>
#include <windows.h>
#include <wingdi.h>

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam)
{
    PAINTSTRUCT ps;
    switch(msg)
    {
        case WM_CLOSE:
            DestroyWindow(hwnd);
        break;
        case WM_DESTROY:
            PostQuitMessage(0);
        break;
        case WM_PAINT:
            {
            HDC dc = BeginPaint(hwnd, &ps);
            SetBkColor(dc, RGB(244, 244, 244));
            TextOutA(dc, 20, 40, "Hello world", 11);
            EndPaint(hwnd, &ps);
            }
        break;
        default:
            return DefWindowProc(hwnd, msg, wParam, lParam);
    }
    return 0;
}

int main() {
    HWND oddapp = FindWindowA(NULL, "Oddworld Abe's Exoddus");
    DWORD process = 0;   
    if (oddapp != NULL) {
        GetWindowThreadProcessId(oddapp, &process);        
        DWORD access = 
               PROCESS_ALL_ACCESS;
        
        HANDLE proc = OpenProcess(access, false, process);
    
        RECT pos;
        RECT size;
        GetWindowRect(oddapp, &pos);
        GetClientRect(oddapp, &size);

        WNDCLASSEX wc;

        //Step 1: Registering the Window Class
        wc.cbSize        = sizeof(WNDCLASSEX);
        wc.style         = 0;
        wc.lpfnWndProc   = WndProc;
        wc.cbClsExtra    = 0;
        wc.cbWndExtra    = 0;
        wc.hInstance     = 0;
        wc.hIcon         = LoadIcon(NULL, IDI_APPLICATION);
        wc.hCursor       = LoadCursor(NULL, IDC_ARROW);
        wc.hbrBackground = (HBRUSH)(COLOR_WINDOW+1);
        wc.lpszMenuName  = NULL;
        wc.lpszClassName = "MyWin";
        wc.hIconSm       = LoadIcon(NULL, IDI_APPLICATION);

        std::cout << wc.cbSize;

        RegisterClassEx(&wc);

        HWND myWin = CreateWindowExA(WS_EX_LAYERED, "MyWin", "Test", WS_POPUP, pos.left, pos.top+30, pos.right-pos.left, size.bottom, oddapp, 0, 0, 0);
        SetLayeredWindowAttributes(myWin, RGB(255,255,255), 0, LWA_COLORKEY);

        ::ShowWindow( myWin, SW_SHOWNORMAL);
        ::UpdateWindow( myWin );
        
        MSG msg = { 0 };
        while ( ::GetMessageW( &msg, myWin, 0, 0 ) > 0 )
        {
            ::TranslateMessage( &msg );
            ::DispatchMessageW( &msg );
        }
    }

    return 0;
}