Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define ROOT "..\..\.."

Name "LDCodex"
OutFile "${ROOT}\dist\windows\LDCodex-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\LDCodex"
InstallDirRegKey HKCU "Software\LDCodex" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "${ROOT}\apps\codex-plus-manager\src-tauri\icons\icon.ico"
!define MUI_UNICON "${ROOT}\apps\codex-plus-manager\src-tauri\icons\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"

  nsExec::ExecToLog 'taskkill /IM ldcodex.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM LDCodexManager.exe /F'
  Pop $0

  File "${ROOT}\dist\windows\app\ldcodex.exe"
  File "${ROOT}\dist\windows\app\LDCodexManager.exe"


  CreateShortcut "$DESKTOP\LDCodex.lnk" "$INSTDIR\ldcodex.exe" "" "$INSTDIR\ldcodex.exe"
  CreateShortcut "$DESKTOP\LDCodex 管理工具.lnk" "$INSTDIR\LDCodexManager.exe" "" "$INSTDIR\LDCodexManager.exe"
  CreateDirectory "$SMPROGRAMS\LDCodex"
  CreateShortcut "$SMPROGRAMS\LDCodex\LDCodex.lnk" "$INSTDIR\ldcodex.exe" "" "$INSTDIR\ldcodex.exe"
  CreateShortcut "$SMPROGRAMS\LDCodex\LDCodex 管理工具.lnk" "$INSTDIR\LDCodexManager.exe" "" "$INSTDIR\LDCodexManager.exe"
  CreateShortcut "$SMPROGRAMS\LDCodex\卸载 LDCodex.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\LDCodexManager.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\LDCodex" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "DisplayName" "LDCodex"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "Publisher" "LUODA"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "DisplayIcon" "$INSTDIR\LDCodexManager.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  nsExec::ExecToLog 'taskkill /IM ldcodex.exe /F'
  Pop $0
  nsExec::ExecToLog 'taskkill /IM LDCodexManager.exe /F'
  Pop $0

  Delete "$DESKTOP\LDCodex.lnk"
  Delete "$DESKTOP\LDCodex 管理工具.lnk"
  Delete "$SMPROGRAMS\LDCodex\LDCodex.lnk"
  Delete "$SMPROGRAMS\LDCodex\LDCodex 管理工具.lnk"
  Delete "$SMPROGRAMS\LDCodex\卸载 LDCodex.lnk"
  RMDir "$SMPROGRAMS\LDCodex"

  Delete "$INSTDIR\ldcodex.exe"
  Delete "$INSTDIR\LDCodexManager.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LDCodex"
  DeleteRegKey HKCU "Software\LDCodex"
SectionEnd
