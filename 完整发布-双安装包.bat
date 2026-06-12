@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "ROOT=%~dp0"
set "NO_PAUSE="
set "DRY_RUN="
set "FORWARD_ARGS=--no-launch"

call :parse_args %*
if defined NO_PAUSE set "FORWARD_ARGS=--no-launch --no-pause"

echo(=== YSN Trans - full release build ===
echo(
echo([cmd] call "%ROOT%build.bat" %FORWARD_ARGS%
echo(

if defined DRY_RUN goto :done

call "%ROOT%build.bat" %FORWARD_ARGS%
set "BUILD_CODE=%errorlevel%"

if not "%BUILD_CODE%"=="0" (
  echo(
  echo([FAIL] Full release build failed with code: %BUILD_CODE%
  exit /b %BUILD_CODE%
)

goto :done

:parse_args
if "%~1"=="" exit /b 0
if /I "%~1"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="/no-pause" set "NO_PAUSE=1"
if /I "%~1"=="--dry-run" set "DRY_RUN=1"
if /I "%~1"=="/dry-run" set "DRY_RUN=1"
shift
goto :parse_args

:done
echo([done] Full release command finished.
if not defined NO_PAUSE pause
exit /b 0
