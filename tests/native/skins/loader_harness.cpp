#define DIRECTINPUT_VERSION 0x0800
#include <windows.h>
#include <dinput.h>
#include <d3d9.h>
#include <iostream>

int wmain()
{
    IDirectInput8W* input = nullptr;
    const auto result = DirectInput8Create(GetModuleHandleW(nullptr), DIRECTINPUT_VERSION,
        IID_IDirectInput8W, reinterpret_cast<void**>(&input), nullptr);
    if (FAILED(result) || input == nullptr) return 10;
    input->Release();
    if (GetModuleHandleW(L"arachnid_skin.asi") == nullptr) return 11;
    if (GetModuleHandleW(L"unknown.asi") != nullptr) return 12;
    if (GetModuleHandleW(L"dinput8Hooked.dll") != nullptr) return 14;
    auto graphics = Direct3DCreate9(D3D_SDK_VERSION);
    if (graphics == nullptr) return 13;
    graphics->Release();
    std::cout << "x86 DirectInput forwarding, managed ASI initialization and Direct3D9 entry passed\n";
    return 0;
}
