# Can also run without configuring/building the SDK:
# cmake -P statsig-cpp/tests/version_metadata_test.cmake
get_filename_component(STATSIG_RELEASE_ROOT "${CMAKE_CURRENT_LIST_DIR}/../.." ABSOLUTE)
file(STRINGS "${STATSIG_RELEASE_ROOT}/Cargo.toml" ROOT_VERSION_LINE
    REGEX "^version = ")
string(REGEX REPLACE "^version = \"([^\"]+)\".*" "\\1"
    ROOT_VERSION "${ROOT_VERSION_LINE}")

# This regression covers stable promotion; prerelease naming is independent.
if(ROOT_VERSION MATCHES "-")
    message(STATUS "Skipping stable C++ metadata check for ${ROOT_VERSION}")
    return()
endif()

file(READ "${STATSIG_RELEASE_ROOT}/statsig-cpp/CMakeLists.txt" CPP_CMAKE)
string(REGEX MATCH "VERSION[ \t\r\n]+([0-9]+\\.[0-9]+\\.[0-9]+)"
    CPP_VERSION_MATCH "${CPP_CMAKE}")
set(CPP_VERSION "${CMAKE_MATCH_1}")
string(REGEX MATCH "set\\(PROJECT_VERSION_SUFFIX[ \t]+\"([^\"]*)\"\\)"
    CPP_SUFFIX_MATCH "${CPP_CMAKE}")
set(CPP_SUFFIX "${CMAKE_MATCH_1}")

if(NOT CPP_VERSION_MATCH OR NOT CPP_SUFFIX_MATCH)
    message(FATAL_ERROR "Could not read C++ release version metadata")
endif()
if(NOT CPP_VERSION STREQUAL ROOT_VERSION OR NOT CPP_SUFFIX STREQUAL "")
    message(FATAL_ERROR
        "Stable C++ release must resolve assets at ${ROOT_VERSION}, got ${CPP_VERSION}${CPP_SUFFIX}")
endif()
message(STATUS "Stable C++ native asset version matches ${ROOT_VERSION}")
