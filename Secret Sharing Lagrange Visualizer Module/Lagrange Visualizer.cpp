// ==============================================================================
// POOR MAN'S MATLAB - MODULE: LAGRANGE SECRET SHARING (VSS UPGRADE)
// Architecture: Native DLL Payload 
// ==============================================================================

#include <iostream>
#include <string>
#include <vector>
#include <cmath>
#include <random>
#include <sstream>
#include <iomanip>

#include "../Project1/imgui/imgui.h"
#include "../Project1/imgui/implot.h"
#include "../Project1/include/IModule.h"

struct Node {
    double origin_x;
    double origin_y;
    double poisoned_y;    // The fake data if this is a rogue node
    double current_x;
    double current_y;
    double drift_phase;
    bool selected;
    bool is_rogue;        // Is this node compromised?
};

class LagrangeSecretSharing : public IModule {
private:
    double secret_S = 42.0;
    int threshold_K = 3;
    int total_nodes_N = 7;
    int security_mode = 0;        // 0 = Standard, 1 = Feldman VSS

    std::vector<double> true_polynomial_coeffs;
    std::vector<Node> network_nodes;

    std::vector<double> true_curve_X, true_curve_Y;
    std::vector<std::string> consoleLogs;
    bool plotContextCreated = false;

    std::string to_string_prec(double a_value, const int n = 2) {
        std::ostringstream out;
        out << std::fixed << std::setprecision(n) << a_value;
        return out.str();
    }

    void DrawLagrangeCurve(const char* id, const std::vector<Node>& points, ImVec4 color, float thickness) {
        if (points.empty()) return;
        std::vector<double> cx, cy;
        for (int i = 0; i <= 200; ++i) {
            double px = -2.0 + (i * (total_nodes_N + 4.0) / 200.0);
            double py = 0.0;
            for (size_t j = 0; j < points.size(); ++j) {
                double term = points[j].current_y;
                for (size_t m = 0; m < points.size(); ++m) {
                    if (j != m) {
                        term = term * (px - points[m].current_x) / (points[j].current_x - points[m].current_x);
                    }
                }
                py += term;
            }
            cx.push_back(px);
            cy.push_back(py);
        }
        ImPlot::PlotLine(id, cx.data(), cy.data(), (int)cx.size(),
            ImPlotSpec(ImPlotProp_LineColor, color, ImPlotProp_LineWeight, thickness));
    }

public:
    ~LagrangeSecretSharing() {
        if (plotContextCreated) ImPlot::DestroyContext();
    }

    std::string GetName() override { return "Task 4: Cryptographic VSS (Lagrange)"; }

