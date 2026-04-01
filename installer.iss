[Setup]
AppName=vis_taskbar
AppVersion=0.8.0
AppPublisher=Vadim Kuras
AppPublisherURL=https://github.com/vakuras/vis_taskbar
DefaultDirName={autopf}\vis_taskbar
DefaultGroupName=vis_taskbar
OutputBaseFilename=vis_taskbar-setup
OutputDir=.
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\vis_taskbar.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern

[Files]
Source: "target\release\vis_taskbar.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\vis_taskbar"; Filename: "{app}\vis_taskbar.exe"; IconFilename: "{app}\icon.ico"
Name: "{group}\Uninstall vis_taskbar"; Filename: "{uninstallexe}"
Name: "{userstartup}\vis_taskbar"; Filename: "{app}\vis_taskbar.exe"; IconFilename: "{app}\icon.ico"

[Run]
Filename: "{app}\vis_taskbar.exe"; Description: "Launch vis_taskbar"; Flags: nowait postinstall skipifsilent
