// ==============================================================================
// Numerical Integration Assignment
// Student Name: Youssef Hesham Said Soliman
// Student ID: [Enter Your ID]
//
// Architecture: Polymorphic C++ Source File
// ==============================================================================

#include <iostream>
#include <cmath>
#include <iomanip>
#include <functional>
#include <string>
#include <vector>
#include <algorithm> 

#if defined(_WINDLL)

    // ==========================================================================
    // REALITY 1: THE GRAPHICS ENGINE PAYLOAD (DLL)
    // ==========================================================================

#include "../Project1/imgui/imgui.h"
#include "../Project1/imgui/implot.h"
#include "../Project1/include/IModule.h"
#include "../Project1/include/tinyexpr.h"

class NumericalIntegration : public IModule {
private:
    double lowerBound = 0.0;
    double upperBound = 1.0;
    int intervals = 4;
    char functionInput[256] = "sin(x) + e^x";

    std::vector<std::string> consoleLogs;
    std::vector<double> graph_X, graph_Y;

    // Riemann Sum Vectors
    std::vector<double> rie_X, rie_Y_approx, rie_Y_perf, rie_Y_over, rie_Y_under;
    std::vector<double> rie_stems_X, rie_stems_Y;

    // Trapezoidal Vectors
    std::vector<double> trap_X, trap_Y_approx, trap_Y_perf, trap_Y_over, trap_Y_under;
    std::vector<double> trap_stems_X, trap_stems_Y;

    // Simpson's Vectors
    std::vector<double> simp_X, simp_Y_approx, simp_Y_perf, simp_Y_over, simp_Y_under;
    std::vector<double> simp_stems_X, simp_stems_Y;

    bool plotContextCreated = false;

public:
    ~NumericalIntegration() {
        if (plotContextCreated) {
            ImPlot::DestroyContext();
        }
    }

    std::string GetName() override {
        return "Task 3: Riemann, Trapezoidal & Simpson's";
    }

