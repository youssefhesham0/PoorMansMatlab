// ==============================================================================
// POOR MAN'S MATLAB - HOST ENGINE (v2.0 - Multi-Module Architecture)
// Architecture: DirectX 11 + Dear ImGui
// ==============================================================================

#include "imgui/imgui.h"
#include "imgui/imgui_impl_win32.h"
#include "imgui/imgui_impl_dx11.h"
#include <d3d11.h>
#include <tchar.h>
#include <windows.h>
#include <filesystem>
#include <string>
#include <vector>
#include "include/IModule.h" // The Contract

// --- GLOBAL STATE (Upgraded for Multi-Targeting) ---
std::vector<HMODULE> g_LoadedDLLs;       // Holds all physical DLLs in RAM
std::vector<IModule*> g_Modules;         // Holds all the math logic pointers
int g_SelectedModuleIdx = -1;            // Tracks which module is currently on screen
std::string g_StatusMessage = "Awaiting DLL Injection...";

// Auto-link the Windows DirectX libraries
#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "d3dcompiler.lib")

// Global DirectX variables
static ID3D11Device* g_pd3dDevice = nullptr;
static ID3D11DeviceContext* g_pd3dDeviceContext = nullptr;
static IDXGISwapChain* g_pSwapChain = nullptr;
static ID3D11RenderTargetView* g_mainRenderTargetView = nullptr;

// Helper functions 
bool CreateDeviceD3D(HWND hWnd);
void CleanupDeviceD3D();
void CreateRenderTarget();
void CleanupRenderTarget();
LRESULT WINAPI WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

// Main Application Entry
int main(int, char**)
{
    // 1. Create a native Windows Application Window
    WNDCLASSEX wc = { sizeof(WNDCLASSEX), CS_CLASSDC, WndProc, 0L, 0L, GetModuleHandle(NULL), NULL, NULL, NULL, NULL, _T("PoorMansMatlab"), NULL };
    ::RegisterClassEx(&wc);
    HWND hwnd = ::CreateWindow(wc.lpszClassName, _T("Poor Man's MATLAB - Host Engine"), WS_OVERLAPPEDWINDOW, 100, 100, 1280, 800, NULL, NULL, wc.hInstance, NULL);

    if (!CreateDeviceD3D(hwnd)) {
        CleanupDeviceD3D();
        ::UnregisterClass(wc.lpszClassName, wc.hInstance);
        return 1;
    }

    ::ShowWindow(hwnd, SW_SHOWDEFAULT);
    ::UpdateWindow(hwnd);

    // 2. Initialize Dear ImGui
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO(); (void)io;
    ImGui::StyleColorsDark();

    ImGui_ImplWin32_Init(hwnd);
    ImGui_ImplDX11_Init(g_pd3dDevice, g_pd3dDeviceContext);

    // 3. The Main Render Loop
    bool done = false;
    while (!done)
    {
        MSG msg;
        while (::PeekMessage(&msg, NULL, 0U, 0U, PM_REMOVE)) {
            ::TranslateMessage(&msg);
            ::DispatchMessage(&msg);
            if (msg.message == WM_QUIT) done = true;
        }
        if (done) break;

        ImGui_ImplDX11_NewFrame();
        ImGui_ImplWin32_NewFrame();
        ImGui::NewFrame();

        // -----------------------------------------------------------
        // OUR NEW SPLIT-SCREEN GUI 
        // -----------------------------------------------------------
        ImGui::SetNextWindowPos(ImVec2(0, 0));
        ImGui::SetNextWindowSize(io.DisplaySize);
        ImGui::Begin("Workspace", nullptr, ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove);

        // --- LEFT PANEL: THE CARTRIDGE SLOT (Sidebar) ---
        ImGui::BeginChild("Sidebar", ImVec2(300, 0), true);

        ImGui::Text("Engine Control");
        ImGui::Separator();
        ImGui::Spacing();

        if (ImGui::Button("Scan & Load Modules", ImVec2(-1, 40))) {
            namespace fs = std::filesystem;
            char exePath[MAX_PATH];
            GetModuleFileNameA(NULL, exePath, MAX_PATH);
            fs::path absoluteExeDir = fs::path(exePath).parent_path();
            fs::path targetFolder = absoluteExeDir / "modules";

            if (fs::exists(targetFolder)) {
                int loadCount = 0;
                for (const auto& entry : fs::directory_iterator(targetFolder)) {
                    if (entry.path().extension() == ".dll") {

                        // Prevent loading the exact same DLL twice if the user clicks scan again
                        bool alreadyLoaded = false;
                        for (auto hDll : g_LoadedDLLs) {
                            char dllPath[MAX_PATH];
                            GetModuleFileNameA(hDll, dllPath, MAX_PATH);
                            if (std::string(dllPath) == entry.path().string()) {
                                alreadyLoaded = true;
                                break;
                            }
                        }

                        if (!alreadyLoaded) {
                            HMODULE hDll = LoadLibraryA(entry.path().string().c_str());
                            if (hDll) {
                                CreateModuleFunc createFunc = (CreateModuleFunc)GetProcAddress(hDll, "CreateModule");
                                if (createFunc) {
                                    g_LoadedDLLs.push_back(hDll);
                                    g_Modules.push_back(createFunc());
                                    loadCount++;
                                }
                            }
                        }
                    }
                }
                g_StatusMessage = "Loaded " + std::to_string(loadCount) + " new modules.";
            }
            else {
                g_StatusMessage = "Error: /modules folder not found!";
            }
        }

        ImGui::Spacing();
        ImGui::TextColored(ImVec4(0.5f, 0.5f, 0.5f, 1.0f), "Status: %s", g_StatusMessage.c_str());

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        ImGui::TextColored(ImVec4(0.0f, 1.0f, 1.0f, 1.0f), "Available Modules");
        ImGui::Separator();

        // Build the selectable list of loaded modules
        for (int i = 0; i < g_Modules.size(); i++) {
            // If the user clicks this item, we change the active index!
            if (ImGui::Selectable(g_Modules[i]->GetName().c_str(), g_SelectedModuleIdx == i)) {
                g_SelectedModuleIdx = i;
            }
        }

        ImGui::EndChild();
        // --- END LEFT PANEL ---

        ImGui::SameLine();

        // --- RIGHT PANEL: THE CANVAS ---
        ImGui::BeginChild("Canvas", ImVec2(0, 0), true);

        if (g_SelectedModuleIdx >= 0 && g_SelectedModuleIdx < g_Modules.size()) {
            // Pass the GPU keys to whichever module is currently selected in the sidebar
            g_Modules[g_SelectedModuleIdx]->RenderUI(ImGui::GetCurrentContext());
        }
        else {
            // Idle Screen
            ImGui::TextColored(ImVec4(0.5f, 0.5f, 0.5f, 1.0f), "No module selected.");
            ImGui::Text("Click 'Scan & Load Modules' and select a payload from the sidebar to begin.");
        }

        ImGui::EndChild();
        // --- END RIGHT PANEL ---

        ImGui::End();
        // -----------------------------------------------------------

        ImGui::Render();
        const float clear_color_with_alpha[4] = { 0.1f, 0.1f, 0.1f, 1.0f };
        g_pd3dDeviceContext->OMSetRenderTargets(1, &g_mainRenderTargetView, NULL);
        g_pd3dDeviceContext->ClearRenderTargetView(g_mainRenderTargetView, clear_color_with_alpha);
        ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
        g_pSwapChain->Present(1, 0);
    }

    // 4. Shutdown and Clean Up Memory
    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext();
    CleanupDeviceD3D();
    ::DestroyWindow(hwnd);
    ::UnregisterClass(wc.lpszClassName, wc.hInstance);

    // Clean up all the modules we created so we don't leak RAM
    for (IModule* mod : g_Modules) { delete mod; }
    for (HMODULE hDll : g_LoadedDLLs) { FreeLibrary(hDll); }

    return 0;
}

