import { fr as saveMessages } from '../composables/m3Messages'
import { fr as additions } from '../composables/m2Messages'
import type { BaseMessageKey, MessageDictionary } from './en'

const baseFr: Record<BaseMessageKey, string> = {
  home: 'Accueil',
  graphicsCrashTitle: 'Le jeu s’est fermé de façon inattendue',
  graphicsCrashReshade:
    'Dans Mods, désactive d’abord ReShade, applique la modification, puis relance le jeu. Si le crash se reproduit, essaie de désactiver les autres mods graphiques.',
  graphicsCrashMods:
    'Dans Mods, désactive les mods graphiques actifs, applique les modifications, puis relance le jeu.',
  graphicsCrashClose: 'Fermer',
  graphicsCrashOpenMods: 'Ouvrir les mods',
  modPackUpdateRequired:
    'Remplacez le précédent pack en le désactivant puis en le réactivant dans Mods.',
  modSettingsOutOfSync:
    'Synchronisez le pack avec les réglages d’affichage dans Mods avant de jouer.',
  modOwnedFileChanged: 'Un fichier du pack a changé hors d’Arachnid. Examinez son état dans Mods.',
  modPayloadMissing:
    'Les ressources locales du pack sont absentes ou invalides. Ouvrez Mods pour le désactiver ou rétablir ses ressources.',
  performancePackUnqualified:
    'Ce pack graphique est indisponible dans cette version d’Arachnid. Désactivez-le dans Mods.',
  graphics: 'Graphismes',
  mods: 'Mods',
  saves: 'Sauvegardes',
  repair: 'Diagnostic',
  settings: 'Paramètres',
  skip: 'Aller au contenu',
  later: 'Disponible dans un prochain jalon',
  welcome: 'Votre jeu, un nouveau départ.',
  setupDescription:
    'Sélectionnez votre installation de The Amazing Spider-Man (2012). Arachnid lit ses fichiers pour reconnaître le jeu et vérifier sa configuration.',
  ownGame: 'Vous devez posséder votre copie du jeu. Arachnid ne le distribue pas.',
  directory: 'Dossier du jeu',
  directoryHint: 'Choisissez le dossier contenant Game.exe et Data.',
  browse: 'Parcourir',
  inspect: 'Vérifier l’installation',
  cancel: 'Annuler',
  continue: 'Continuer',
  scanning: 'Vérification de votre installation',
  inventory: 'Lecture de l’inventaire',
  binaries: 'Analyse des en-têtes et empreintes des exécutables',
  filesRead: 'Fichiers recensés',
  recognized: 'Installation reconnue',
  invalid: 'Installation non reconnue',
  recognizedHint:
    'La structure attendue est présente. Cela ne confirme pas que le jeu peut démarrer.',
  invalidHint: 'Vérifiez le dossier sélectionné et consultez les résultats ci-dessous.',
  readOnly: 'Diagnostic en lecture seule',
  readOnlyHint:
    'Le diagnostic laisse les fichiers du jeu inchangés. Les modifications de réglages demandent un aperçu et un accord distincts.',
  play: 'Jouer',
  playUnavailable: 'Le lancement ne fait pas partie de M1.',
  gameTitle: 'The Amazing Spider-Man',
  gameEdition: 'PC · 2012',
  overview: 'Votre installation',
  files: 'Fichiers',
  archives: 'Archives PKZ',
  size: 'Taille de l’installation',
  folder: 'Dossier d’installation',
  lastScan: 'Dernière vérification',
  viewDiagnostics: 'Consulter le diagnostic',
  checksTitle: 'Comprendre votre installation.',
  checksDescription:
    'Des contrôles de fichiers et de configuration aux limites explicites. Un contrôle réussi confirme uniquement ce qui a été examiné.',
  scanAgain: 'Vérifier à nouveau',
  export: 'Exporter le rapport',
  exportHint:
    'Aperçu : le chemin absolu et la liste complète des fichiers sont exclus. Le rapport contient les contrôles, les compteurs et les métadonnées de l’exécutable. Rien n’est envoyé en ligne.',
  exportDone: 'Rapport enregistré.',
  exportTitle: 'Export du rapport',
  passed: 'Contrôlé',
  warning: 'À examiner',
  error: 'Problème',
  unverified: 'Non vérifié',
  skipped: 'Non applicable',
  technical: 'Détails de l’exécutable',
  hash: 'SHA-256',
  imports: 'Imports directs',
  importsOmitted:
    'Imports non analysés pour ce fichier volumineux. Les en-têtes et l’empreinte complète ont été examinés.',
  architecture: 'Architecture',
  settingsTitle: 'Arachnid, à votre façon.',
  settingsDescription: 'Gérez l’installation et les préférences locales du launcher.',
  language: 'Langue de l’interface et du jeu',
  languageHint:
    'Automatique suit la langue de l’appareil. Une langue non prise en charge utilise l’anglais.',
  automatic: 'Automatique',
  changeFolder: 'Changer de dossier',
  about: 'À propos d’Arachnid',
  aboutDescription: 'Launcher communautaire · GPLv2 · The Amazing Spider-Man (2012) uniquement.',
  desktopOnly:
    'Ouvrez Arachnid comme application de bureau pour accéder au diagnostic de l’installation.',
  loading: 'Chargement d’Arachnid',
  retry: 'Réessayer',
  errorTitle: 'Une vérification est nécessaire',
  unknownError: 'L’opération n’a pas abouti. Réessayez.',
  invalidPath:
    'Choisissez un dossier local. Les chemins relatifs, réseau et liés ne sont pas pris en charge.',
  directoryUnavailable: 'Ce dossier est indisponible ou ne peut pas être lu.',
  fileUnreadable:
    'Certains fichiers sont illisibles. Vérifiez les droits du dossier puis réessayez.',
  invalidBinary: 'Un exécutable n’est pas un fichier PE pris en charge.',
  fileChanged:
    'Un fichier a changé pendant l’analyse. Attendez la fin des autres opérations puis réessayez.',
  cancelled: 'L’analyse a été annulée. Aucun fichier du jeu n’a été modifié.',
  scanLimit:
    'Ce dossier dépasse les limites d’analyse. Sélectionnez le dossier d’installation du jeu.',
  scanBusy: 'Une analyse est déjà en cours.',
  preferencesUnavailable:
    'Les préférences du launcher ne peuvent pas être lues ou enregistrées. Vérifiez le dossier de données de l’application.',
  preferencesInvalid:
    'Le fichier de préférences est endommagé. Il a été conservé. Sauvegardez-le avant de le réinitialiser.',
  preferencesVersion: 'Ces préférences proviennent d’une autre version. Le fichier a été conservé.',
  dialogUnavailable: 'Le sélecteur de dossier système n’a pas pu s’ouvrir.',
  exportFailed: 'Le rapport n’a pas pu être enregistré. Choisissez une autre destination.',
  exportInsideGame:
    'Enregistrez le rapport hors du dossier du jeu pour conserver l’installation intacte.',
  noReport: 'Effectuez d’abord une vérification de l’installation.',
  unauthorized: 'Cette fenêtre ne peut pas effectuer cette opération.',
  invalidResponse: 'Le service de bureau a retourné une réponse non prise en charge.',
  referenceInvalid: 'La référence de structure embarquée est invalide.',
  referenceResources: 'Ressources de la référence locale',
  referenceResourcesHint:
    'Compare les noms et tailles des ressources Data et Video hors DLC à la référence locale choisie. Le nombre affiché indique les écarts. Leur contenu n’est pas hashé.',
  referenceUnknown: 'Autre build de l’exécutable',
  referenceUnknownHint:
    'La référence locale cible une autre empreinte d’exécutable. Aucune exhaustivité des archives n’est annoncée pour ce build.',
  gameBinary: 'Exécutable du jeu',
  gameBinaryHint:
    'Vérifie la structure PE, l’architecture x86 et l’empreinte SHA-256 sans exécuter le fichier.',
  resources: 'Archives du jeu',
  resourcesHint:
    'Compte les PKZ non vides dans Data. Le contenu et l’exhaustivité des archives ne sont pas vérifiés.',
  shaders: 'Ressources de shaders',
  shadersHint: 'Vérifie que Data/Shaders contient des fichiers non vides.',
  gameLibrary: 'Bibliothèque du jeu',
  gameLibraryHint:
    'Vérifie la présence d’un steam_api.dll non vide. Sa présence ne valide pas son origine ou sa compatibilité.',
  graphicsPresets: 'Préréglages graphiques',
  graphicsPresetsHint: 'Vérifie la présence de GraphicsPreset.xml. Aucun préréglage n’est modifié.',
  videoDetection: 'Composant de détection vidéo',
  videoDetectionHint: 'Vérifie VideoDetection.dll sans le charger.',
  wrapper: 'Wrapper Direct3D facultatif',
  wrapperHint: 'Un d3d9.dll local nécessite sa propre validation. Son absence est normale.',
  resourceIntegrity: 'Intégrité complète des ressources',
  resourceIntegrityHint:
    'Aucun manifeste universel de référence n’est disponible. La présence des fichiers ne prouve pas l’intégrité des archives.',
  binaryArchitecture: 'Architecture des exécutables locaux',
  binaryArchitectureHint:
    'Un exécutable ou une DLL diffère de l’architecture x86 du jeu. Examiner son rôle avant toute modification.',
  localBinary: 'Format des exécutables locaux',
  localBinaryHint:
    'Un exécutable ou une DLL n’a pas pu être analysé. Aucun fichier n’a été chargé ou supprimé.',
  linksSkipped: 'Fichiers liés exclus',
  linksSkippedHint:
    'Les liens symboliques et points de réanalyse ont été ignorés. L’inventaire peut être incomplet.',
  displaySettings: 'Résolution enregistrée',
  displaySettingsHint:
    'Lit Width et Height comme DWORD dans la vue 32 bits du registre utilisateur du jeu. Ce n’est pas une mesure en jeu.',
  displayAbsent: 'Réglages d’affichage absents',
  displayAbsentHint:
    'La clé d’affichage du jeu est absente. Cela ne suffit pas à conclure à une installation défectueuse.',
  displayUnreadable: 'Réglages d’affichage inaccessibles',
  displayUnreadableHint: 'La clé du registre n’a pas pu être lue. Aucune valeur n’a été modifiée.',
  direct3d: 'Direct3D 9 x86',
  direct3dHint:
    'Vérifie l’emplacement de la DLL système x86 ou signale un wrapper local à examiner. Aucune DLL n’est chargée.',
  visualCpp: 'Dépendances Visual C++ directes',
  visualCppHint:
    'Vérifie les PE x86 des bibliothèques MSVC directement importées par Game.exe et VideoDetection.dll. Le nombre indique les dépendances non résolues. Les dépendances transitives et launchers facultatifs restent hors périmètre.',
  legacyDirectx: 'Composant DirectX hérité',
  legacyDirectxHint:
    'Recherche d3dx9_43.dll dans le dossier système x86. La présence d’un fichier ne prouve pas que toutes les dépendances sont satisfaites.',
  runtimeUntested: 'Fonctionnement en jeu',
  runtimeUntestedHint:
    'Ce diagnostic n’a pas lancé le jeu. Performances, stabilité et chargement des sauvegardes ne sont pas testés.',
  windowsOnly: 'Configuration Windows',
  windowsOnlyHint: 'Ce contrôle est disponible uniquement sous Windows.',
}

export const fr = { ...baseFr, ...additions, ...saveMessages } satisfies MessageDictionary
