# Additional clean files
cmake_minimum_required(VERSION 3.16)

if("${CONFIG}" STREQUAL "" OR "${CONFIG}" STREQUAL "")
  file(REMOVE_RECURSE
  "CMakeFiles/Omnibox_autogen.dir/AutogenUsed.txt"
  "CMakeFiles/Omnibox_autogen.dir/ParseCache.txt"
  "Omnibox_autogen"
  )
endif()
