// ==============================================================================
// POOR MAN'S MATLAB - MODULE: DIGITAL LOGIC & VERILOG EDA
// Architecture: Native DLL Payload + ImNodes
// ==============================================================================
#define IMGUI_DEFINE_MATH_OPERATORS
#include <iostream>
#include <string>
#include <vector>

#include "../Project1/imgui/imgui.h"
#include "../Project1/imgui/imnodes.h"
#include "../Project1/include/IModule.h"

class LogicSimulatorModule : public IModule {
private:
    bool nodesContextCreated = false;

    // Basic state for our hardcoded test nodes
    int switch_state = 0; // 0 = Low, 1 = High

public:
    ~LogicSimulatorModule() {
        if (nodesContextCreated) {
            ImNodes::DestroyContext();
        }
    }

    std::string GetName() override {
        return "EDA: Digital Logic & Verilog Auto-Gen";
    }

    void RenderUI(void* imguiContext) override {
        // 1. Memory Hijack (Same as ImPlot)
        ImGui::SetCurrentContext((ImGuiContext*)imguiContext);
        if (!nodesContextCreated) {
            ImNodes::SetImGuiContext((ImGuiContext*)imguiContext);
            ImNodes::CreateContext();

            // Optional: Make it look like a cool dark-mode Blueprint editor
            ImNodes::StyleColorsDark();
            nodesContextCreated = true;
        }

        // 2. The Top Control Panel
        ImGui::TextColored(ImVec4(0.0f, 1.0f, 1.0f, 1.0f), "Logic Canvas Controls");
        ImGui::Separator();
        if (ImGui::Button("Compile to Verilog", ImVec2(200, 40))) {
            ExecuteMath();
        }
        ImGui::Spacing();

        // 3. THE INFINITE CANVAS
        ImGui::Text("Drag with Middle-Mouse to pan. Click and drag pins to create wires.");

        ImNodes::BeginNodeEditor();

        // --- NODE 1: The Input Switch ---
        // IDs must be unique integers!
        ImNodes::BeginNode(1);
        ImNodes::BeginNodeTitleBar();
        ImGui::TextUnformatted("Input: A");
        ImNodes::EndNodeTitleBar();

        // A node contains attributes (Pins). 
        // Input pins are on the left, Output pins are on the right.
        ImGui::RadioButton("Low (0)", &switch_state, 0); ImGui::SameLine();
        ImGui::RadioButton("High (1)", &switch_state, 1);

        // Output Pin (ID: 100)
        ImNodes::BeginOutputAttribute(100);
        ImGui::Text("Signal Out");
        ImNodes::EndOutputAttribute();
        ImNodes::EndNode();

        // --- NODE 2: The AND Gate ---
        ImNodes::BeginNode(2);
        ImNodes::BeginNodeTitleBar();
        ImGui::TextUnformatted("AND Gate");
        ImNodes::EndNodeTitleBar();

        // Input Pin 1 (ID: 200)
        ImNodes::BeginInputAttribute(200);
        ImGui::Text("In 1");
        ImNodes::EndInputAttribute();

        // Input Pin 2 (ID: 201)
        ImNodes::BeginInputAttribute(201);
        ImGui::Text("In 2");
        ImNodes::EndInputAttribute();

        // Output Pin (ID: 202)
        ImNodes::BeginOutputAttribute(202);
        ImGui::Text("Out");
        ImNodes::EndOutputAttribute();
        ImNodes::EndNode();

        // --- THE WIRE ---
        // Link ID: 1, connects Pin 100 to Pin 200
        ImNodes::Link(1, 100, 200);

        ImNodes::EndNodeEditor();
    }

    void ExecuteMath() override {
        // This is where we will eventually write the C++ logic to 
        // trace the wires and auto-generate the Verilog text!
        std::cout << "Verilog compilation triggered...\n";
    }
};

extern "C" __declspec(dllexport) IModule* CreateModule() {
    return new LogicSimulatorModule();
}