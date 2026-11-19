import { en as uiEn, fr as uiFr, de as uiDe, it as uiIt, es as uiEs } from './saveguardUiMessages'
export const en = {
  ...uiEn,
  saveNeedsRecovery:
    'An interrupted restoration needs your attention in Saves before the game can start.',
  saveTitle: 'Your progress, kept safe',
  saveIntro:
    'SaveGuard keeps verified copies outside the selected save folder. A verified copy is not proof that the game can load it.',
  saveChoose: 'Choose save folder',
  saveChooseHint:
    'Select the active save folder explicitly. Start with a disposable test profile; no private account folders are scanned.',
  saveEnable: 'Monitor this folder',
  saveLifecycle:
    'Monitoring runs while Arachnid is open, with verified copies before and after supervised sessions. Closing Arachnid stops protection.',
  saveKeep: 'Unpinned copies to keep',
  saveQuota: 'Storage limit (MiB)',
  saveConfigure: 'Save preferences',
  saveSnapshot: 'Create verified copy',
  saveHistory: 'Copy history',
  saveVerified: 'Copy verified · game loading not tested',
  savePin: 'Pin',
  saveUnpin: 'Unpin',
  saveRestore: 'Preview restoration',
  saveProducers: 'The game and apps that modify these files, including cloud sync, are closed.',
  saveReview:
    'Review the exact file changes before approving. A precautionary copy of the current files is pinned automatically.',
  saveApply: 'Approve this restoration',
  saveUndo: 'Undo / cancel restoration',
  saveLimit:
    'Up to 32 files, 1 MiB per file and 32 MiB per generation. Quiet for 750 ms, then checked again during copying. This does not establish semantic consistency.',
  saveEmptyHistory: 'No complete copy yet.',
  saveProtectionOff: 'Monitoring disabled',
  saveProtectionOn: 'Monitoring enabled',
  saveBusy: 'Reading or copying files',
  saveDone: 'Operation completed.',
  saveNotConfigured: 'Choose a save folder first.',
  savePathsOverlap: 'Source and backup folders must be separate and cannot contain each other.',
  saveSourceUnavailable: 'The save folder cannot be read. Existing copies are preserved.',
  saveUnstable:
    'Files are changing or locked. No incomplete copy was published; monitoring will retry.',
  saveCorrupt: 'A copy or its metadata failed validation. No restoration was applied.',
  saveEmpty: 'The folder contains no files. No empty snapshot was published.',
  saveQuotaReached:
    'The storage limit was reached. Existing copies were preserved. Increase the limit or unpin older copies.',
  saveNoChanges: 'The selected copy already matches the current files.',
  saveProducerActive: 'Confirm that the game and all other writers are stopped before restoring.',
  saveManual: 'Manual',
  saveWatch: 'Folder change',
  saveBeforeGame: 'Before session',
  saveAfterGame: 'After session',
  saveBeforeRestore: 'Before restoration',
  saveRemoved: 'Absent',
  savePhase: 'Restoration state',
} as const

type SaveMessages = Record<keyof typeof en, string>

