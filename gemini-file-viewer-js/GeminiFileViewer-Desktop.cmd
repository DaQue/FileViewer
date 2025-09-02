@echo off
setlocal
rem Desktop launcher that always runs from the project directory
set "APP_DIR=C:\Users\david\Documents\rust\gemini-file-viewer\gemini-file-viewer-js"
if not exist "%APP_DIR%\package.json" (
  echo Project directory not found: %APP_DIR%
  echo Please edit this file and update APP_DIR to your repo path.
  pause
  exit /b 1
)
pushd "%APP_DIR%"

if exist "dist\GeminiFileViewer-Portable-0.1.0.exe" (
  start "" "%APP_DIR%\dist\GeminiFileViewer-Portable-0.1.0.exe"
) else if exist "node_modules\.bin\electron.cmd" (
  call "node_modules\.bin\electron.cmd" .
) else (
  echo Electron is not installed locally. Attempting to use npx...
  npx --yes electron@^30.0.0 .
)

popd
