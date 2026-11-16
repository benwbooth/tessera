; Inno Setup script for the Windows installer.
; Built by .github/workflows/release.yml:
;   iscc /DAppVersion=1.2.3 /DSourceDir=<windeployqt folder> /DOutputDir=<out> tessera.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\dist\Tessera"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
AppId={{3D9A7C21-58E4-4F0B-A6C2-9E4B1F7D2A65}
AppName=Tessera
AppVersion={#AppVersion}
AppVerName=Tessera {#AppVersion}
AppPublisher=Ben Booth
AppPublisherURL=https://github.com/benwbooth/tessera
AppSupportURL=https://github.com/benwbooth/tessera/issues
DefaultDirName={autopf}\Tessera
DefaultGroupName=Tessera
DisableProgramGroupPage=yes
; Per-user install by default (no admin prompt); the person can choose all users.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Tessera-{#AppVersion}-windows-x64-setup
SetupIconFile=..\icons\tessera.ico
UninstallDisplayIcon={app}\tessera.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Tessera"; Filename: "{app}\tessera.exe"
Name: "{autodesktop}\Tessera"; Filename: "{app}\tessera.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\tessera.exe"; Description: "{cm:LaunchProgram,Tessera}"; Flags: nowait postinstall skipifsilent
