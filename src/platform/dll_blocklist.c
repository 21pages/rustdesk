/**
 * DLL Blocklist Implementation for RustDesk
 *
 * Blocks problematic DLLs from being loaded into the RustDesk process by hooking
 * NtMapViewOfSection using Microsoft Detours. This prevents crashes caused by
 * third-party software (like Astrill VPN's ASProxy64.dll) that inject DLLs which
 * interfere with RustDesk's operation.
 *
 * Inspired by OBS Studio's DLL blocklist implementation.
 */

#include <Windows.h>
#include <psapi.h>
#include <stdint.h>
#include <stdbool.h>
#include <detours/detours.h>

/* NT status codes */
#define STATUS_SUCCESS ((NTSTATUS)0x00000000L)
#define STATUS_DLL_NOT_FOUND ((NTSTATUS)0xC0000135L)

/* Section information class for NtQuerySection */
#define SectionBasicInformation 0

/* Section flags */
#define SEC_IMAGE 0x1000000

/* Timestamp comparison methods */
#define TS_IGNORE 0     /* Ignore timestamp, always block */
#define TS_EQUAL 1      /* Block if timestamp equals */
#define TS_LESS_THAN 2  /* Block if timestamp is less than */

/* Section basic information structure */
typedef struct _SECTION_BASIC_INFORMATION {
    PVOID BaseAddress;
    ULONG Attributes;
    LARGE_INTEGER Size;
} SECTION_BASIC_INFORMATION, *PSECTION_BASIC_INFORMATION;

/* NT function signatures */
typedef NTSTATUS(STDMETHODCALLTYPE *fn_NtMapViewOfSection)(
    HANDLE SectionHandle,
    HANDLE ProcessHandle,
    PVOID *BaseAddress,
    ULONG_PTR ZeroBits,
    SIZE_T CommitSize,
    PLARGE_INTEGER SectionOffset,
    PSIZE_T ViewSize,
    ULONG InheritDisposition,
    ULONG AllocationType,
    ULONG Win32Protect);

typedef NTSTATUS(STDMETHODCALLTYPE *fn_NtUnmapViewOfSection)(
    HANDLE ProcessHandle,
    PVOID BaseAddress);

typedef NTSTATUS(STDMETHODCALLTYPE *fn_NtQuerySection)(
    HANDLE SectionHandle,
    ULONG SectionInformationClass,
    PVOID SectionInformation,
    SIZE_T SectionInformationLength,
    PSIZE_T ReturnLength);

/* Original function pointers */
static fn_NtMapViewOfSection ntMap = NULL;
static fn_NtUnmapViewOfSection ntUnmap = NULL;
static fn_NtQuerySection ntQuery = NULL;

/* Blocked module entry */
typedef struct {
    const wchar_t *name;
    size_t name_len;
    const uint32_t timestamp;
    const int method;
    volatile LONG64 blocked_count;
} blocked_module_t;

/* List of blocked DLLs */
static blocked_module_t blocked_modules[] = {
    /* Astrill VPN Proxy - injects and causes access violation crashes */
    {L"\\asproxy64.dll", 0, 0, TS_IGNORE, 0},
};

static const size_t blocked_modules_count = sizeof(blocked_modules) / sizeof(blocked_modules[0]);

/* Check if a filename matches a blocked module */
static bool is_blocked(const wchar_t *filename, size_t filename_len)
{
    for (size_t i = 0; i < blocked_modules_count; i++) {
        blocked_module_t *mod = &blocked_modules[i];

        /* Initialize name_len on first use */
        if (mod->name_len == 0) {
            mod->name_len = wcslen(mod->name);
        }

        if (filename_len < mod->name_len) {
            continue;
        }

        /* Compare end of filename (case-insensitive) */
        const wchar_t *suffix = filename + filename_len - mod->name_len;
        if (_wcsicmp(suffix, mod->name) == 0) {
            /* For TS_IGNORE, always block */
            if (mod->method == TS_IGNORE) {
                InterlockedIncrement64(&mod->blocked_count);
                return true;
            }
            /* TODO: Add timestamp checking for TS_EQUAL and TS_LESS_THAN if needed */
        }
    }
    return false;
}

/* Hook function for NtMapViewOfSection */
static NTSTATUS STDMETHODCALLTYPE NtMapViewOfSection_hook(
    HANDLE SectionHandle,
    HANDLE ProcessHandle,
    PVOID *BaseAddress,
    ULONG_PTR ZeroBits,
    SIZE_T CommitSize,
    PLARGE_INTEGER SectionOffset,
    PSIZE_T ViewSize,
    ULONG InheritDisposition,
    ULONG AllocationType,
    ULONG Win32Protect)
{
    /* Call original function first */
    NTSTATUS ret = ntMap(SectionHandle, ProcessHandle, BaseAddress, ZeroBits,
                         CommitSize, SectionOffset, ViewSize, InheritDisposition,
                         AllocationType, Win32Protect);

    /* Only proceed if successful and targeting current process */
    if (ret < 0 || ProcessHandle != GetCurrentProcess()) {
        return ret;
    }

    /* Check if this is an image section (DLL/EXE) */
    SECTION_BASIC_INFORMATION sbi;
    SIZE_T returnLength;
    NTSTATUS queryRet = ntQuery(SectionHandle, SectionBasicInformation, &sbi,
                                sizeof(sbi), &returnLength);

    if (queryRet < 0 || !(sbi.Attributes & SEC_IMAGE)) {
        return ret;
    }

    /* Get the mapped filename */
    wchar_t filename[MAX_PATH];
    DWORD filenameLen = K32GetMappedFileNameW(GetCurrentProcess(), *BaseAddress,
                                               filename, MAX_PATH);

    if (filenameLen == 0) {
        return ret;
    }

    /* Check if this DLL should be blocked */
    if (is_blocked(filename, filenameLen)) {
        /* Unmap the section */
        ntUnmap(ProcessHandle, *BaseAddress);
        *BaseAddress = NULL;
        return STATUS_DLL_NOT_FOUND;
    }

    return ret;
}

/**
 * Install the DLL blocklist hook.
 * Should be called early in process initialization, before problematic DLLs load.
 */
void install_dll_blocklist_hook(void)
{
    HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");
    if (!ntdll) {
        return;
    }

    ntMap = (fn_NtMapViewOfSection)GetProcAddress(ntdll, "NtMapViewOfSection");
    ntUnmap = (fn_NtUnmapViewOfSection)GetProcAddress(ntdll, "NtUnmapViewOfSection");
    ntQuery = (fn_NtQuerySection)GetProcAddress(ntdll, "NtQuerySection");

    if (!ntMap || !ntUnmap || !ntQuery) {
        return;
    }

    DetourTransactionBegin();
    DetourUpdateThread(GetCurrentThread());

    if (DetourAttach((PVOID *)&ntMap, NtMapViewOfSection_hook) != NO_ERROR) {
        DetourTransactionAbort();
        return;
    }

    DetourTransactionCommit();
}

/**
 * Log information about blocked DLLs.
 * Can be called to get statistics about blocked modules.
 */
void log_blocked_dlls(void)
{
    for (size_t i = 0; i < blocked_modules_count; i++) {
        LONG64 count = InterlockedCompareExchange64(&blocked_modules[i].blocked_count, 0, 0);
        if (count > 0) {
            /* Logging would go here - for now we just track the counts */
            /* The Rust side can call this and check counts if needed */
        }
    }
}