// ==============================================================================
// DIRECTX 11 BOILERPLATE
// ==============================================================================
extern IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

LRESULT WINAPI WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if (ImGui_ImplWin32_WndProcHandler(hWnd, msg, wParam, lParam)) return true;
    switch (msg) {
    case WM_SIZE:
        if (g_pd3dDevice != NULL && wParam != SIZE_MINIMIZED) {
            CleanupRenderTarget();
            g_pSwapChain->ResizeBuffers(0, (UINT)LOWORD(lParam), (UINT)HIWORD(lParam), DXGI_FORMAT_UNKNOWN, 0);
            CreateRenderTarget();
        }
        return 0;
    case WM_SYSCOMMAND:
        if ((wParam & 0xfff0) == SC_KEYMENU) return 0;
        break;
    case WM_DESTROY:
        ::PostQuitMessage(0);
        return 0;
    }
    return ::DefWindowProc(hWnd, msg, wParam, lParam);
}

bool CreateDeviceD3D(HWND hWnd) {
    DXGI_SWAP_CHAIN_DESC sd;
    ZeroMemory(&sd, sizeof(sd));
    sd.BufferCount = 2;
    sd.BufferDesc.Width = 0;
    sd.BufferDesc.Height = 0;
    sd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sd.BufferDesc.RefreshRate.Numerator = 60;
    sd.BufferDesc.RefreshRate.Denominator = 1;
    sd.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.OutputWindow = hWnd;
    sd.SampleDesc.Count = 1;
    sd.SampleDesc.Quality = 0;
    sd.Windowed = TRUE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;

    UINT createDeviceFlags = 0;
    D3D_FEATURE_LEVEL featureLevel;
    const D3D_FEATURE_LEVEL featureLevelArray[2] = { D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0, };
    if (D3D11CreateDeviceAndSwapChain(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, createDeviceFlags, featureLevelArray, 2, D3D11_SDK_VERSION, &sd, &g_pSwapChain, &g_pd3dDevice, &featureLevel, &g_pd3dDeviceContext) != S_OK)
        return false;
    CreateRenderTarget();
    return true;
}

void CleanupDeviceD3D() {
    CleanupRenderTarget();
    if (g_pSwapChain) { g_pSwapChain->Release(); g_pSwapChain = NULL; }
    if (g_pd3dDeviceContext) { g_pd3dDeviceContext->Release(); g_pd3dDeviceContext = NULL; }
    if (g_pd3dDevice) { g_pd3dDevice->Release(); g_pd3dDevice = NULL; }
}

void CreateRenderTarget() {
    ID3D11Texture2D* pBackBuffer = nullptr;
    g_pSwapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    if (pBackBuffer) {
        g_pd3dDevice->CreateRenderTargetView(pBackBuffer, NULL, &g_mainRenderTargetView);
        pBackBuffer->Release();
    }
}

void CleanupRenderTarget() {
    if (g_mainRenderTargetView) {
        g_mainRenderTargetView->Release();
        g_mainRenderTargetView = NULL;
    }
}