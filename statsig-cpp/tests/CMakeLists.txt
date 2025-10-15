cmake_minimum_required(VERSION 3.11)

include(FetchContent)

FetchContent_Declare(googletest
  GIT_REPOSITORY    https://github.com/google/googletest.git
  GIT_TAG           v1.12.0
)
# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

unset(CMAKE_MODULE_PATH CACHE)

include(GoogleTest)
enable_testing()

file(GLOB STATSIG_TEST_FILES "${PROJECT_SOURCE_DIR}/tests/*")

add_executable(google_tests
  ${STATSIG_SOURCE_FILES}
  ${STATSIG_TEST_FILES}
)

message(STATUS "🐾 PROJECT_SOURCE_DIR = ${PROJECT_SOURCE_DIR}")
message(STATUS "🐾 CMAKE_CURRENT_SOURCE_DIR = ${CMAKE_CURRENT_SOURCE_DIR}")
message(STATUS "🐾 STATSIG_SOURCE_FILES = ${STATSIG_SOURCE_FILES}")
message(STATUS "🐾 STATSIG_TEST_FILES = ${STATSIG_TEST_FILES}")


target_include_directories(google_tests PRIVATE "../include")
target_include_directories(google_tests PRIVATE "../src")

target_link_libraries(google_tests Statsig)
target_link_libraries(google_tests GTest::gtest_main)
target_compile_features(google_tests PUBLIC cxx_std_17)

add_definitions( -DTESTS_ROOT_DIR="${CMAKE_CURRENT_SOURCE_DIR}" )

gtest_discover_tests(google_tests)