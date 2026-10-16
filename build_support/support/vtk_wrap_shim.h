#pragma once

#include <cstddef>
#include <cstdint>

extern "C" {
// Reference-counting helpers shared by every generated translation unit.
void vtk_wrap_register(void* obj) noexcept;
void vtk_wrap_delete(void* obj) noexcept;

// Heap string helpers for `std::string` returns.
char* vtk_wrap_strdup(const char* s) noexcept;
void vtk_wrap_free(void* p) noexcept;
}
