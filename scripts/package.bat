@echo off
setlocal

set "ROOT=%~dp0.."
cd /d "%ROOT%"

if defined RUST_TARGET (
  set "TARGET=%RUST_TARGET%"
) else (
  set "TARGET=x86_64-pc-windows-gnu"
)

set "NATIVE_DIR=%ROOT%\target\%TARGET%\release"
if defined RUST_TARGET (
  where rustup >nul 2>nul
  if errorlevel 1 (
    echo rustup not found >&2
    exit /b 2
  )
  call rustup target add "%TARGET%"
  pushd "%ROOT%"
  call cargo build --release --target "%TARGET%"
  if errorlevel 1 exit /b 1
  popd
) else if exist "%ROOT%\target\%TARGET%" (
  where rustup >nul 2>nul
  if errorlevel 1 (
    echo rustup not found >&2
    exit /b 2
  )
  call rustup target add "%TARGET%"
  pushd "%ROOT%"
  call cargo build --release --target "%TARGET%"
  if errorlevel 1 exit /b 1
  popd
) else (
  pushd "%ROOT%"
  call cargo build --release
  if errorlevel 1 exit /b 1
  popd
  set "NATIVE_DIR=%ROOT%\target\release"
)

if defined GUI_JAVA_HOME (
  set "JAVA_HOME=%GUI_JAVA_HOME%"
) else (
  set "JAVA_HOME=C:\Program Files\Java\jdk-21"
)
set "PATH=%JAVA_HOME%\bin;%PATH%"

pushd "%ROOT%\gui"
call mvn -q -DskipTests package -Daudio.engine.native.dir="%NATIVE_DIR%"
if errorlevel 1 exit /b 1
popd

set "DIST=%ROOT%\gui\target\audio-engine-gui-0.1.0-dist"
set "OUT=%ROOT%\dist"
if exist "%OUT%" rmdir /s /q "%OUT%"
if not exist "%OUT%" mkdir "%OUT%"

if defined PACKAGE_TYPE (
  set "TYPE=%PACKAGE_TYPE%"
) else (
  set "TYPE=exe"
)

"%JAVA_HOME%\bin\jpackage" ^
  --type "%TYPE%" ^
  --input "%DIST%" ^
  --dest "%OUT%" ^
  --name "AudioEngine" ^
  --app-version "0.1.0" ^
  --main-jar "audio-engine-gui-0.1.0.jar" ^
  --main-class "com.losshifi.audioengine.Main" ^
  --module-path "%DIST%" ^
  --add-modules "javafx.controls" ^
  --java-options "-Dprism.order=sw"

echo package output: %OUT%
endlocal
