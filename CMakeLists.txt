# Project setup.

cmake_minimum_required(VERSION 3.13.4)

project(egg-netlist-synthesizer)

set(version 0.1.0)

set(CMAKE_CXX_STANDARD 17)

# Generate and compile a library.

if (NOT DEFINED CBC_SOLVER_LIB)
  message(FATAL_ERROR "Define CBC_SOLVER_LIB as the path to libCbcSolver.")
endif()

set(CARGO_CMD cargo build --release)
set(TARGET_DIR "release")
set(RUST_GEN_LIB "${CMAKE_CURRENT_BINARY_DIR}/${TARGET_DIR}/libegg_netlist_synthesizer.a")
set(RUST_GEN_H "${CMAKE_CURRENT_BINARY_DIR}/egg_netlist_synthesizer.h")
set(RUST_GEN_CXX "${CMAKE_CURRENT_BINARY_DIR}/egg_netlist_synthesizer.cpp")

add_library(egg_netlist_synthesizer STATIC ${RUST_GEN_CXX})

target_link_libraries(egg_netlist_synthesizer pthread dl ${CBC_SOLVER_LIB} ${RUST_GEN_LIB})

add_custom_command(
  OUTPUT ${RUST_GEN_CXX}
  COMMAND CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD}
  COMMAND cp ${CMAKE_CURRENT_BINARY_DIR}/cxxbridge/egg-netlist-synthesizer/src/lib.rs.h ${RUST_GEN_H}
  COMMAND cp ${CMAKE_CURRENT_BINARY_DIR}/cxxbridge/egg-netlist-synthesizer/src/lib.rs.cc ${RUST_GEN_CXX}
  WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Install the generated header and library.

install(FILES ${RUST_GEN_H} DESTINATION include/egg-netlist-synthesizer-${version})
install(TARGETS egg_netlist_synthesizer
        DESTINATION lib/egg-netlist-synthesizer-${version}
        EXPORT egg-netlist-synthesizer-targets)
install(EXPORT egg-netlist-synthesizer-targets
        DESTINATION lib/egg-netlist-synthesizer-${version})

# Create and install package configuration and version files.

configure_file(
   ${CMAKE_CURRENT_SOURCE_DIR}/pkg/egg-netlist-synthesizer-config.cmake.in
   ${CMAKE_CURRENT_BINARY_DIR}/pkg/egg-netlist-synthesizer-config.cmake @ONLY)

configure_file(
   ${CMAKE_CURRENT_SOURCE_DIR}/pkg/egg-netlist-synthesizer-config-version.cmake.in
   ${CMAKE_CURRENT_BINARY_DIR}/pkg/egg-netlist-synthesizer-config-version.cmake @ONLY)

install(FILES
         ${CMAKE_CURRENT_BINARY_DIR}/pkg/egg-netlist-synthesizer-config.cmake
         ${CMAKE_CURRENT_BINARY_DIR}/pkg/egg-netlist-synthesizer-config-version.cmake
       DESTINATION lib/egg-netlist-synthesizer-${version})
