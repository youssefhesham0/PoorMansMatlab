// ==============================================================================
// Numerical Integration Assignment
// Student Name: Youssef hesham
// Student ID: 263381
//
// Architecture: Shape-Shifting C++ (Poor Man's MATLAB Module, ignore unless you want to try the full framework)
// ==============================================================================

#include <iostream>
#define _USE_MATH_DEFINES
#include <cmath>
#include <cmath>
#include <iomanip>
#include <functional>
#include <string>

// ------------------------------------------------------------------------------
// 1. THE CORE MATH ENGINE (Shared across all realities)
// ------------------------------------------------------------------------------

// Task 1: Trapezoid Rule Function
// Uses std::function so we can pass our mathematical equations directly as arguments (instead of like 3 functions for x^3)
double TrapezoidRule(std::function<double(double)> f, double a, double b, int n) {
    double h = (b - a) / n;
    double sum = f(a) + f(b); // Add the first and last terms

    for (int i = 1; i < n; ++i) {
        double x = a + i * h;
        sum += 2.0 * f(x);    // Inner terms are multiplied by 2
    }
    return (h / 2.0) * sum;
}

// Task 2: Simpson's Rule Function
double SimpsonsRule(std::function<double(double)> f, double a, double b, int n) {
    // Enforce that n must be even
    if (n % 2 != 0) {
        std::cout << "[WARNING] n must be even for Simpson's Rule. Incrementing " << n << " to " << n + 1 << ".\n";
        n++;
    }

    double h = (b - a) / n;
    double sum = f(a) + f(b); // Add the first and last terms

    for (int i = 1; i < n; ++i) {
        double x = a + i * h;
        if (i % 2 == 0) {
            sum += 2.0 * f(x); // Even index gets coefficient 2
        }
        else {
            sum += 4.0 * f(x); // Odd index gets coefficient 4
        }
    }
    return (h / 3.0) * sum;
}

// Task 3: Test Functions
double f1(double x) { return x * x; }       // f(x) = x^2
double f2(double x) { return std::sin(x); } // f(x) = sin(x)
double f3(double x) { return std::exp(x); } // f(x) = e^x

// ------------------------------------------------------------------------------
// 2. THE ENVIRONMENTAL RADAR (The Framework Switch)
// ------------------------------------------------------------------------------

#if __has_include("../../include/IModule.h") && __has_include("imgui.h")

    // === POOR MAN'S MATLAB REALITY ===
    // If the compiler sees our framework headers, delete main() and become a DLL.

#include "../../include/IModule.h"
#include "imgui.h"

extern "C" __declspec(dllexport) IModule* CreateModule() {
    // We will build the actual GUI binding structure here later.
    // For now, it just returns a module hook.
    return new IntegrationModule();
}

#else

    // === ACADEMIC REALITY ===
    // If the compiler is the TA's Visual Studio, compile standard CLI program.

    // Helper function to keep the main() clean and print the required tables
void RunTest(const std::string& funcName, std::function<double(double)> f, double a, double b, double exactValue) {
    int n_values[] = { 4, 10, 100, 1000 };

    std::cout << "Function: " << funcName << " Interval: [" << a << ", " << b << "]\n\n";
    std::cout << "Method\t\tn\tResult\t\tExact\t\tError\n";
    std::cout << "--------------------------------------------------------------------\n";

    // Run Trapezoid
    for (int n : n_values) {
        double trapResult = TrapezoidRule(f, a, b, n);
        double trapError = std::abs(exactValue - trapResult);
        std::cout << "Trapezoid\t" << n << "\t" << std::fixed << std::setprecision(6)
            << trapResult << "\t" << exactValue << "\t" << trapError << "\n";
    }

    std::cout << "\n";

    // Run Simpson's
    for (int n : n_values) {
        double simpResult = SimpsonsRule(f, a, b, n);
        double simpError = std::abs(exactValue - simpResult);
        std::cout << "Simpson\t\t" << n << "\t" << std::fixed << std::setprecision(6)
            << simpResult << "\t" << exactValue << "\t" << simpError << "\n";
    }
    std::cout << "\n====================================================================\n\n";
}

int main() {
    std::cout << "\nNumerical Integration Results\n\n";

    // Test 1: x^2 on [0, 1]
    RunTest("f(x) = x^2", f1, 0.0, 1.0, 1.0 / 3.0);

    // Test 2: sin(x) on [0, pi]
    RunTest("f(x) = sin(x)", f2, 0.0, std::acos(-1.0), 2.0);

    // Test 3: e^x on [0, 1]
    RunTest("f(x) = e^x", f3, 0.0, 1.0, std::exp(1.0) - 1.0);

    return 0;
}

#endif