export const fr = {
  ...uiFr,
  saveNeedsRecovery:
    'Une restauration interrompue doit être résolue dans Sauvegardes avant de lancer le jeu.',
  saveTitle: 'Votre progression à l’abri',
  saveIntro:
    'SaveGuard conserve des copies vérifiées hors du dossier choisi. Une copie vérifiée ne prouve pas que le jeu peut la recharger.',
  saveChoose: 'Choisir le dossier de sauvegardes',
  saveChooseHint:
    'Choisissez explicitement le dossier actif. Commencez avec un profil de test jetable ; aucun compte privé n’est parcouru.',
  saveEnable: 'Surveiller ce dossier',
  saveLifecycle:
    'La surveillance fonctionne tant qu’Arachnid est ouvert, avec copies avant et après les sessions supervisées. Fermer Arachnid arrête la protection.',
  saveKeep: 'Copies non épinglées à conserver',
  saveQuota: 'Limite de stockage (Mio)',
  saveConfigure: 'Enregistrer les préférences',
  saveSnapshot: 'Créer une copie vérifiée',
  saveHistory: 'Historique des copies',
  saveVerified: 'Copie vérifiée · rechargement en jeu non testé',
  savePin: 'Épingler',
  saveUnpin: 'Désépingler',
  saveRestore: 'Examiner la restauration',
  saveProducers:
    'Le jeu et les applications qui modifient ces fichiers sont arrêtés (cloud inclus).',
  saveReview:
    'Examinez les changements exacts avant d’approuver. Une copie de précaution des fichiers actuels est automatiquement épinglée.',
  saveApply: 'Approuver cette restauration',
  saveUndo: 'Annuler / revenir avant la restauration',
  saveLimit:
    '32 fichiers au maximum, 1 Mio par fichier et 32 Mio par génération. Stabilité de 750 ms, puis nouvelle vérification pendant la copie. Cela ne garantit pas la cohérence du contenu.',
  saveEmptyHistory: 'Aucune copie complète pour le moment.',
  saveProtectionOff: 'Surveillance désactivée',
  saveProtectionOn: 'Surveillance activée',
  saveBusy: 'Lecture ou copie des fichiers',
  saveDone: 'Opération terminée.',
  saveNotConfigured: 'Choisissez d’abord le dossier de sauvegardes.',
  savePathsOverlap: 'Les dossiers source et de copies doivent être séparés et ne pas se contenir.',
  saveSourceUnavailable:
    'Le dossier de sauvegardes est illisible. Les copies existantes sont conservées.',
  saveUnstable:
    'Des fichiers changent ou sont verrouillés. Aucune copie incomplète publiée ; la surveillance réessaiera.',
  saveCorrupt:
    'Une copie ou ses métadonnées ne passent pas la vérification. Aucune restauration appliquée.',
  saveEmpty: 'Le dossier ne contient aucun fichier. Aucune copie vide publiée.',
  saveQuotaReached:
    'Limite de stockage atteinte. Les copies existantes sont conservées. Augmentez la limite ou désépinglez d’anciennes copies.',
  saveNoChanges: 'Cette copie correspond déjà aux fichiers actuels.',
  saveProducerActive: 'Confirmez l’arrêt du jeu et des autres producteurs avant de restaurer.',
  saveManual: 'Manuelle',
  saveWatch: 'Changement du dossier',
  saveBeforeGame: 'Avant la partie',
  saveAfterGame: 'Après la partie',
  saveBeforeRestore: 'Avant restauration',
  saveRemoved: 'Absent',
  savePhase: 'État de restauration',
} satisfies SaveMessages

export const de = {
  ...uiDe,
  saveNeedsRecovery:
    'Eine unterbrochene Wiederherstellung muss unter Spielstände abgeschlossen werden, bevor das Spiel startet.',
  saveTitle: 'Dein Fortschritt, sicher aufbewahrt',
  saveIntro:
    'SaveGuard speichert geprüfte Kopien außerhalb des gewählten Ordners. Eine geprüfte Kopie ist kein Nachweis, dass das Spiel sie laden kann.',
  saveChoose: 'Spielstandordner wählen',
  saveChooseHint:
    'Wähle den aktiven Ordner ausdrücklich. Beginne mit einem entbehrlichen Testprofil; private Konten werden nicht durchsucht.',
  saveEnable: 'Diesen Ordner überwachen',
  saveLifecycle:
    'Die Überwachung läuft, solange Arachnid geöffnet ist, mit Kopien vor und nach überwachten Sitzungen. Beim Schließen endet der Schutz.',
  saveKeep: 'Nicht angeheftete Kopien behalten',
  saveQuota: 'Speicherlimit (MiB)',
  saveConfigure: 'Einstellungen speichern',
  saveSnapshot: 'Geprüfte Kopie erstellen',
  saveHistory: 'Kopienverlauf',
  saveVerified: 'Kopie geprüft · Laden im Spiel nicht getestet',
  savePin: 'Anheften',
  saveUnpin: 'Lösen',
  saveRestore: 'Wiederherstellung prüfen',
  saveProducers:
    'Das Spiel und Apps, die diese Dateien ändern, sind beendet (einschließlich Cloud-Synchronisierung).',
  saveReview:
    'Prüfe die genauen Dateiänderungen vor der Zustimmung. Eine Sicherheitskopie der aktuellen Dateien wird automatisch angeheftet.',
  saveApply: 'Diese Wiederherstellung genehmigen',
  saveUndo: 'Wiederherstellung abbrechen / rückgängig machen',
  saveLimit:
    'Höchstens 32 Dateien, 1 MiB je Datei und 32 MiB je Generation. 750 ms Ruhe und erneute Prüfung während der Kopie garantieren keine inhaltliche Konsistenz.',
  saveEmptyHistory: 'Noch keine vollständige Kopie.',
  saveProtectionOff: 'Überwachung deaktiviert',
  saveProtectionOn: 'Überwachung aktiviert',
  saveBusy: 'Dateien werden gelesen oder kopiert',
  saveDone: 'Vorgang abgeschlossen.',
  saveNotConfigured: 'Wähle zuerst einen Spielstandordner.',
  savePathsOverlap:
    'Quell- und Sicherungsordner müssen getrennt sein und dürfen einander nicht enthalten.',
  saveSourceUnavailable: 'Spielstandordner nicht lesbar. Vorhandene Kopien bleiben erhalten.',
  saveUnstable:
    'Dateien ändern sich oder sind gesperrt. Keine unvollständige Kopie veröffentlicht; erneuter Versuch folgt.',
  saveCorrupt: 'Eine Kopie oder ihre Metadaten sind ungültig. Keine Wiederherstellung angewendet.',
  saveEmpty: 'Der Ordner enthält keine Dateien. Keine leere Kopie veröffentlicht.',
  saveQuotaReached:
    'Speicherlimit erreicht. Vorhandene Kopien bleiben erhalten. Erhöhe das Limit oder löse ältere Kopien.',
  saveNoChanges: 'Diese Kopie entspricht bereits den aktuellen Dateien.',
  saveProducerActive:
    'Bestätige vor der Wiederherstellung, dass alle Schreibprogramme beendet sind.',
  saveManual: 'Manuell',
  saveWatch: 'Ordneränderung',
  saveBeforeGame: 'Vor der Sitzung',
  saveAfterGame: 'Nach der Sitzung',
  saveBeforeRestore: 'Vor Wiederherstellung',
  saveRemoved: 'Nicht vorhanden',
  savePhase: 'Wiederherstellungsstatus',
} satisfies SaveMessages

