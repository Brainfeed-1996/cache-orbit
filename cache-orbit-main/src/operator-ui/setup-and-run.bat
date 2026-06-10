@echo off
setlocal enabledelayedexpansion
echo ==========================================
echo   Cache Orbit - UI Setup
echo ==========================================
echo.

REM Vérifier Node.js
echo [1/3] Verifying Node.js installation...
node --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Node.js not found. Please install Node.js 20+ from https://nodejs.org
    pause
    exit /b 1
)
echo ✓ Node.js detected
echo.

REM Vérifier npm
echo [2/3] Verifying npm installation...
npm --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: npm not found. Please install Node.js 20+ (includes npm)
    pause
    exit /b 1
)
echo ✓ npm detected
echo.

REM Installer les dépendances
echo [3/3] Installing UI dependencies...
echo This may take a few minutes on first run...
echo.
call npm install
if errorlevel 1 (
    echo.
    echo ERROR: npm install failed. Please check your Node.js installation.
    pause
    exit /b 1
)
echo.
echo ==========================================
echo   Setup complete!
echo ==========================================
echo.
echo Starting development server...
echo Open http://localhost:3000 in your browser
echo.
call npm run dev
pause