    void RenderUI(void* imguiContext) override {
        ImGui::SetCurrentContext((ImGuiContext*)imguiContext);
        if (!plotContextCreated) {
            ImPlot::SetImGuiContext((ImGuiContext*)imguiContext);
            ImPlot::CreateContext();
            plotContextCreated = true;
        }

        double time = ImGui::GetTime();

        ImGui::TextColored(ImVec4(0.0f, 1.0f, 1.0f, 1.0f), "Network Setup Parameters");
        ImGui::Separator();

        ImGui::InputDouble("The Secret (y-intercept)", &secret_S);
        ImGui::SliderInt("Threshold (K)", &threshold_K, 2, 10);
        ImGui::SliderInt("Total Nodes (N)", &total_nodes_N, threshold_K, 20);

        ImGui::Spacing();
        ImGui::Text("Security Protocol:");
        ImGui::RadioButton("Standard Shamir (Vulnerable)", &security_mode, 0); ImGui::SameLine();
        ImGui::RadioButton("Feldman VSS (Verifiable)", &security_mode, 1);

        ImGui::Spacing();
        if (ImGui::Button("Generate Secure Network", ImVec2(250, 40))) { ExecuteMath(); }
        ImGui::SameLine();
        if (ImGui::Button("Clear Selection", ImVec2(150, 40))) {
            for (auto& n : network_nodes) n.selected = false;
            Log("Selection cleared.");
        }

        ImGui::Spacing();
        ImGui::Separator();

        if (!network_nodes.empty()) {
            for (auto& node : network_nodes) {
                node.current_x = node.origin_x + std::sin(time * 0.8 + node.drift_phase) * 0.4;
                // If rogue, drift around the poisoned Y. Otherwise, drift around the true Y.
                double base_y = node.is_rogue ? node.poisoned_y : node.origin_y;
                node.current_y = base_y + std::cos(time * 1.2 + node.drift_phase) * 2.0;
            }

            ImGui::TextWrapped("INSTRUCTIONS: Catch and click the drifting nodes. Watch out for Rogue nodes!");

            if (ImPlot::BeginPlot("Lagrange Interpolation Space", ImVec2(-1, 500), ImPlotFlags_NoBoxSelect)) {

                if (ImPlot::IsPlotHovered() && ImGui::IsMouseClicked(0)) {
                    ImVec2 mouse_pixels = ImGui::GetMousePos();
                    for (auto& node : network_nodes) {
                        ImVec2 node_pixels = ImPlot::PlotToPixels(node.current_x, node.current_y);
                        float dx = mouse_pixels.x - node_pixels.x;
                        float dy = mouse_pixels.y - node_pixels.y;
                        if (dx * dx + dy * dy < 150.0f) {
                            node.selected = !node.selected;

                            // Log Security Events
                            if (node.selected) {
                                if (node.is_rogue && security_mode == 1) {
                                    Log("-> [VSS ALERT] Active Firewall Blocked Rogue Node at X=" + to_string_prec(node.current_x));
                                }
                                else if (node.is_rogue && security_mode == 0) {
                                    Log("-> [WARNING] Node connected. (System Unaware of Data Poisoning)");
                                }
                                else {
                                    Log("-> Valid Node Connected.");
                                }
                            }
                            break;
                        }
                    }
                }

                double y_axis_x = 0.0;
                ImPlot::PlotInfLines("The Vault (X=0)", &y_axis_x, 1, ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 1.0f, 1.0f, 0.2f), ImPlotProp_LineWeight, 2.0f));

                double zero_x = 0.0;
                ImPlot::PlotScatter("The Secret (S)", &zero_x, &secret_S, 1, ImPlotSpec(ImPlotProp_Marker, ImPlotMarker_Diamond, ImPlotProp_MarkerSize, 12.0f, ImPlotProp_MarkerFillColor, ImVec4(1.0f, 0.8f, 0.0f, 1.0f)));

                std::vector<Node> accepted_nodes, rejected_nodes;
                std::vector<double> sel_x, sel_y, unsel_x, unsel_y, rej_x, rej_y;

                for (const auto& n : network_nodes) {
                    if (n.selected) {
                        // VSS catches rogues and rejects them from the math
                        if (n.is_rogue && security_mode == 1) {
                            rejected_nodes.push_back(n);
                            rej_x.push_back(n.current_x); rej_y.push_back(n.current_y);
                        }
                        else {
                            // Standard mode accepts blindly. VSS accepts valid nodes.
                            accepted_nodes.push_back(n);
                            sel_x.push_back(n.current_x); sel_y.push_back(n.current_y);
                        }
                    }
                    else {
                        unsel_x.push_back(n.current_x); unsel_y.push_back(n.current_y);
                    }
                }

                // Draw Math Curves
                int current_k = accepted_nodes.size();
                if (current_k > 0) {
                    if (current_k < threshold_K) {
                        for (int g = 0; g < 15; ++g) {
                            std::vector<Node> ghost_points = accepted_nodes;
                            for (int f = 0; f < (threshold_K - current_k); ++f) {
                                Node fake;
                                fake.current_x = -1.0 - f;
                                fake.current_y = std::sin(time * 15.0 + g * 3.14 + f) * 50.0;
                                ghost_points.push_back(fake);
                            }
                            DrawLagrangeCurve("##Ghost", ghost_points, ImVec4(1.0f, 0.0f, 0.0f, 0.15f), 1.0f);
                        }
                    }
                    else {
                        DrawLagrangeCurve("Lagrange Reconstruction", accepted_nodes, ImVec4(0.0f, 0.8f, 1.0f, 1.0f), 3.0f);

                        // Check if we hit the actual secret
                        double guessed_secret = 0.0;
                        for (size_t j = 0; j < accepted_nodes.size(); ++j) {
                            double term = accepted_nodes[j].current_y;
                            for (size_t m = 0; m < accepted_nodes.size(); ++m) {
                                if (j != m) term = term * (0.0 - accepted_nodes[m].current_x) / (accepted_nodes[j].current_x - accepted_nodes[m].current_x);
                            }
                            guessed_secret += term;
                        }

                        if (std::abs(guessed_secret - secret_S) < 0.1) {
                            ImPlot::PlotScatter("##Success", &zero_x, &secret_S, 1, ImPlotSpec(ImPlotProp_Marker, ImPlotMarker_Circle, ImPlotProp_MarkerSize, 20.0f, ImPlotProp_MarkerLineColor, ImVec4(0.0f, 1.0f, 0.0f, 1.0f), ImPlotProp_LineWeight, 3.0f));
                        }
                        else {
                            ImPlot::PlotScatter("##Fail", &zero_x, &guessed_secret, 1, ImPlotSpec(ImPlotProp_Marker, ImPlotMarker_Cross, ImPlotProp_MarkerSize, 20.0f, ImPlotProp_MarkerLineColor, ImVec4(1.0f, 0.0f, 0.0f, 1.0f), ImPlotProp_LineWeight, 3.0f));
                            Log("-> [CRITICAL] DATA POISONED. Secret Compromised! Guessed: " + to_string_prec(guessed_secret));
                        }
                    }
                }

                // Draw Dots
                if (!unsel_x.empty()) ImPlot::PlotScatter("Idle Nodes", unsel_x.data(), unsel_y.data(), (int)unsel_x.size(), ImPlotSpec(ImPlotProp_MarkerSize, 6.0f, ImPlotProp_MarkerFillColor, ImVec4(0.5f, 0.5f, 0.5f, 1.0f)));
                if (!sel_x.empty()) ImPlot::PlotScatter("Connected Nodes", sel_x.data(), sel_y.data(), (int)sel_x.size(), ImPlotSpec(ImPlotProp_MarkerSize, 8.0f, ImPlotProp_MarkerFillColor, ImVec4(0.0f, 1.0f, 0.0f, 1.0f)));
                if (!rej_x.empty()) ImPlot::PlotScatter("REJECTED (Rogue)", rej_x.data(), rej_y.data(), (int)rej_x.size(), ImPlotSpec(ImPlotProp_Marker, ImPlotMarker_Cross, ImPlotProp_MarkerSize, 12.0f, ImPlotProp_MarkerLineColor, ImVec4(1.0f, 0.0f, 0.0f, 1.0f), ImPlotProp_LineWeight, 3.0f));

                ImPlot::EndPlot();
            }
        }

        ImGui::TextColored(ImVec4(1.0f, 1.0f, 0.0f, 1.0f), "Diagnostic Log");
        std::string fullLog = "";
        for (auto it = consoleLogs.rbegin(); it != consoleLogs.rend(); ++it) fullLog += *it + "\n";
        ImGui::InputTextMultiline("##ConsoleLogs", (char*)fullLog.c_str(), fullLog.size() + 1, ImVec2(-FLT_MIN, 150), ImGuiInputTextFlags_ReadOnly);
    }

    void Log(const std::string& msg) { consoleLogs.push_back(msg); }

    void ExecuteMath() override {
        consoleLogs.clear();
        network_nodes.clear();
        true_polynomial_coeffs.clear();

        std::mt19937 rng(std::random_device{}());
        std::uniform_real_distribution<double> dist(-15.0, 15.0);
        std::uniform_real_distribution<double> phase_dist(0.0, 6.28);

        true_polynomial_coeffs.push_back(secret_S);
        for (int i = 1; i < threshold_K; ++i) true_polynomial_coeffs.push_back(dist(rng));

        // Determine how many rogues to spawn (at least 1, max 30% of network)
        int num_rogues = std::max(1, total_nodes_N / 3);
        std::vector<int> rogue_indices;
        for (int i = 0; i < num_rogues; i++) rogue_indices.push_back((rng() % total_nodes_N) + 1);

        for (int i = 1; i <= total_nodes_N; ++i) {
            double x = (double)i;
            double y = 0.0;
            for (size_t p = 0; p < true_polynomial_coeffs.size(); ++p) {
                y += true_polynomial_coeffs[p] * std::pow(x, p);
            }

            bool is_rogue = std::find(rogue_indices.begin(), rogue_indices.end(), i) != rogue_indices.end();
            double poisoned = y + (dist(rng) > 0 ? 15.0 : -15.0) + dist(rng); // Push it wildly off the curve

            network_nodes.push_back({ x, y, poisoned, x, y, phase_dist(rng), false, is_rogue });
        }

        Log("======================================");
        Log("SYSTEM: New Secure MANET Generated.");
        Log("Target Threshold (K): " + std::to_string(threshold_K) + " nodes required.");
        Log("WARNING: System detected " + std::to_string(num_rogues) + " rogue nodes in sector.");
    }
};

extern "C" __declspec(dllexport) IModule* CreateModule() {
    return new LagrangeSecretSharing();
}