export const it = {
  ...uiIt,
  saveNeedsRecovery: 'Risolvi il ripristino interrotto in Salvataggi prima di avviare il gioco.',
  saveTitle: 'I tuoi progressi al sicuro',
  saveIntro:
    'SaveGuard conserva copie verificate fuori dalla cartella scelta. Una copia verificata non dimostra che il gioco possa caricarla.',
  saveChoose: 'Scegli cartella dei salvataggi',
  saveChooseHint:
    'Scegli esplicitamente la cartella attiva. Inizia con un profilo di prova eliminabile; nessun account privato viene esaminato.',
  saveEnable: 'Monitora questa cartella',
  saveLifecycle:
    'Il monitoraggio funziona mentre Arachnid è aperto, con copie prima e dopo le sessioni supervisionate. Chiudere Arachnid interrompe la protezione.',
  saveKeep: 'Copie non fissate da conservare',
  saveQuota: 'Limite di spazio (MiB)',
  saveConfigure: 'Salva preferenze',
  saveSnapshot: 'Crea copia verificata',
  saveHistory: 'Cronologia delle copie',
  saveVerified: 'Copia verificata · caricamento nel gioco non testato',
  savePin: 'Fissa',
  saveUnpin: 'Sblocca',
  saveRestore: 'Esamina ripristino',
  saveProducers:
    'Il gioco e le app che modificano questi file sono chiusi, inclusa la sincronizzazione cloud.',
  saveReview:
    'Esamina le modifiche esatte prima di approvare. Una copia precauzionale dei file attuali viene fissata automaticamente.',
  saveApply: 'Approva questo ripristino',
  saveUndo: 'Annulla / torna a prima del ripristino',
  saveLimit:
    'Massimo 32 file, 1 MiB per file e 32 MiB per generazione. Stabilità di 750 ms e nuova verifica durante la copia non garantiscono la coerenza del contenuto.',
  saveEmptyHistory: 'Nessuna copia completa.',
  saveProtectionOff: 'Monitoraggio disattivato',
  saveProtectionOn: 'Monitoraggio attivo',
  saveBusy: 'Lettura o copia dei file',
  saveDone: 'Operazione completata.',
  saveNotConfigured: 'Scegli prima la cartella dei salvataggi.',
  savePathsOverlap: 'Le cartelle sorgente e di backup devono essere separate e non contenersi.',
  saveSourceUnavailable: 'Cartella dei salvataggi illeggibile. Le copie esistenti sono conservate.',
  saveUnstable:
    'I file cambiano o sono bloccati. Nessuna copia incompleta pubblicata; seguirà un nuovo tentativo.',
  saveCorrupt: 'Una copia o i suoi metadati non sono validi. Nessun ripristino applicato.',
  saveEmpty: 'La cartella non contiene file. Nessuna copia vuota pubblicata.',
  saveQuotaReached:
    'Limite di spazio raggiunto. Copie esistenti conservate. Aumenta il limite o sblocca le copie precedenti.',
  saveNoChanges: 'Questa copia corrisponde già ai file attuali.',
  saveProducerActive:
    'Conferma la chiusura del gioco e degli altri programmi di scrittura prima del ripristino.',
  saveManual: 'Manuale',
  saveWatch: 'Modifica della cartella',
  saveBeforeGame: 'Prima della sessione',
  saveAfterGame: 'Dopo la sessione',
  saveBeforeRestore: 'Prima del ripristino',
  saveRemoved: 'Assente',
  savePhase: 'Stato del ripristino',
} satisfies SaveMessages

