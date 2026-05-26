cmake_minimum_required(VERSION 3.16)
project(ScreenshotTranslator VERSION 1.0 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../release")

# 寻找 Qt 库 (同时支持 Qt5 和 Qt6 极高兼容性)
find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Core Gui Widgets Network)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Core Gui Widgets Network)

add_executable(ScreenshotTranslator WIN32
    main.cpp
    config.h config.cpp
    networkclient.h networkclient.cpp
    localocrmanager.h localocrmanager.cpp
    screenshotwindow.h screenshotwindow.cpp
    settingspanel.h settingspanel.cpp
    resource.rc
    resources.qrc
)

target_link_libraries(ScreenshotTranslator PRIVATE
    Qt${QT_VERSION_MAJOR}::Core
    Qt${QT_VERSION_MAJOR}::Gui
    Qt${QT_VERSION_MAJOR}::Widgets
    Qt${QT_VERSION_MAJOR}::Network
)
