#include <windows.h>

// IL2CPP raises managed exceptions as MSVC C++ exceptions. If one unwinds into Rust,
// the process aborts (catch_unwind cannot handle foreign exceptions, and extern "C"
// Rust frames abort on any unwind). This calls an IL2CPP method with no Rust frames
// between the throw and the __except, so the exception is caught here instead.

#define MSVC_CPP_EXCEPTION 0xE06D7363

static int capture(EXCEPTION_POINTERS *info, unsigned long *code, void **il2cpp_exception)
{
    EXCEPTION_RECORD *record = info->ExceptionRecord;
    *code = record->ExceptionCode;
    *il2cpp_exception = NULL;
    // For C++ exceptions, ExceptionInformation[1] points at the thrown object. IL2CPP
    // throws Il2CppExceptionWrapper, whose first field is the managed Il2CppException*.
    if (record->ExceptionCode == MSVC_CPP_EXCEPTION && record->NumberParameters >= 3 &&
        record->ExceptionInformation[1] != 0)
    {
        *il2cpp_exception = *(void **)record->ExceptionInformation[1];
    }
    return EXCEPTION_EXECUTE_HANDLER;
}

typedef void *(*il2cpp_static_fn2)(void *, void *);

// Returns 0 and writes *result on success; returns 1 and fills code/il2cpp_exception if
// the call raised any SEH or C++ exception.
int veritas_guarded_call2(il2cpp_static_fn2 fn, void *arg0, void *arg1, void **result,
                          unsigned long *code, void **il2cpp_exception)
{
    __try
    {
        *result = fn(arg0, arg1);
        return 0;
    }
    __except (capture(GetExceptionInformation(), code, il2cpp_exception))
    {
        return 1;
    }
}
