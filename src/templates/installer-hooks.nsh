; Seelen Lite: no elevated service, per-user install.
!macro NSIS_HOOK_PREINSTALL
  StrCpy $1 "taskkill.exe /F /T /IM seelen-ui.exe"
  DetailPrint 'Exec: $1'
  nsExec::Exec $1
  Pop $0

  ; Clean static folder to remove assets from previous versions
  DetailPrint 'Cleaning static folder from previous installation...'
  RMDir /r "$INSTDIR\static"

  File /a "${__FILEDIR__}\..\..\sluhk.dll"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Autostart at logon (same value the app writes in set_auto_start)
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "SeelenLite" '"$INSTDIR\seelen-ui.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $1 "taskkill.exe /T /IM seelen-ui.exe"
  nsExec::Exec $1
  Pop $0
  Sleep 2000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "SeelenLite"
  Delete "$INSTDIR\sluhk.dll"
!macroend
