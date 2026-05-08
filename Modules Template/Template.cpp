// ==============================================================================
// POOR MAN'S MATLAB - MODULE TEMPLATE
// Architecture: Native DLL Payload (No CLI Backup)
// ==============================================================================

#include <iostream>
#include <string>
#include <vector>

// Framework Headers (Adjust paths based on your folder structure)
#include "../Project1/imgui/imgui.h"
#include "../Project1/imgui/implot.h"
#include "../Project1/include/IModule.h"

// Rename this class to whatever your new module is (e.g., LagrangeVisualizer)
class MyCustomModule : public IModule {
private:
    // 1. UI Parameters (Inputs)
    double paramA = 0.0;
    int paramB = 10;

    // 2. Data Vectors (For drawing the graphs)
    std::vector<double> graph_X;
    std::vector<double> graph_Y;

    // 3. Console Output
    std::vector<std::string> consoleLogs;

    // 4. ImPlot Context Flag
    bool plotContextCreated = false;

public:
    // Memory Cleanup
    ~MyCustomModule() {
        if (plotContextCreated) {
            ImPlot::DestroyContext();
        }
    }

    // The name that will appear in the Host Engine UI
    std::string GetName() override {
        return "Template: Blank Math Module";
    }

    // The Graphical Interface
    void RenderUI(void* imguiContext) override {
        // Step 1: Hijack the Host Engine's Memory Context
        ImGui::SetCurrentContext((ImGuiContext*)imguiContext);
        if (!plotContextCreated) {
            ImPlot::SetImGuiContext((ImGuiContext*)imguiContext);
            ImPlot::CreateContext();
            plotContextCreated = true;
        }

        // Step 2: Draw the Control Panel
        ImGui::TextColored(ImVec4(0.0f, 1.0f, 1.0f, 1.0f), "Module Parameters");
        ImGui::Separator();

        ImGui::InputDouble("Parameter A", &paramA);
        ImGui::SliderInt("Parameter B", &paramB, 1, 100);

        if (ImGui::Button("Calculate & Graph", ImVec2(200, 40))) {
            ExecuteMath();
        }
        ImGui::Spacing();
        ImGui::Separator();

        // Step 3: Draw the Graph (Only if we have data)
        if (graph_X.size() > 0) {
            if (ImPlot::BeginPlot("Visual Output", ImVec2(-1, 500))) { // -1 width = fill window

                // Example of drawing a line using the v1.0 ImPlotSpec styling
                ImPlot::PlotLine("Data Curve", graph_X.data(), graph_Y.data(), (int)graph_X.size(),
                    ImPlotSpec(ImPlotProp_LineColor, ImVec4(0.2f, 0.8f, 0.2f, 1.0f), ImPlotProp_LineWeight, 2.0f));

                ImPlot::EndPlot();
            }
        }

        // Step 4: Draw the Output Console
        ImGui::TextColored(ImVec4(1.0f, 1.0f, 0.0f, 1.0f), "Output Console");
        std::string fullLog = "";
        for (const auto& log : consoleLogs) {
            fullLog += log + "\n";
        }
        ImGui::InputTextMultiline("##ConsoleLogs", (char*)fullLog.c_str(), fullLog.size() + 1, ImVec2(-FLT_MIN, 150), ImGuiInputTextFlags_ReadOnly);
    }

    // The Mathematical Engine
    void ExecuteMath() override {
        // Clear previous runs
        consoleLogs.clear();
        graph_X.clear();
        graph_Y.clear();

        consoleLogs.push_back("Executing Math Engine...");

        // Example Math: Generate a simple curve based on parameters
        for (int i = 0; i < paramB; i++) {
            double x = i * 0.1;
            double y = x * x + paramA; // y = x^2 + A

            graph_X.push_back(x);
            graph_Y.push_back(y);
        }

        consoleLogs.push_back("Calculated " + std::to_string(paramB) + " data points.");
        consoleLogs.push_back("Done.");
    }
};

// ==============================================================================
// THE EXPORT HOOK
// The Host Engine looks for this exact function to pull the class into RAM.
// If you rename 'MyCustomModule' above, rename it here too.
// ==============================================================================
extern "C" __declspec(dllexport) IModule* CreateModule() {
    return new MyCustomModule();
}