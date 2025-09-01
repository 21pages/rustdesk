vcpkg_check_linkage(ONLY_STATIC_LIBRARY)

vcpkg_from_git(
    OUT_SOURCE_PATH SOURCE_PATH
    URL https://chromium.googlesource.com/libyuv/libyuv
    REF a37e6bc81b52d39cdcfd0f1428f5d6c2b2bc9861 # 1896 Fixes build error on macOS Homebrew LLVM 19
    # Check https://chromium.googlesource.com/libyuv/libyuv/+/refs/heads/main/include/libyuv/version.h for a version!
    PATCHES
        cmake.diff
)

vcpkg_cmake_get_vars(cmake_vars_file)
include("${cmake_vars_file}")
if (VCPKG_DETECTED_CMAKE_CXX_COMPILER_ID STREQUAL "MSVC" AND NOT VCPKG_TARGET_IS_UWP)
    # Most of libyuv accelerated features need to be compiled by clang/gcc, so force use clang-cl, otherwise the performance is too poor.
    # Manually build the port with clang-cl when using MSVC as compiler

    message(STATUS "Set compiler to clang-cl when using MSVC")

    # https://github.com/microsoft/vcpkg/pull/10398
    set(VCPKG_POLICY_SKIP_ARCHITECTURE_CHECK enabled)

    vcpkg_find_acquire_program(CLANG)
    if (CLANG MATCHES "-NOTFOUND")
        message(FATAL_ERROR "Clang is required.")
    endif ()
    get_filename_component(CLANG "${CLANG}" DIRECTORY)

    if(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm")
        set(CLANG_TARGET "arm")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
        set(CLANG_TARGET "aarch64")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "x86")
        set(CLANG_TARGET "i686")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
        set(CLANG_TARGET "x86_64")
    else()
        message(FATAL_ERROR "Unsupported target architecture")
    endif()

    set(CLANG_TARGET "${CLANG_TARGET}-pc-windows-msvc")

    message(STATUS "Using clang target ${CLANG_TARGET}")
    string(APPEND VCPKG_DETECTED_CMAKE_CXX_FLAGS --target=${CLANG_TARGET})
    string(APPEND VCPKG_DETECTED_CMAKE_C_FLAGS --target=${CLANG_TARGET})

    set(BUILD_OPTIONS
            -DCMAKE_CXX_COMPILER=${CLANG}/clang-cl.exe
            -DCMAKE_C_COMPILER=${CLANG}/clang-cl.exe
            -DCMAKE_CXX_FLAGS=${VCPKG_DETECTED_CMAKE_CXX_FLAGS}
            -DCMAKE_C_FLAGS=${VCPKG_DETECTED_CMAKE_C_FLAGS})
endif ()

vcpkg_cmake_configure(
    SOURCE_PATH ${SOURCE_PATH}
    DISABLE_PARALLEL_CONFIGURE
    OPTIONS
        ${BUILD_OPTIONS}
    OPTIONS_DEBUG
        -DCMAKE_DEBUG_POSTFIX=d
)

vcpkg_cmake_install()
vcpkg_cmake_config_fixup()

file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/include")
file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/share")

file(COPY "${CMAKE_CURRENT_LIST_DIR}/libyuv-config.cmake" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")
file(COPY "${CMAKE_CURRENT_LIST_DIR}/usage" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")

vcpkg_cmake_get_vars(cmake_vars_file)
include("${cmake_vars_file}")
if(VCPKG_DETECTED_CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
    file(APPEND "${CURRENT_PACKAGES_DIR}/share/${PORT}/usage" [[

Attention:
You are using MSVC to compile libyuv. This build won't compile any
of the acceleration codes, which results in a very slow library.
See workarounds: https://github.com/microsoft/vcpkg/issues/28446
]])
endif()

vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")
