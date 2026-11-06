; NOMNOM installer hooks. The installer runs per user, so nothing here needs administrator rights.
;
; After install: set up whatever the user has already chosen. On a first install that is nothing,
; because NomNom asks on its first run (right-click menu, Firefox, start with Windows).
; Before uninstall: remove everything NomNom added outside its own folder: the right-click menu
; (Windows 11 package and classic entries), the Firefox connection and the start-with-Windows entry.

Var NomNomRunBackup

; The default per-user folder is %LOCALAPPDATA%\NOMNOM, which is also where NomNom keeps its
; settings, history and tools. Program files go under Programs\ like other per-user apps, so the
; two never share a folder. A folder the user picked themselves is left alone.
!macro NSIS_HOOK_PREINSTALL
  ${If} $INSTDIR == "$LOCALAPPDATA\${PRODUCTNAME}"
    StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${PRODUCTNAME}"
    SetOutPath $INSTDIR
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec '"$INSTDIR\nomnom.exe" --install-integrations'
!macroend

; The standard uninstaller deletes a start-with-Windows value named like the product ("NOMNOM"),
; which another program may own. NomNom's own value has a different name and is removed by
; --uninstall-integrations, so whatever was under "NOMNOM" is put back afterwards.
!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $NomNomRunBackup HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  nsExec::Exec '"$INSTDIR\nomnom.exe" --uninstall-integrations'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $NomNomRunBackup != ""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" "$NomNomRunBackup"
  ${EndIf}
  ; The installer remembers its folder under HKCU\Software\nomnom\NOMNOM so an upgrade can reuse it.
  ; On a real uninstall (not an update) that note is no longer needed.
  ${If} $UpdateMode <> 1
    DeleteRegKey HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"
  ${EndIf}
!macroend