    void RenderUI(void* imguiContext) override {
        ImGui::SetCurrentContext((ImGuiContext*)imguiContext);

        if (!plotContextCreated) {
            ImPlot::SetImGuiContext((ImGuiContext*)imguiContext);
            ImPlot::CreateContext();
            plotContextCreated = true;
        }

        ImGui::TextColored(ImVec4(0.0f, 1.0f, 1.0f, 1.0f), "Integration Parameters");
        ImGui::Separator();

        ImGui::InputText("Function f(x)", functionInput, IM_ARRAYSIZE(functionInput));
        ImGui::InputDouble("Lower Bound (a)", &lowerBound);
        ImGui::InputDouble("Upper Bound (b)", &upperBound);
        ImGui::SliderInt("Intervals (n)", &intervals, 2, 1000);

        if (ImGui::Button("Calculate & Graph", ImVec2(200, 40))) {
            ExecuteMath();
        }

        ImGui::Spacing();
        ImGui::Separator();

        if (graph_X.size() > 0) {
            if (ImPlot::BeginSubplots("Error Analysis Suite", 4, 1, ImVec2(-1, 1000))) {

                // --- GRAPH 1: Left Riemann Sum ---
                if (ImPlot::BeginPlot("1. Left Riemann Sum (Rectangles)")) {

                    ImPlot::PlotShaded("Base Area", rie_X.data(), rie_Y_approx.data(), (int)rie_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(0.2f, 0.5f, 0.8f, 0.6f)));

                    ImPlot::PlotShaded("Overestimate (Error)", rie_X.data(), rie_Y_over.data(), rie_Y_perf.data(), (int)rie_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.2f, 0.2f, 0.7f)));

                    ImPlot::PlotShaded("Underestimate (Error)", rie_X.data(), rie_Y_perf.data(), rie_Y_under.data(), (int)rie_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.9f, 0.0f, 0.7f)));

                    ImPlot::PlotLine("Top Edge", rie_X.data(), rie_Y_approx.data(), (int)rie_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 1.0f, 1.0f, 1.0f), ImPlotProp_LineWeight, 1.5f));

                    ImPlot::PlotStems("Inner Walls", rie_stems_X.data(), rie_stems_Y.data(), (int)rie_stems_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(0.6f, 0.6f, 0.6f, 1.0f), ImPlotProp_LineWeight, 1.0f));

                    ImPlot::PlotLine("Discrete Sampling (500 pts)", graph_X.data(), graph_Y.data(), (int)graph_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 0.5f, 0.0f, 2.0f), ImPlotProp_LineWeight, 2.0f));

                    ImPlot::EndPlot();
                }

                // --- GRAPH 2: Trapezoidal Rule ---
                if (ImPlot::BeginPlot("2. Trapezoidal Rule (Straight Lines)")) {

                    ImPlot::PlotShaded("Base Area", trap_X.data(), trap_Y_approx.data(), (int)trap_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(0.2f, 0.5f, 0.8f, 0.6f)));

                    ImPlot::PlotShaded("Overestimate (Error)", trap_X.data(), trap_Y_over.data(), trap_Y_perf.data(), (int)trap_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.2f, 0.2f, 0.7f)));

                    ImPlot::PlotShaded("Underestimate (Error)", trap_X.data(), trap_Y_perf.data(), trap_Y_under.data(), (int)trap_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.9f, 0.0f, 0.7f)));

                    ImPlot::PlotLine("Top Edge", trap_X.data(), trap_Y_approx.data(), (int)trap_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 1.0f, 1.0f, 1.0f), ImPlotProp_LineWeight, 1.5f));

                    ImPlot::PlotStems("Inner Walls", trap_stems_X.data(), trap_stems_Y.data(), (int)trap_stems_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(0.6f, 0.6f, 0.6f, 1.0f), ImPlotProp_LineWeight, 1.0f));

                    ImPlot::PlotLine("Perfect Curve", graph_X.data(), graph_Y.data(), (int)graph_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 0.5f, 0.0f, 2.0f), ImPlotProp_LineWeight, 2.0f));

                    ImPlot::EndPlot();
                }

                // --- GRAPH 3: Simpson's 1/3 Rule ---
                if (ImPlot::BeginPlot("3. Simpson's 1/3 Rule (Quadratic Parabolas)")) {

                    ImPlot::PlotShaded("Base Area", simp_X.data(), simp_Y_approx.data(), (int)simp_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(0.2f, 0.5f, 0.8f, 0.6f)));

                    ImPlot::PlotShaded("Overestimate (Error)", simp_X.data(), simp_Y_over.data(), simp_Y_perf.data(), (int)simp_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.2f, 0.2f, 0.7f)));

                    ImPlot::PlotShaded("Underestimate (Error)", simp_X.data(), simp_Y_perf.data(), simp_Y_under.data(), (int)simp_X.size(),
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(1.0f, 0.9f, 0.0f, 0.7f)));

                    ImPlot::PlotLine("Parabola Edges", simp_X.data(), simp_Y_approx.data(), (int)simp_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 1.0f, 1.0f, 1.0f), ImPlotProp_LineWeight, 1.5f));

                    ImPlot::PlotStems("Inner Walls", simp_stems_X.data(), simp_stems_Y.data(), (int)simp_stems_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(0.6f, 0.6f, 0.6f, 1.0f), ImPlotProp_LineWeight, 1.0f));

                    ImPlot::PlotLine("Perfect Curve", graph_X.data(), graph_Y.data(), (int)graph_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 0.5f, 0.0f, 2.0f), ImPlotProp_LineWeight, 2.0f));

                    ImPlot::EndPlot();
                }

                // --- GRAPH 4: Perfect Analytical Area ---
                if (ImPlot::BeginPlot("4. High-Resolution Discrete Sampling")) {

                    ImPlot::PlotShaded("Exact Area", graph_X.data(), graph_Y.data(), (int)graph_X.size(), 0.0,
                        ImPlotSpec(ImPlotProp_FillColor, ImVec4(0.2f, 0.5f, 0.8f, 0.6f)));

                    ImPlot::PlotLine("f(x)", graph_X.data(), graph_Y.data(), (int)graph_X.size(),
                        ImPlotSpec(ImPlotProp_LineColor, ImVec4(1.0f, 0.5f, 0.0f, 2.0f), ImPlotProp_LineWeight, 2.0f));

                    ImPlot::EndPlot();
                }

                ImPlot::EndSubplots();
            }
        }

        ImGui::TextColored(ImVec4(1.0f, 1.0f, 0.0f, 1.0f), "Output Console");
        std::string fullLog = "";
        for (const auto& log : consoleLogs) {
            fullLog += log + "\n";
        }
        ImGui::InputTextMultiline("##ConsoleLogs", (char*)fullLog.c_str(), fullLog.size() + 1, ImVec2(-FLT_MIN, 150), ImGuiInputTextFlags_ReadOnly);
    }

    void ExecuteMath() override {
        consoleLogs.clear();
        graph_X.clear(); graph_Y.clear();
        rie_X.clear(); rie_Y_approx.clear(); rie_Y_perf.clear(); rie_Y_over.clear(); rie_Y_under.clear(); rie_stems_X.clear(); rie_stems_Y.clear();
        trap_X.clear(); trap_Y_approx.clear(); trap_Y_perf.clear(); trap_Y_over.clear(); trap_Y_under.clear(); trap_stems_X.clear(); trap_stems_Y.clear();
        simp_X.clear(); simp_Y_approx.clear(); simp_Y_perf.clear(); simp_Y_over.clear(); simp_Y_under.clear(); simp_stems_X.clear(); simp_stems_Y.clear();

        consoleLogs.push_back("Executing Integration for f(x) = " + std::string(functionInput));

        double currentX = 0;
        te_variable vars[] = { {"x", &currentX} };
        int err;
        te_expr* expr = te_compile(functionInput, vars, 1, &err);

        if (!expr) {
            consoleLogs.push_back("ERROR: Invalid math syntax near character " + std::to_string(err));
            return;
        }

        int plotPoints = 500;
        double plotStep = (upperBound - lowerBound) / plotPoints;
        for (int i = 0; i <= plotPoints; i++) {
            currentX = lowerBound + (i * plotStep);
            graph_X.push_back(currentX);
            graph_Y.push_back(te_eval(expr));
        }

        double deltaX = (upperBound - lowerBound) / intervals;

        // 1. Riemann Sum (Left)
        double riemannArea = 0.0;
        for (int i = 0; i < intervals; ++i) {
            double x0 = lowerBound + (i * deltaX);
            double x1 = lowerBound + ((i + 1) * deltaX);
            currentX = x0; double y0 = te_eval(expr);

            riemannArea += y0 * deltaX;
            rie_stems_X.push_back(x0); rie_stems_Y.push_back(y0);
            if (i == intervals - 1) { rie_stems_X.push_back(x1); rie_stems_Y.push_back(y0); }

            int steps = 20;
            double subStep = (x1 - x0) / steps;
            for (int j = 0; j <= steps; ++j) {
                double px = x0 + (j * subStep);
                currentX = px; double py_perf = te_eval(expr);

                rie_X.push_back(px);
                rie_Y_approx.push_back(y0);
                rie_Y_perf.push_back(py_perf);
                rie_Y_over.push_back(std::max(y0, py_perf));
                rie_Y_under.push_back(std::min(y0, py_perf));
            }
        }

        // 2. Trapezoidal Rule
        double trapArea = 0.0;
        currentX = lowerBound; double fa = te_eval(expr);
        currentX = upperBound; double fb = te_eval(expr);
        trapArea += (fa + fb) / 2.0;

        for (int i = 1; i < intervals; ++i) {
            currentX = lowerBound + (i * deltaX);
            trapArea += te_eval(expr);
        }
        trapArea *= deltaX;

        for (int i = 0; i < intervals; ++i) {
            double x0 = lowerBound + (i * deltaX);
            double x1 = lowerBound + ((i + 1) * deltaX);
            currentX = x0; double y0 = te_eval(expr);
            currentX = x1; double y1 = te_eval(expr);

            if (i == 0) { trap_stems_X.push_back(x0); trap_stems_Y.push_back(y0); }
            trap_stems_X.push_back(x1); trap_stems_Y.push_back(y1);

            int steps = 20;
            double subStep = (x1 - x0) / steps;
            for (int j = 0; j <= steps; ++j) {
                double px = x0 + (j * subStep);
                double py_line = y0 + (y1 - y0) * (px - x0) / (x1 - x0);
                currentX = px; double py_perf = te_eval(expr);

                trap_X.push_back(px);
                trap_Y_approx.push_back(py_line);
                trap_Y_perf.push_back(py_perf);
                trap_Y_over.push_back(std::max(py_line, py_perf));
                trap_Y_under.push_back(std::min(py_line, py_perf));
            }
        }

        // 3. Simpson's 1/3 Rule
        double simpsonArea = 0.0;
        int simpsonIntervals = intervals;

        if (simpsonIntervals % 2 != 0) {
            consoleLogs.push_back("Warning: Simpson's Rule requires even intervals. Incrementing " + std::to_string(simpsonIntervals) + " to " + std::to_string(simpsonIntervals + 1) + ".");
            simpsonIntervals++;
        }

        double simpsonDeltaX = (upperBound - lowerBound) / simpsonIntervals;

        for (int i = 0; i < simpsonIntervals; i += 2) {
            double x0 = lowerBound + i * simpsonDeltaX;
            double x1 = lowerBound + (i + 1) * simpsonDeltaX;
            double x2 = lowerBound + (i + 2) * simpsonDeltaX;

            currentX = x0; double y0 = te_eval(expr);
            currentX = x1; double y1 = te_eval(expr);
            currentX = x2; double y2 = te_eval(expr);

            simpsonArea += (y0 + 4.0 * y1 + y2);

            if (i == 0) { simp_stems_X.push_back(x0); simp_stems_Y.push_back(y0); }
            simp_stems_X.push_back(x1); simp_stems_Y.push_back(y1);
            simp_stems_X.push_back(x2); simp_stems_Y.push_back(y2);

            int curveSteps = 20;
            double stepSize = (x2 - x0) / curveSteps;
            for (int j = 0; j <= curveSteps; ++j) {
                double px = x0 + (j * stepSize);

                double L0 = ((px - x1) * (px - x2)) / ((x0 - x1) * (x0 - x2));
                double L1 = ((px - x0) * (px - x2)) / ((x1 - x0) * (x1 - x2));
                double L2 = ((px - x0) * (px - x1)) / ((x2 - x0) * (x2 - x1));
                double py_parabola = (y0 * L0) + (y1 * L1) + (y2 * L2);

                currentX = px; double py_perf = te_eval(expr);

                simp_X.push_back(px);
                simp_Y_approx.push_back(py_parabola);
                simp_Y_perf.push_back(py_perf);
                simp_Y_over.push_back(std::max(py_parabola, py_perf));
                simp_Y_under.push_back(std::min(py_parabola, py_perf));
            }
        }
        simpsonArea *= (simpsonDeltaX / 3.0);

        // ==========================================
        // 4. The "Exact Area" Proxy (100k Simpson's)
        // ==========================================
        int exact_intervals = 100000;
        double exact_deltaX = (upperBound - lowerBound) / exact_intervals;

        currentX = lowerBound; double exactArea = te_eval(expr);
        currentX = upperBound; exactArea += te_eval(expr);

        for (int i = 1; i < exact_intervals; ++i) {
            currentX = lowerBound + (i * exact_deltaX);
            exactArea += (i % 2 == 0 ? 2.0 : 4.0) * te_eval(expr);
        }
        exactArea *= (exact_deltaX / 3.0);

        te_free(expr);

        consoleLogs.push_back("Result (Left Riemann Sum): " + std::to_string(riemannArea));
        consoleLogs.push_back("Result (Trapezoidal Rule): " + std::to_string(trapArea));
        consoleLogs.push_back("Result (Simpson's 1/3 Rule): " + std::to_string(simpsonArea));
        consoleLogs.push_back("Result (Exact Analytical): " + std::to_string(exactArea));
        consoleLogs.push_back("==================================================");
    }
};