export const es = {
  ...uiEs,
  saveNeedsRecovery: 'Resuelve la restauración interrumpida en Partidas antes de iniciar el juego.',
  saveTitle: 'Tu progreso, a salvo',
  saveIntro:
    'SaveGuard guarda copias verificadas fuera de la carpeta elegida. Una copia verificada no demuestra que el juego pueda cargarla.',
  saveChoose: 'Elegir carpeta de partidas',
  saveChooseHint:
    'Elige explícitamente la carpeta activa. Empieza con un perfil de prueba desechable; no se examinan cuentas privadas.',
  saveEnable: 'Vigilar esta carpeta',
  saveLifecycle:
    'La vigilancia funciona mientras Arachnid está abierto, con copias antes y después de las sesiones supervisadas. Cerrar Arachnid detiene la protección.',
  saveKeep: 'Copias sin fijar que conservar',
  saveQuota: 'Límite de espacio (MiB)',
  saveConfigure: 'Guardar preferencias',
  saveSnapshot: 'Crear copia verificada',
  saveHistory: 'Historial de copias',
  saveVerified: 'Copia verificada · carga en el juego sin probar',
  savePin: 'Fijar',
  saveUnpin: 'Desfijar',
  saveRestore: 'Revisar restauración',
  saveProducers:
    'El juego y las aplicaciones que modifican estos archivos están cerrados, incluida la sincronización en la nube.',
  saveReview:
    'Revisa los cambios exactos antes de aprobar. Se fija automáticamente una copia preventiva de los archivos actuales.',
  saveApply: 'Aprobar esta restauración',
  saveUndo: 'Cancelar / deshacer restauración',
  saveLimit:
    'Máximo 32 archivos, 1 MiB por archivo y 32 MiB por generación. La estabilidad de 750 ms y la revisión durante la copia no garantizan la coherencia del contenido.',
  saveEmptyHistory: 'Todavía no hay copias completas.',
  saveProtectionOff: 'Vigilancia desactivada',
  saveProtectionOn: 'Vigilancia activada',
  saveBusy: 'Leyendo o copiando archivos',
  saveDone: 'Operación completada.',
  saveNotConfigured: 'Elige primero la carpeta de partidas.',
  savePathsOverlap: 'Las carpetas de origen y copias deben estar separadas y no contenerse.',
  saveSourceUnavailable:
    'No se puede leer la carpeta de partidas. Las copias existentes se conservan.',
  saveUnstable:
    'Los archivos cambian o están bloqueados. No se publicó ninguna copia incompleta; se reintentará.',
  saveCorrupt: 'Una copia o sus metadatos no superan la validación. No se aplicó la restauración.',
  saveEmpty: 'La carpeta no contiene archivos. No se publicó una copia vacía.',
  saveQuotaReached:
    'Límite de espacio alcanzado. Copias existentes conservadas. Aumenta el límite o desfija copias antiguas.',
  saveNoChanges: 'Esta copia ya coincide con los archivos actuales.',
  saveProducerActive:
    'Confirma que el juego y los demás programas de escritura están cerrados antes de restaurar.',
  saveManual: 'Manual',
  saveWatch: 'Cambio de carpeta',
  saveBeforeGame: 'Antes de la sesión',
  saveAfterGame: 'Después de la sesión',
  saveBeforeRestore: 'Antes de restaurar',
  saveRemoved: 'Ausente',
  savePhase: 'Estado de restauración',
} satisfies SaveMessages
