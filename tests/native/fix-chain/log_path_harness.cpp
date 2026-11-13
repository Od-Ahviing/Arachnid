#include <windows.h>
#include <fstream>
#include <string>
#include <cstdio>

int main()
{
    wchar_t directory[MAX_PATH];
    const DWORD length = GetCurrentDirectoryW(MAX_PATH, directory);
    if (length == 0 || length >= MAX_PATH || directory[1] != L':') return 20;
    const std::wstring normal = std::wstring(directory) + L"/normal-dxvk-style.log";
    const std::wstring extended = L"\\\\?\\" + std::wstring(directory) + L"/verbatim-dxvk-style.log";
    std::ofstream normalLog(normal.c_str());
    if (!normalLog) return 21;
    normalLog << "Original logging fixture only.\n";
    normalLog.close();
    std::ofstream extendedLog(extended.c_str());
    if (extendedLog) return 22;
    std::puts("Original Windows fixture: verbatim prefix with slash prevents log creation; normal path succeeds.");
    return 0;
}
