cmake_minimum_required(VERSION 3.28)
project(Statsig)

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
if(USE_LOCAL_BUILD_FILE)
    # Get the FFI binary path from environment variable
    set(FFI_BINARY_PATH $ENV{FFI_BINARY_PATH})
    if(NOT FFI_BINARY_PATH)
        message(WARN "USE_LOCAL_BUILD_FILE is true but FFI_BINARY_PATH environment variable is not set, using default")
        set(FFI_BINARY_PATH "${CMAKE_CURRENT_SOURCE_DIR}/../target/release/libstatsig_ffi.dylib")
    endif()
    message(STATUS "Using local build file from: ${FFI_BINARY_PATH}")
    target_link_libraries(Statsig PRIVATE ${FFI_BINARY_PATH})
else()
    # (TODO: xinli)Need to dynamically download from GITHUB 
    message(STATUS "Current source dir: ${CMAKE_CURRENT_SOURCE_DIR}")
    target_link_libraries(Statsig PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/target/libstatsig_ffi.dylib)
endif()


if(ENABLE_STATSIG_CPP_UNIT_TEST)
    enable_testing()
    add_subdirectory(tests)
endif()

