#include <windows.h>
#include <cstdio>
#include <cstring>
#include <cwchar>

int main(int argc, char **argv)
{
    if (argc != 2) return 10;
    char path[260];
    std::memset(path, 0x5a, sizeof(path));
    unsigned int stackBefore = 0;
    unsigned int stackAfter = 0;
    __asm {
        lea eax, path
        mov stackBefore, esp
        push 260
        push eax
        _emit 0xc6
        _emit 0x00
        _emit 0x00
        _emit 0x83
        _emit 0xc4
        _emit 0x08
        mov stackAfter, esp
    }
    if (stackBefore != stackAfter || path[0] != 0) return 11;
    for (size_t index = 1; index < sizeof(path); ++index)
        if (path[index] != 0x5a) return 12;
    if (strcat_s(path, "ar_dxvk.dll") != 0) return 13;
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX);
    HMODULE module = LoadLibraryA(path);
    if (std::strcmp(argv[1], "missing") == 0) return module == nullptr ? 0 : 14;
    if (std::strcmp(argv[1], "present") != 0 || module == nullptr) return 15;
    using Probe = int (__cdecl *)();
    auto probe = reinterpret_cast<Probe>(GetProcAddress(module, "ArachnidOriginalProbe"));
    if (probe == nullptr || probe() != 2012) return 16;
    wchar_t executable[MAX_PATH];
    wchar_t loaded[MAX_PATH];
    if (GetModuleFileNameW(nullptr, executable, MAX_PATH) == 0 || GetModuleFileNameW(module, loaded, MAX_PATH) == 0) return 17;
    wchar_t *separator = std::wcsrchr(executable, L'\\');
    if (separator == nullptr) return 18;
    *(separator + 1) = L'\0';
    if (wcscat_s(executable, L"ar_dxvk.dll") != 0 || _wcsicmp(executable, loaded) != 0) return 19;
    FreeLibrary(module);
    std::puts("Original x86 fixture: stack, buffer and application-local delegation verified.");
    return 0;
}
