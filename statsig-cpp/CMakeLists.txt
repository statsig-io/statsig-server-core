cmake_minimum_required(VERSION 3.5)

project(Statsig
    VERSION 0.13.0
    DESCRIPTION "Statsig C++ Core SDK"
    LANGUAGES CXX
)
set(PROJECT_VERSION_SUFFIX "rc.2511250016")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(STATSIG_CPP_VERSION "${PROJECT_VERSION}${PROJECT_VERSION_SUFFIX}")
# Wrap the prebuilt dylib as an imported library
file(GLOB STATSIG_SOURCE_FILES "src/*")
add_library(Statsig ${STATSIG_SOURCE_FILES})

# Set include directories
target_include_directories(Statsig PRIVATE 
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# External Dependency
include(FetchContent)
FetchContent_Declare(
    json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG v3.12.0  
)

FetchContent_MakeAvailable(json)
target_link_libraries(Statsig PUBLIC nlohmann_json::nlohmann_json)

# Check if we should use local build file
message(STATUS "CMAKE_BINARY_DIR: ${CMAKE_BINARY_DIR}")

#--------------------- Built Binary Resolution Helper ---------------------#
function (detect_cpu cpu_arch_var)
    set(cpu_arch "${CMAKE_SYSTEM_PROCESSOR}")
    if(cpu_arch MATCHES "x86_64|AMD64")
        set(${cpu_arch_var} "x86_64" PARENT_SCOPE)
    elseif(cpu_arch MATCHES "aarch64|arm64")
        set(${cpu_arch_var} "aarch64" PARENT_SCOPE)
    else()
        message(FATAL_ERROR "Unsupported CPU architecture: ${cpu_arch}, Contact Statsig support to add support for it")
    endif()
endfunction()

function (detect_is_musl is_musl_var)
    execute_process(
        COMMAND ldd --version
        RESULT_VARIABLE _ldd_result
        OUTPUT_VARIABLE _ldd_output
        ERROR_VARIABLE _ldd_error
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_STRIP_TRAILING_WHITESPACE
    )
    if(_ldd_result EQUAL 0)
        if(_ldd_output MATCHES "musl")
            set(${is_musl_var} TRUE PARENT_SCOPE)
        else()
            set(${is_musl_var} FALSE PARENT_SCOPE)
        endif()
    else()
        message(FATAL_ERROR "Failed to determine libc type: ${_ldd_error}")
    endif()
endfunction()

function (detect_os_target os_target_var library_suffix_var)
    set(os_name "${CMAKE_SYSTEM_NAME}")
    if(os_name STREQUAL "Darwin")
        set(${os_target_var} "apple-darwin" PARENT_SCOPE)
        set(${library_suffix_var} "dylib" PARENT_SCOPE)
    elseif(os_name STREQUAL "Linux")
        set(${library_suffix_var} "so" PARENT_SCOPE)
        detect_is_musl(_is_musl)
        if(_is_musl)
            set(${os_target_var} "linux-musl" PARENT_SCOPE)
        else()
            set(${os_target_var} "linux-gnu" PARENT_SCOPE)
        endif()
    else()
        message(FATAL_ERROR "Unsupported OS: ${os_name}, Contact Statsig support to add support for it")
    endif()
endfunction()

function(generate_target_name version target_var suffix_var)
    # Detect OS
    detect_os_target(os_target suffix)
    message(STATUS "within generate target name and ${os_target}, ${suffix_var}")
    detect_cpu(cpu_target)
   
    # Compose target name
    set(target_name "statsig-ffi-${version}-${cpu_target}-${os_target}.zip")

    # Return value to caller
    set(${target_var} "${target_name}" PARENT_SCOPE)
    set(${suffix_var} "${suffix}" PARENT_SCOPE)
endfunction()
#--------------------- End of Helper ---------------------#

if(USE_LOCAL_BUILD_FILE)
    # Get the FFI binary path from environment variable
    set(FFI_BINARY_PATH $ENV{FFI_BINARY_PATH})
    if(NOT FFI_BINARY_PATH)
        message(WARN "USE_LOCAL_BUILD_FILE is true but FFI_BINARY_PATH environment variable is not set, using default")
        detect_os_target(_ os_library_suffix)
        set(FFI_BINARY_PATH "${CMAKE_CURRENT_SOURCE_DIR}/../target/release/libstatsig_ffi.${os_library_suffix}")
    endif()
    message(STATUS "Using local build file from: ${FFI_BINARY_PATH}")
    target_link_libraries(Statsig PRIVATE ${FFI_BINARY_PATH})
else()
    # Adjust asset name according to your OS/arch
    generate_target_name(${STATSIG_CPP_VERSION} ASSET_NAME SUFFIX_VAR)
    set(ASSET_URL "https://github.com/statsig-io/statsig-cpp-core/releases/download/${STATSIG_CPP_VERSION}/${ASSET_NAME}")
    set(ASSET_PATH "${ASSET_NAME}")

    message(STATUS "Downloading asset from: ${ASSET_URL}")
    file(DOWNLOAD
        "${ASSET_URL}"
        "${ASSET_PATH}"
    )
    message(STATUS "Extracting ${ASSET_NAME} to ${CMAKE_BINARY_DIR}")
    execute_process(
        COMMAND ${CMAKE_COMMAND} -E tar xvf  "${ASSET_PATH}"
        WORKING_DIRECTORY "${CMAKE_BINARY_DIR}"
    )
    target_link_libraries(Statsig PRIVATE "${CMAKE_BINARY_DIR}/libstatsig_ffi.${SUFFIX_VAR}")

    set_target_properties(Statsig PROPERTIES
        BUILD_RPATH "@loader_path"
        INSTALL_RPATH "@loader_path"
    )
endif()


if(ENABLE_STATSIG_CPP_UNIT_TEST)
    enable_testing()
    add_subdirectory(tests)
endif()