extern "C" __declspec(dllexport) IModule* CreateModule() {
    return new NumericalIntegration();
}

#else

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

double RiemannSum(std::function<double(double)> f, double a, double b, int n) {
    double h = (b - a) / n;
    double sum = 0;
    for (int i = 0; i < n; ++i) { sum += f(a + i * h); }
    return sum * h;
}

double TrapezoidRule(std::function<double(double)> f, double a, double b, int n) {
    double h = (b - a) / n;
    double sum = f(a) + f(b);
    for (int i = 1; i < n; ++i) { sum += 2.0 * f(a + i * h); }
    return (h / 2.0) * sum;
}

double SimpsonsRule(std::function<double(double)> f, double a, double b, int n) {
    if (n % 2 != 0) { std::cout << "Warning: Incrementing odd n to " << ++n << "\n"; }
    double h = (b - a) / n;
    double sum = f(a) + f(b);
    for (int i = 1; i < n; ++i) { sum += (i % 2 == 0 ? 2.0 : 4.0) * f(a + i * h); }
    return (h / 3.0) * sum;
}

double f1(double x) { return x * x; }
double f2(double x) { return std::sin(x); }
double f3(double x) { return std::exp(x); }

void RunTest(const std::string& funcName, std::function<double(double)> f, double a, double b, double exactValue) {
    int n_values[] = { 4, 10, 100, 1000 };
    std::cout << "Function: " << funcName << " Interval: [" << a << ", " << b << "]\n";
    std::cout << "Method\t\tn\tResult\t\tExact\t\tError\n";
    std::cout << "--------------------------------------------------------------------\n";

    for (int n : n_values) {
        double r = RiemannSum(f, a, b, n);
        std::cout << "L-Riemann\t" << n << "\t" << std::fixed << std::setprecision(6) << r << "\t" << exactValue << "\t" << std::abs(exactValue - r) << "\n";
    }
    std::cout << "\n";
    for (int n : n_values) {
        double t = TrapezoidRule(f, a, b, n);
        std::cout << "Trapezoid\t" << n << "\t" << std::fixed << std::setprecision(6) << t << "\t" << exactValue << "\t" << std::abs(exactValue - t) << "\n";
    }
    std::cout << "\n";
    for (int n : n_values) {
        double s = SimpsonsRule(f, a, b, n);
        std::cout << "Simpson\t\t" << n << "\t" << std::fixed << std::setprecision(6) << s << "\t" << exactValue << "\t" << std::abs(exactValue - s) << "\n";
    }
    std::cout << "\n====================================================================\n\n";
}

// A helper function that acts as our brute-force Computer Algebra System
double GetExactArea(std::function<double(double)> f, double a, double b) {
    // Run Simpson's Rule with 100,000 intervals for maximum double-precision accuracy
    return SimpsonsRule(f, a, b, 100000);
}

int main() {
    std::cout << "\nNumerical Integration Results\n\n";

    // 1. Calculate the exact answers AT RUNTIME using brute force
    double exact_f1 = GetExactArea(f1, 0.0, 1.0);
    double exact_f2 = GetExactArea(f2, 0.0, M_PI);
    double exact_f3 = GetExactArea(f3, 0.0, 1.0);

    // 2. Run the academic tests against the dynamically calculated exact answers
    RunTest("f(x) = x^2", f1, 0.0, 1.0, exact_f1);
    RunTest("f(x) = sin(x)", f2, 0.0, M_PI, exact_f2);
    RunTest("f(x) = e^x", f3, 0.0, 1.0, exact_f3);

    return 0;
}

#endif