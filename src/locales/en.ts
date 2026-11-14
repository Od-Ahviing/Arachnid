import { en as saveMessages } from '../composables/m3Messages'
import { en as additions } from '../composables/m2Messages'

const baseEn = {
  home: 'Home',
  localDirect3dUnverified:
    'A local d3d9.dll blocks launch. Review graphics mods installed outside Arachnid.',
  localLegacyDirectxUnverified:
    'A local d3dx9_43.dll blocks launch. Review this file in your game folder.',
  gameBuild: 'Game executable version',
  gameBuildHint:
    'If this version is unsupported, export the diagnostic report to share the Game.exe fingerprint.',
  profilesUnavailable:
    'At least one graphics setting has an unreadable format. See the settings details below.',
  defaultModsDeferred: 'Fix Enhancers could not be applied automatically.',
  modReconcile:
    'Some managed graphics files differ from the installed pack. Apply changes to back them up and use the verified originals and selected mods.',
  appUpdateAvailable: 'Update available',
  appUpdateInstall: 'Update now',
  appUpdateLater: 'Later',
  appUpdateDownloading: 'Downloading update…',
  appUpdateVerifying: 'Verifying and preparing the installation…',
  appUpdateCheckFailed: 'Could not check for updates. Restart Arachnid to try again.',
  appUpdateDownloadFailed: 'The download or signature verification failed. Try again.',
  appUpdateInstallFailed: 'Could not start the installer. Try again.',
  appUpdateBusy: 'An update operation is already in progress.',
  graphicsCrashTitle: 'The game closed unexpectedly',
  graphicsCrashReshade:
    'Disable ReShade in Mods, apply the change, then launch the game again. If it still crashes, try disabling the other graphics mods.',
  graphicsCrashMods:
    'Disable the active graphics mods in Mods, apply the changes, then launch the game again.',
  graphicsCrashClose: 'Close',
  graphicsCrashOpenMods: 'Open Mods',
  modPackUpdateRequired:
    'Replace the previous performance pack through deactivation and reactivation in Mods.',
  modSettingsOutOfSync:
    'Synchronise the performance pack with the display settings in Mods before playing.',
  modOwnedFileChanged:
    'A performance pack file changed outside Arachnid. Review its state in Mods.',
  modPayloadMissing:
    'The offline performance pack resources are missing or invalid. Open Mods to deactivate the pack or restore its resources.',
  performancePackUnqualified:
    'This graphics pack is unavailable in this build of Arachnid. Disable it in Mods.',
  graphics: 'Graphics',
  mods: 'Mods',
  saves: 'Saves',
  repair: 'Diagnostics',
  settings: 'Settings',
  skip: 'Skip to content',
  later: 'Available in a later milestone',
  welcome: 'Your game, ready for a fresh start.',
  setupDescription:
    'Select your installation of The Amazing Spider-Man (2012). Arachnid reads its files to identify the game and check its configuration.',
  ownGame: 'You need your own copy of the game. Arachnid does not distribute it.',
  directory: 'Game folder',
  directoryHint: 'Choose the folder containing Game.exe and Data.',
  browse: 'Browse',
  inspect: 'Check installation',
  cancel: 'Cancel',
  continue: 'Continue',
  scanning: 'Checking your installation',
  inventory: 'Reading the file inventory',
  binaries: 'Inspecting executable headers and hashes',
  filesRead: 'Files read',
  recognized: 'Installation recognized',
  invalid: 'Installation not recognized',
  recognizedHint: 'The expected structure was found. This does not confirm that the game can run.',
  invalidHint: 'Check the selected folder and review the findings below.',
  readOnly: 'Read-only diagnostics',
  readOnlyHint:
    'Diagnostics leave game files unchanged. Settings changes require a separate preview and approval.',
  play: 'Play',
  playUnavailable: 'Launching is not included in M1.',
  gameTitle: 'The Amazing Spider-Man',
  gameEdition: 'PC · 2012',
  overview: 'Installation overview',
  files: 'Files',
  archives: 'PKZ archives',
  size: 'Installation size',
  folder: 'Installation folder',
  lastScan: 'Last checked',
  viewDiagnostics: 'View diagnostics',
  checksTitle: 'Understand your installation.',
  checksDescription:
    'File and configuration checks, with clear limits. A passed check only confirms what was inspected.',
  scanAgain: 'Check again',
  export: 'Export report',
  exportHint:
    'Preview: absolute installation path and full file list are excluded. The report includes checks, file counts and executable metadata. Nothing is uploaded.',
  exportDone: 'Report saved.',
  exportTitle: 'Report export',
  passed: 'Checked',
  warning: 'Attention',
  error: 'Problem',
  unverified: 'Not verified',
  skipped: 'Not applicable',
  technical: 'Executable details',
  hash: 'SHA-256',
  imports: 'Direct imports',
  importsOmitted:
    'Imports omitted for this large file. Headers and the full file hash were inspected.',
  architecture: 'Architecture',
  settingsTitle: 'Make Arachnid your own.',
  settingsDescription: 'Manage the installation and local launcher preferences.',
  language: 'Interface and game language',
  languageHint: 'Automatic uses your device language. Unsupported languages fall back to English.',
  automatic: 'Automatic',
  changeFolder: 'Change folder',
  about: 'About Arachnid',
  aboutDescription: 'Community launcher · GPLv2 · The Amazing Spider-Man (2012) only.',
  desktopOnly: 'Open Arachnid as a desktop application to access installation diagnostics.',
  loading: 'Loading Arachnid',
  retry: 'Try again',
  errorTitle: 'Something needs your attention',
  unknownError: 'The operation could not be completed. Try again.',
  invalidPath: 'Choose a local folder. Relative, network and linked paths are not supported.',
  directoryUnavailable: 'This folder is unavailable or cannot be read.',
  fileUnreadable: 'Some files cannot be read. Check folder permissions and try again.',
  invalidBinary: 'An executable is not a supported PE file.',
  fileChanged: 'A file changed during the scan. Wait until other tools finish, then check again.',
  cancelled: 'The scan was cancelled. No game files were changed.',
  scanLimit: 'This folder exceeds the scan limits. Select the game installation folder.',
  scanBusy: 'A scan is already running.',
  preferencesUnavailable:
    'Launcher preferences could not be saved or read. Check the application data folder.',
  preferencesInvalid:
    'The preferences file is damaged. It has been preserved. Back it up before resetting it.',
  preferencesVersion:
    'These preferences were created by another version. The file has been preserved.',
  dialogUnavailable: 'The system folder dialog could not open.',
  exportFailed: 'The report could not be saved. Choose another destination.',
  exportInsideGame: 'Save the report outside the game folder to keep the installation unchanged.',
  noReport: 'Run an installation check first.',
  unauthorized: 'This window cannot perform that operation.',
  invalidResponse: 'The desktop service returned an unsupported response.',
  referenceInvalid: 'The bundled structure reference is invalid.',
  referenceResources: 'Local reference resources',
  referenceResourcesHint:
    'Compares names and sizes of non-DLC Data and Video resources with the selected local reference. The number shown is the count of differences. Contents are not hashed.',
  referenceUnknown: 'Different executable build',
  referenceUnknownHint:
    'The local structure reference targets another executable hash. No archive completeness claim is made for this build.',
  gameBinary: 'Game executable',
  gameBinaryHint:
    'Checks the PE structure, x86 architecture and SHA-256 without executing the file.',
  resources: 'Game archives',
  resourcesHint:
    'Counts non-empty PKZ files in Data. Archive contents and completeness are not verified.',
  shaders: 'Shader resources',
  shadersHint: 'Checks that Data/Shaders contains non-empty files.',
  gameLibrary: 'Game library',
  gameLibraryHint:
    'Checks for a non-empty steam_api.dll. Its presence does not validate its origin or compatibility.',
  graphicsPresets: 'Graphics presets',
  graphicsPresetsHint: 'Checks for GraphicsPreset.xml. No presets are modified.',
  videoDetection: 'Video detection component',
  videoDetectionHint: 'Checks for VideoDetection.dll without loading it.',
  wrapper: 'Optional Direct3D wrapper',
  wrapperHint: 'A local d3d9.dll requires its own compatibility checks. Its absence is normal.',
  resourceIntegrity: 'Full resource integrity',
  resourceIntegrityHint:
    'No universal reference manifest is available. File presence cannot prove archive integrity.',
  binaryArchitecture: 'Local executable architecture',
  binaryArchitectureHint:
    'A local executable or DLL differs from the game’s x86 architecture. Review its role before changing anything.',
  localBinary: 'Local executable format',
  localBinaryHint: 'An executable or DLL could not be parsed. No file was loaded or removed.',
  linksSkipped: 'Linked files excluded',
  linksSkippedHint:
    'Symbolic links and reparse points were skipped. The inventory may be incomplete.',
  displaySettings: 'Saved display resolution',
  displaySettingsHint:
    'Reads Width and Height as DWORD values from the game’s 32-bit user registry view. This is not a measurement in-game.',
  displayAbsent: 'Display settings not found',
  displayAbsentHint:
    'The game’s display registry key is absent. This alone does not indicate a broken installation.',
  displayUnreadable: 'Display settings unavailable',
  displayUnreadableHint: 'The registry key could not be read. No values were changed.',
  visualCpp: 'Direct Visual C++ dependencies',
  visualCppHint: 'Lists missing Visual C++ libraries imported directly by Game.exe.',
  direct3d: 'Direct3D 9 x86',
  direct3dHint:
    'If d3d9.dll is listed, Arachnid could not verify a local wrapper. Review any graphics mods installed outside Arachnid. Otherwise, check the Windows Direct3D 9 component.',
  legacyDirectx: 'Legacy DirectX component',
  legacyDirectxHint:
    'Checks for d3dx9_43.dll in the x86 system location. A file check alone does not prove all runtime dependencies are satisfied.',
  runtimeUntested: 'In-game operation',
  runtimeUntestedHint:
    'The game has not been launched by this diagnostic. Performance, stability and save loading have not been tested.',
  windowsOnly: 'Windows configuration',
  windowsOnlyHint: 'This check is only available on Windows.',
} as const

export type BaseMessageKey = keyof typeof baseEn
export const en = { ...baseEn, ...additions, ...saveMessages } as const
export type MessageDictionary = Record<keyof typeof en, string>
