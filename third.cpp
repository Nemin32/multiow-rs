#include <iostream>
#include <windows.h>
#include <wingdi.h>

int main() {
    HWND oddapp = FindWindow(NULL, "Oddworld Abe's Exoddus");
    DWORD process = 0;
    
    if (oddapp != NULL) {
        GetWindowThreadProcessId(oddapp, &process);        
        DWORD access = 
               PROCESS_ALL_ACCESS;
        
        HANDLE proc = OpenProcess(access, false, process);
    }

    HWND hWnd = CreateWindowEx(WS_EX_LAYERED, NULL, 0, WS_OVERLAPPED,
        CW_USEDEFAULT, 0, CW_USEDEFAULT, 0, oddapp, NULL, NULL, NULL);

    int width = 800;
    int height = 600;

    HDC hdcScreen = GetDC(oddapp);
    HDC hdc = CreateCompatibleDC(hdcScreen);
    ReleaseDC(0, hdcScreen);

    POINT dcOffset = {0, 0};
    SIZE size = {width, height};
    BLENDFUNCTION bf;
    bf.BlendOp = AC_SRC_OVER;
    bf.BlendFlags = 0;
    bf.SourceConstantAlpha = 255;
    bf.AlphaFormat = AC_SRC_ALPHA;
    
    HBRUSH b = CreateSolidBrush(RGB(255, 0, 0));
    SelectObject(hdc, &b);
    Rectangle(hdc, 0,0, 200, 200);
    
    UpdateLayeredWindow(hWnd, 0, 0, &size, hdc, &dcOffset, 0, &bf, ULW_ALPHA);
    DeleteDC(hdc);

    ShowWindow(hWnd, SW_SHOW);

    MSG msg;

    // Main message loop:
    while (GetMessage(&msg, NULL, 0, 0))
    {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    return (int)msg.wParam;
}