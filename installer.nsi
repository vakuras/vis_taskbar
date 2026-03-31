!include "MUI2.nsh"

Name "vis_taskbar"
OutFile "vis_taskbar-0.5.0-setup.exe"
InstallDir "$LOCALAPPDATA\vis_taskbar"
InstallDirRegKey HKCU "Software\vis_taskbar" "InstallDir"
RequestExecutionLevel user

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\vis_taskbar.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch vis_taskbar"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
    SetOutPath "$INSTDIR"
    File "target\release\vis_taskbar.exe"
    File "icon.ico"
    File "README.md"
    File "LICENSE"
    File "CHANGELOG.md"

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start menu shortcut
    CreateDirectory "$SMPROGRAMS\vis_taskbar"
    CreateShortCut "$SMPROGRAMS\vis_taskbar\vis_taskbar.lnk" "$INSTDIR\vis_taskbar.exe" "" "$INSTDIR\icon.ico"
    CreateShortCut "$SMPROGRAMS\vis_taskbar\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    ; Startup shortcut (optional — run on login)
    CreateShortCut "$SMSTARTUP\vis_taskbar.lnk" "$INSTDIR\vis_taskbar.exe" "" "$INSTDIR\icon.ico"

    ; Registry
    WriteRegStr HKCU "Software\vis_taskbar" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar" "DisplayName" "vis_taskbar"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar" "DisplayIcon" "$INSTDIR\icon.ico"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar" "Publisher" "Vadim Kuras"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar" "DisplayVersion" "0.5.0"
SectionEnd

Section "Uninstall"
    ; Kill running instance
    nsExec::ExecToLog 'taskkill /F /IM vis_taskbar.exe'

    Delete "$INSTDIR\vis_taskbar.exe"
    Delete "$INSTDIR\icon.ico"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\CHANGELOG.md"
    Delete "$INSTDIR\vis_taskbar.toml"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\vis_taskbar\vis_taskbar.lnk"
    Delete "$SMPROGRAMS\vis_taskbar\Uninstall.lnk"
    RMDir "$SMPROGRAMS\vis_taskbar"
    Delete "$SMSTARTUP\vis_taskbar.lnk"

    DeleteRegKey HKCU "Software\vis_taskbar"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\vis_taskbar"
SectionEnd
