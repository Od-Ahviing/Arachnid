import { de as saveMessages } from '../composables/m3Messages'
import type { MessageDictionary } from './en'

export const de = {
  ...saveMessages,
  graphicsProfile: 'Grafikprofil',
  profileveryLow: 'Sehr niedrig',
  profilelow: 'Niedrig',
  profilemedium: 'Mittel',
  profilehigh: 'Hoch',
  profileamazing: 'Fantastisch (Amazing)',
  profilecustom: 'Benutzerdefiniert',
  profileNotice:
    'Vorlagen aus GraphicsPreset.xml des offiziellen Launchers. Auflösung, Anzeigemodus und Sprache bleiben erhalten. Änderungen werden erst beim Anwenden übernommen.',
  mediumOption: 'Mittel',
  veryHighOption: 'Sehr hoch',
  settingShadowDetail: 'Schattendetails',
  settingParticles: 'Partikel',
  settingLightShafts: 'Lichtstrahlen',
  settingHalo: 'Halo',
  settingDrawDistance: 'Sichtweite',
  settingGodrays: 'Sonnenstrahlen',
  settingSsao: 'Umgebungsverdeckung',
  settingDepthOfField: 'Tiefenschärfe',
  settingBloom: 'Leuchteffekt',
  settingHeat: 'Hitzeflimmern',
  home: 'Startseite',
  localDirect3dUnverified:
    'Eine lokale d3d9.dll blockiert den Start. Prüfe Grafikmods, die außerhalb von Arachnid installiert wurden.',
  localLegacyDirectxUnverified:
    'Eine lokale d3dx9_43.dll blockiert den Start. Prüfe diese Datei im Spielordner.',
  gameBuild: 'Version der Spieldatei',
  gameBuildHint:
    'Exportiere bei einer nicht unterstützten Version den Bericht mit der Prüfsumme von Game.exe.',
  profilesUnavailable:
    'Mindestens eine Grafikeinstellung hat ein unlesbares Format. Details stehen weiter unten.',
  defaultModsDeferred: 'Fix Enhancers konnte nicht automatisch angewendet werden.',
  modReconcile:
    'Verwaltete Grafikdateien weichen vom installierten Paket ab. Beim Anwenden werden sie gesichert und die geprüften Originale als Grundlage verwendet.',
  appUpdateAvailable: 'Update verfügbar',
  appUpdateInstall: 'Jetzt aktualisieren',
  appUpdateLater: 'Später',
  appUpdateDownloading: 'Update wird heruntergeladen…',
  appUpdateVerifying: 'Installation wird geprüft und vorbereitet…',
  appUpdateCheckFailed:
    'Updates konnten nicht geprüft werden. Starte Arachnid neu und versuche es erneut.',
  appUpdateDownloadFailed: 'Download oder Signaturprüfung fehlgeschlagen. Versuche es erneut.',
  appUpdateInstallFailed:
    'Das Installationsprogramm konnte nicht gestartet werden. Versuche es erneut.',
  appUpdateBusy: 'Ein Update-Vorgang läuft bereits.',
  graphicsCrashTitle: 'Das Spiel wurde unerwartet beendet',
  graphicsCrashReshade:
    'Deaktiviere zuerst ReShade unter Mods, übernimm die Änderung und starte das Spiel erneut. Falls es erneut abstürzt, deaktiviere die anderen Grafikmods.',
  graphicsCrashMods:
    'Deaktiviere die aktiven Grafikmods unter Mods, übernimm die Änderungen und starte das Spiel erneut.',
  graphicsCrashClose: 'Schließen',
  graphicsCrashOpenMods: 'Mods öffnen',
  modPackUpdateRequired:
    'Ersetze das vorherige Paket unter Mods durch Deaktivieren und erneutes Aktivieren.',
  modSettingsOutOfSync:
    'Gleiche das Leistungspaket vor dem Spielen unter Mods mit den Anzeigeeinstellungen ab.',
  modOwnedFileChanged:
    'Eine Paketdatei wurde außerhalb von Arachnid geändert. Prüfe ihren Status unter Mods.',
  modPayloadMissing:
    'Lokale Paketressourcen fehlen oder sind ungültig. Deaktiviere das Paket unter Mods oder stelle die Ressourcen wieder her.',
  performancePackUnqualified:
    'Dieses Grafikpaket ist in dieser Arachnid-Version nicht verfügbar. Deaktiviere es unter Mods.',
  graphics: 'Grafik',
  mods: 'Mods',
  saves: 'Spielstände',
  repair: 'Diagnose',
  settings: 'Einstellungen',
  skip: 'Zum Inhalt springen',
  later: 'In einem späteren Meilenstein verfügbar',
  welcome: 'Dein Spiel, bereit für einen Neuanfang.',
  setupDescription:
    'Wähle deine Installation von The Amazing Spider-Man (2012). Arachnid liest die Dateien, um das Spiel zu erkennen und seine Konfiguration zu prüfen.',
  ownGame: 'Du benötigst eine eigene Kopie des Spiels. Arachnid stellt das Spiel nicht bereit.',
  directory: 'Spielordner',
  directoryHint: 'Wähle den Ordner mit Game.exe und Data.',
  browse: 'Durchsuchen',
  inspect: 'Installation prüfen',
  cancel: 'Abbrechen',
  continue: 'Weiter',
  scanning: 'Installation wird geprüft',
  inventory: 'Dateiverzeichnis wird gelesen',
  binaries: 'Dateiköpfe und Prüfsummen werden untersucht',
  filesRead: 'Erfasste Dateien',
  recognized: 'Installation erkannt',
  invalid: 'Installation nicht erkannt',
  recognizedHint:
    'Die erwartete Struktur ist vorhanden. Das bestätigt noch nicht, dass das Spiel startet.',
  invalidHint: 'Prüfe den Ordner und die folgenden Ergebnisse.',
  readOnly: 'Schreibgeschützte Diagnose',
  readOnlyHint:
    'Die Diagnose verändert keine Spieldateien. Einstellungsänderungen erfordern eine eigene Vorschau und Zustimmung.',
  play: 'Spielen',
  playUnavailable: 'Der Spielstart gehört nicht zu M1.',
  gameTitle: 'The Amazing Spider-Man',
  gameEdition: 'PC · 2012',
  overview: 'Deine Installation',
  files: 'Dateien',
  archives: 'PKZ-Archive',
  size: 'Installationsgröße',
  folder: 'Installationsordner',
  lastScan: 'Zuletzt geprüft',
  viewDiagnostics: 'Diagnose ansehen',
  checksTitle: 'Deine Installation verstehen.',
  checksDescription:
    'Datei- und Konfigurationsprüfungen mit klaren Grenzen. Ein erfolgreicher Test bestätigt nur den geprüften Sachverhalt.',
  scanAgain: 'Erneut prüfen',
  export: 'Bericht exportieren',
  exportHint:
    'Vorschau: Installationspfad und vollständige Dateiliste sind ausgeschlossen. Der Bericht enthält Prüfungen, Dateizahlen und Programmmetadaten. Es wird nichts hochgeladen.',
  exportDone: 'Bericht gespeichert.',
  exportTitle: 'Bericht exportieren',
  passed: 'Geprüft',
  warning: 'Zu prüfen',
  error: 'Problem',
  unverified: 'Nicht verifiziert',
  skipped: 'Nicht zutreffend',
  technical: 'Details der Programmdateien',
  hash: 'SHA-256',
  imports: 'Direkte Importe',
  importsOmitted:
    'Importe wurden bei dieser großen Datei ausgelassen. Dateiköpfe und vollständige Prüfsumme wurden untersucht.',
  architecture: 'Architektur',
  settingsTitle: 'Arachnid nach deinen Wünschen.',
  settingsDescription: 'Installation und lokale Launcher-Einstellungen verwalten.',
  language: 'Sprache von Oberfläche und Spiel',
  languageHint:
    'Automatisch folgt der Gerätesprache. Bei nicht unterstützten Sprachen wird Englisch verwendet.',
  automatic: 'Automatisch',
  changeFolder: 'Ordner ändern',
  about: 'Über Arachnid',
  aboutDescription: 'Community-Launcher · GPLv2 · Nur The Amazing Spider-Man (2012).',
  desktopOnly: 'Öffne Arachnid als Desktop-Anwendung, um die Installation zu prüfen.',
  loading: 'Arachnid wird geladen',
  retry: 'Erneut versuchen',
  errorTitle: 'Eine Prüfung ist erforderlich',
  unknownError: 'Der Vorgang konnte nicht abgeschlossen werden. Versuche es erneut.',
  invalidPath:
    'Wähle einen lokalen Ordner. Relative Pfade, Netzwerkpfade und Verknüpfungen werden nicht unterstützt.',
  directoryUnavailable: 'Dieser Ordner ist nicht verfügbar oder nicht lesbar.',
  fileUnreadable: 'Einige Dateien sind nicht lesbar. Prüfe die Ordnerberechtigungen.',
  invalidBinary: 'Eine Programmdatei ist keine unterstützte PE-Datei.',
  fileChanged:
    'Eine Datei wurde während der Prüfung geändert. Warte auf das Ende anderer Vorgänge.',
  cancelled: 'Die Prüfung wurde abgebrochen. Keine Spieldateien wurden geändert.',
  scanLimit:
    'Dieser Ordner überschreitet die Prüfgrenzen. Wähle den Installationsordner des Spiels.',
  scanBusy: 'Eine Prüfung läuft bereits.',
  preferencesUnavailable:
    'Launcher-Einstellungen konnten nicht gelesen oder gespeichert werden. Prüfe den Anwendungsdatenordner.',
  preferencesInvalid:
    'Die Einstellungsdatei ist beschädigt und wurde beibehalten. Sichere sie vor dem Zurücksetzen.',
  preferencesVersion:
    'Diese Einstellungen stammen aus einer anderen Version. Die Datei wurde beibehalten.',
  dialogUnavailable: 'Die Ordnerauswahl konnte nicht geöffnet werden.',
  exportFailed: 'Der Bericht konnte nicht gespeichert werden. Wähle ein anderes Ziel.',
  exportInsideGame:
    'Speichere den Bericht außerhalb des Spielordners, damit die Installation unverändert bleibt.',
  noReport: 'Prüfe zuerst die Installation.',
  unauthorized: 'Dieses Fenster darf den Vorgang nicht ausführen.',
  invalidResponse: 'Der Desktop-Dienst hat eine nicht unterstützte Antwort zurückgegeben.',
  referenceInvalid: 'Die mitgelieferte Strukturreferenz ist ungültig.',
  referenceResources: 'Ressourcen der lokalen Referenz',
  referenceResourcesHint:
    'Vergleicht Namen und Größen der Data- und Video-Ressourcen ohne DLC mit der lokalen Referenz. Die Zahl gibt die Abweichungen an. Inhalte werden nicht gehasht.',
  referenceUnknown: 'Andere Programmversion',
  referenceUnknownHint:
    'Die Strukturreferenz gehört zu einer anderen Prüfsumme. Für diese Version wird keine Vollständigkeit bestätigt.',
  gameBinary: 'Programmdatei des Spiels',
  gameBinaryHint: 'Prüft PE-Struktur, x86-Architektur und SHA-256, ohne die Datei auszuführen.',
  resources: 'Spielarchive',
  resourcesHint:
    'Zählt nicht leere PKZ-Dateien in Data. Inhalt und Vollständigkeit werden nicht geprüft.',
  shaders: 'Shader-Ressourcen',
  shadersHint: 'Prüft, ob Data/Shaders nicht leere Dateien enthält.',
  gameLibrary: 'Spielbibliothek',
  gameLibraryHint:
    'Prüft auf eine nicht leere steam_api.dll. Ihre Anwesenheit bestätigt weder Herkunft noch Kompatibilität.',
  graphicsPresets: 'Grafikvorgaben',
  graphicsPresetsHint: 'Prüft auf GraphicsPreset.xml. Es werden keine Vorgaben geändert.',
  videoDetection: 'Grafikerkennung',
  videoDetectionHint: 'Prüft VideoDetection.dll, ohne sie zu laden.',
  wrapper: 'Optionaler Direct3D-Wrapper',
  wrapperHint: 'Eine lokale d3d9.dll benötigt eine eigene Prüfung. Ihr Fehlen ist normal.',
  resourceIntegrity: 'Vollständige Ressourcenintegrität',
  resourceIntegrityHint:
    'Es liegt kein universelles Referenzmanifest vor. Vorhandene Dateien beweisen keine Archivintegrität.',
  binaryArchitecture: 'Architektur lokaler Programmdateien',
  binaryArchitectureHint:
    'Eine EXE oder DLL weicht von x86 ab. Prüfe ihre Funktion vor Änderungen.',
  localBinary: 'Format lokaler Programmdateien',
  localBinaryHint:
    'Eine EXE oder DLL konnte nicht analysiert werden. Es wurde nichts geladen oder entfernt.',
  linksSkipped: 'Verknüpfte Dateien ausgeschlossen',
  linksSkippedHint:
    'Symbolische Links und Reparse Points wurden übersprungen. Die Dateiliste kann unvollständig sein.',
  displaySettings: 'Gespeicherte Auflösung',
  displaySettingsHint:
    'Liest Width und Height als DWORD aus der 32-Bit-Benutzerregistrierung des Spiels. Keine Messung im Spiel.',
  displayAbsent: 'Anzeigeeinstellungen fehlen',
  displayAbsentHint:
    'Der Registrierungsschlüssel für die Anzeige fehlt. Das allein bedeutet keine defekte Installation.',
  displayUnreadable: 'Anzeigeeinstellungen nicht lesbar',
  displayUnreadableHint:
    'Der Registrierungsschlüssel konnte nicht gelesen werden. Es wurde nichts geändert.',
  visualCpp: 'Direkte Visual-C++-Abhängigkeiten',
  visualCppHint: 'Listet fehlende Visual-C++-Bibliotheken auf, die Game.exe direkt importiert.',
  direct3d: 'Direct3D 9 x86',
  direct3dHint:
    'Prüft die x86-System-DLL oder kennzeichnet einen lokalen Wrapper. Keine DLL wird geladen.',
  legacyDirectx: 'Ältere DirectX-Komponente',
  legacyDirectxHint:
    'Sucht d3dx9_43.dll im x86-Systemordner. Eine vorhandene Datei bestätigt nicht alle Laufzeitabhängigkeiten.',
  runtimeUntested: 'Betrieb im Spiel',
  runtimeUntestedHint:
    'Diese Diagnose hat das Spiel nicht gestartet. Leistung, Stabilität und das Laden von Spielständen wurden nicht getestet.',
  windowsOnly: 'Windows-Konfiguration',
  windowsOnlyHint: 'Diese Prüfung ist nur unter Windows verfügbar.',
  gameLanguageHint:
    'Das Spiel folgt der auf der Startseite gewählten Sprache gemäß den installierten Ressourcen.',
  languageResourceEvidence:
    'Kennung des nativen Launchers untersucht; Verfügbarkeit anhand installierter Sprachressourcen. Sprachausgabe muss im Spiel geprüft werden.',
  languageResourcePresent: 'Sprachressource gefunden',
  languageResourcesMissing: 'Benötigte Sprachressource fehlt',
  nativeLanguageDefault: 'Native Vorgabe · nicht gespeichert',
  capabilityMatrix: 'Kompatibilität und Prüfung je Option',
  capabilityKindnative: 'Native Einstellung',
  capabilityKindexternal: 'Erweiterung erforderlich',
  capabilityKindderived: 'Arachnid-Profil',
  capabilityStatereadOnly: 'Nur lesen',
  capabilityStateexperimental: 'Experimentell',
  capabilityStatelocalUserTest: 'Lokaler Benutzertest bestanden',
  capabilityStateunavailable: 'Nicht verfügbar',
  localDisplayTest:
    'Fenstermodus und Rückkehr zu Vollbild wurden an der Referenzinstallation bestätigt. Andere Versionen und Geräte sind ungetestet.',
  localResolutionTest:
    '1280 × 720 und Rückkehr zu 1920 × 1080 wurden an der Referenzinstallation bestätigt. Andere Modi sind ungetestet.',
  needsInGameValidation: 'Native Zuordnung untersucht; Ergebnis muss noch im Spiel geprüft werden.',
  languageMappingUnknown: 'Sprachkennungen und vorhandene Sprachdateien sind noch nicht geprüft.',
  noQualifiedExtension: 'Für diese Spielversion wurde keine kompatible Erweiterung geprüft.',
  needsMeasurements: 'Dieses Profil erfordert wiederholbare Messungen im Spiel.',
  settingFpsLimit: 'Bildratenlimit',
  settingFieldOfView: 'Sichtfeld',
  settingBorderless: 'Randloses Fenster',
  settingVeryLowProfile: 'Sehr niedrig',
  settingLowProfile: 'Niedrig',
  settingMediumProfile: 'Mittel',
  settingHighProfile: 'Hoch',
  settingAmazingProfile: 'Fantastisch (Amazing)',
  inspectedSettings: 'Weitere untersuchte Einstellungen · nur lesen',
  enabledOption: 'Ein',
  disabledOption: 'Aus',
  lowOption: 'Niedrig',
  highOption: 'Hoch',
  launching: 'Spiel wird gestartet…',
  gameRunning: 'Spiel läuft. Wenn du Arachnid schließt, bleibt das Spiel geöffnet.',
  gameExited: 'Letzte Sitzung beendet. Exitcode:',
  exitCodeUnknown: 'nicht verfügbar',
  launchHint: 'Startet Game.exe direkt. Die Prüfungen werden vor jedem Start wiederholt.',
  installationInvalid: 'Wähle eine erkannte Spielinstallation.',
  recoveryRequired:
    'Ein unvollständiger Vorgang muss vor dem Spielen unter Grafik wiederhergestellt werden.',
  channelUnqualified: 'Für diese Programmversion gibt es noch keinen unterstützten Startweg.',
  diagnosticBlocking: 'Behebe die blockierenden Diagnoseergebnisse vor dem Start.',
  direct3dUnresolved: 'Direct3D 9 x86 muss geprüft werden.',
  legacyDirectxUnresolved: 'Die ältere DirectX-Komponente muss geprüft werden.',
  visualCppUnresolved: 'Eine direkte Visual-C++-Abhängigkeit konnte nicht aufgelöst werden.',
  installationBusy:
    'Ein anderer Vorgang verwendet diese Installation. Warte und versuche es erneut.',
  gameAlreadyRunning:
    'Das Spiel läuft oder sein Zustand ist unklar. Beende es vor einem weiteren Start oder Änderungen.',
  processStateUnknown:
    'Der Windows-Prozesszustand konnte nicht geprüft werden. Kein Start und keine Änderung wurden ausgeführt.',
  launchFailed: 'Windows konnte das Spiel nicht starten. Prüfe die Installation.',
  lockUnavailable: 'Die Installationssperre konnte nicht geöffnet werden.',
  graphicsTitle: 'Deine Anzeige, deine Einstellungen.',
  graphicsDescription: 'Anzeige und Bildqualität deines Spiels.',
  settingsExperimental:
    'Experimentelle Einstellungen: Native Zuordnung untersucht, sichtbare Auswirkungen teilweise noch ungetestet. Nur unterstützte Werte sind bearbeitbar.',
  settingsApplied:
    'Werte geschrieben und erneut gelesen. Starte das Spiel, um das Ergebnis zu prüfen.',
  settingAbsent: 'Nicht gespeichert',
  settingOpaque: 'Unerwarteter Typ · beibehalten',
  previewSettings: 'Änderungen prüfen',
  settingsPlan: 'Änderungen warten auf deine Zustimmung',
  settingsPlanHint:
    'Es wurde noch nichts angewendet. Du kannst ohne diese Änderungen spielen. Ursprüngliche Werte sind außerhalb des Spielordners gesichert. Während des Spiels ist das Anwenden gesperrt. Die Zustimmung verfällt nach zehn Minuten.',
  approveSettings: 'Zustimmen und anwenden',
  restoreSettings: 'Vorherige Werte wiederherstellen',
  restoreSettingsHint:
    'Stellt die gezeigten Werte wieder her. Eine externe Änderung führt zu einem Konflikt und bleibt erhalten.',
  profilesAndLanguages: 'Profile und Spielsprachen',
  profilesPending:
    'Profile und erweiterte Effekte benötigen noch Messungen im Spiel. FPS, Sichtfeld und randloser Modus erfordern eine kompatible Erweiterung. Die Spielsprachen folgen den unten aufgeführten Ressourcen.',
  settingWidth: 'Breite der Auflösung',
  settingHeight: 'Höhe der Auflösung',
  settingFullscreen: 'Vollbild',
  settingVSync: 'Vertikale Synchronisierung',
  settingMaxAniso: 'Anisotrope Filterung',
  settingAntiAliasing: 'Kantenglättung',
  settingMotionBlur: 'Bewegungsunschärfe',
  settingQualitySettings: 'Qualitätsflags',
  settingRenderingFlags: 'Renderingflags',
  settingAspectRatio: 'Seitenverhältniscode · nur lesen',
  settingRefreshRate: 'Bildwiederholfrequenz · nur lesen',
  settingGraphicsPreset: 'Native Grafikvorgabe',
  settingFxPreset: 'Native Effektvorgabe',
  settingLightningPreset: 'Native Beleuchtungsvorgabe',
  settingShadowsPreset: 'Native Schattenvorgabe',
  settingCityLifeDensity: 'Native Stadtdichte',
  settingLanguage: 'Spielsprache',
  unsupportedSetting:
    'Dieser Wert fehlt, wird nicht unterstützt oder liegt außerhalb des zulässigen Bereichs. Es wurde nichts geschrieben.',
  invalidPlan: 'Der Änderungsplan ist ungültig. Prüfe die Werte und bereite ihn erneut vor.',
  stalePlan: 'Seit der Vorschau wurde ein Wert geändert. Erstelle einen neuen Plan.',
  approvalInvalid:
    'Die Zustimmung ist abgelaufen oder passt nicht zum Plan. Bereite ihn erneut vor.',
  journalUnavailable:
    'Das Sicherungsjournal konnte nicht gespeichert werden. Prüfe Speicherplatz und Berechtigungen.',
  registryUnreadable:
    'Native Einstellungen konnten nicht gelesen werden. Keine Werte wurden geändert.',
  writeFailed: 'Ein Schreibvorgang ist fehlgeschlagen. Prüfe, ob eine Wiederherstellung nötig ist.',
  rollbackConflict:
    'Eine externe Änderung verhindert die Wiederherstellung. Der aktuelle Wert wurde beibehalten.',
  validationFailed:
    'Der geschriebene Wert entspricht nicht dem Plan. Eine Wiederherstellung wurde angefordert.',
  operationLimit: 'Dieser Vorgang überschreitet die unterstützten Größenlimits.',
  spaceUnavailable:
    'Der verfügbare Speicherplatz konnte nicht geprüft werden. Keine Änderung angewendet.',
  insufficientSpace:
    'Nicht genügend freier Speicher für Journal, Zwischendateien und Sicherheitsreserve.',
  clockInvalid: 'Die Systemuhr konnte nicht zur Prüfung der Zustimmung verwendet werden.',
} satisfies MessageDictionary
