; Panopticon — Windows Installer Script
; Requires Inno Setup 6 or later.
;
; Build from the repository root:
;   ISCC.exe /DAppVersion="1.2.3" installer\panopticon.iss
;
; The compiled installer is written to dist\panopticon-{version}-setup.exe.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

; ---------------------------------------------------------------------------
[Setup]
; Keep this GUID stable across releases so upgrades work correctly.
AppId={{E4B2A1F3-7C9D-4E5B-8F2A-3D6C0B1E9A5F}
AppName=Panopticon
AppVersion={#AppVersion}
AppPublisher=gvastethecreator
AppPublisherURL=https://github.com/gvastethecreator/panopticon
AppSupportURL=https://github.com/gvastethecreator/panopticon/issues
AppUpdatesURL=https://github.com/gvastethecreator/panopticon/releases
DefaultDirName={autopf}\Panopticon
DefaultGroupName=Panopticon
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=panopticon-{#AppVersion}-setup
SetupIconFile=..\assets\icon.ico
Compression=lzma
SolidCompression=yes
WizardStyle=modern
; Do not require admin — a tray utility works fine in the user profile.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
; Ensure Panopticon is closed before install/uninstall touches its executable.
CloseApplications=yes
CloseApplicationsFilter=panopticon.exe
RestartApplications=no
; Ship a 64-bit-only installer.
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\panopticon.exe

; ---------------------------------------------------------------------------
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

; ---------------------------------------------------------------------------
[Tasks]
Name: "desktopicon"; \
  Description: "{cm:CreateDesktopIcon}"; \
  GroupDescription: "{cm:AdditionalIcons}"; \
  Flags: unchecked
Name: "startuptask"; \
  Description: "Launch Panopticon when Windows starts"; \
  GroupDescription: "System integration:"; \
  Flags: unchecked

; ---------------------------------------------------------------------------
[Files]
; themes.json is embedded via include_str! at compile time — no separate copy needed.
Source: "..\target\release\panopticon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md";                     DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE";                       DestDir: "{app}"; Flags: ignoreversion

; ---------------------------------------------------------------------------
[Icons]
Name: "{group}\Panopticon";                        Filename: "{app}\panopticon.exe"
Name: "{group}\{cm:UninstallProgram,Panopticon}";  Filename: "{uninstallexe}"
Name: "{commondesktop}\Panopticon";                Filename: "{app}\panopticon.exe"; Tasks: desktopicon

; ---------------------------------------------------------------------------
[Registry]
; Optional: launch on Windows startup (only when the user chose the task).
Root: HKCU; \
  Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
  ValueType: string; ValueName: "Panopticon"; \
  ValueData: """{app}\panopticon.exe"""; \
  Flags: uninsdeletevalue; \
  Tasks: startuptask

; ---------------------------------------------------------------------------
[Run]
Filename: "{app}\panopticon.exe"; \
  Description: "{cm:LaunchProgram,Panopticon}"; \
  Flags: nowait postinstall skipifsilent

; ---------------------------------------------------------------------------
[UninstallDelete]
; Remove user-generated state and logs so uninstall actually cleans up the app.
Type: filesandordirs; Name: "{userappdata}\Panopticon"
Type: filesandordirs; Name: "{tmp}\Panopticon"
Type: filesandordirs; Name: "{tmp}\panopticon"
; Remove the install directory if the regular uninstaller cleanup emptied it.
Type: dirifempty; Name: "{app}"
