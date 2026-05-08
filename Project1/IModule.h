#pragma once
#include <string>

// ==============================================================================
// POOR MAN'S MATLAB - THE INTERFACE CONTRACT
// Every math module MUST inherit from this class and guarantee it can 
// answer these exact functions, otherwise the Host Engine will reject it.
// ==============================================================================

class IModule {
public:
    // A virtual destructor ensures memory is cleaned up properly when we hot-swap modules
    virtual ~IModule() = default;

    // 1. Identity: What is the name of this module? (e.g., "Numerical Integration")
    virtual std::string GetName() = 0;

    // 2. The UI: The module will use this space to draw its own sliders and graphs
    virtual void RenderUI() = 0;

    // 3. The Math: The actual heavy lifting calculations
    virtual void ExecuteMath() = 0;
};

// ------------------------------------------------------------------------------
// THE EXPORT HOOK
// This defines the exact shape of the function we will use to "hook" the DLL 
// out of the ether and pull it into our Host Engine's RAM.
// ------------------------------------------------------------------------------
typedef IModule* (*CreateModuleFunc)();