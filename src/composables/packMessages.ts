import { locale, t } from './i18n'
const en = {
  update:
    'A replacement pack is ready. Deactivate this version, then reactivate the single pack to review the new files.',
  candidate: 'Local test pack · compatibility still to be confirmed in game',
  note: 'Fix Enhancers, DXVK and ReShade are managed together. ReShade starts through Vulkan only for this game session. No shader preset is included in this first test.',
  preference: 'Use the performance pack',
  active: 'Installed · three components configured together',
  inactive: 'Not installed',
  reviewEnable: 'Review activation',
  reviewDisable: 'Review deactivation',
  reviewSync: 'Review display synchronisation',
  reviewRecover: 'Review recovery',
  review: 'Review these file changes',
  reviewHint:
    'The game must be closed. Arachnid keeps verified copies of the original files. Approval applies only to this exact operation.',
  approve: 'Approve these changes',
  cancel: 'Cancel',
  absent: 'Absent',
  before: 'Before',
  after: 'After',
  done: 'The operation is complete.',
  failed: 'The operation could not be completed. Check the pack state before trying again.',
  conflict:
    'A pack file changed outside Arachnid. It has been preserved; automatic replacement is blocked.',
  sync: 'The native display settings changed. Review synchronisation before playing with the pack.',
  recovery: 'An interrupted pack operation needs recovery before the game can start.',
  stopped: 'Close the game before modifying the pack.',
  unsupported: 'This first candidate supports the tested game build and 16:9 resolutions only.',
}
type Messages = Record<keyof typeof en, string>
const fr: Messages = {
  update:
    'Un nouveau candidat du pack est prêt. Désactivez cette version, puis réactivez le pack unique pour examiner les nouveaux fichiers.',
  candidate: 'Pack de test local · compatibilité à confirmer en jeu',
  note: 'Fix Enhancers, DXVK et ReShade se gèrent ensemble. ReShade utilise Vulkan uniquement pendant cette session du jeu. Aucun preset de shaders n’est inclus dans ce premier essai.',
  preference: 'Utiliser le pack de performance',
  active: 'Installé · trois composants configurés ensemble',
  inactive: 'Non installé',
  reviewEnable: 'Examiner l’activation',
  reviewDisable: 'Examiner la désactivation',
  reviewSync: 'Examiner la synchronisation de l’affichage',
  reviewRecover: 'Examiner la récupération',
  review: 'Examiner ces modifications de fichiers',
  reviewHint:
    'Le jeu doit être fermé. Arachnid conserve des copies vérifiées des fichiers d’origine. L’accord concerne uniquement cette opération précise.',
  approve: 'Approuver ces modifications',
  cancel: 'Annuler',
  absent: 'Absent',
  before: 'Avant',
  after: 'Après',
  done: 'L’opération est terminée.',
  failed: 'L’opération n’a pas pu aboutir. Vérifiez l’état du pack avant de réessayer.',
  conflict:
    'Un fichier du pack a changé hors d’Arachnid. Il est conservé ; le remplacement automatique est bloqué.',
  sync: 'Les réglages d’affichage natifs ont changé. Examinez leur synchronisation avant de jouer avec le pack.',
  recovery: 'Une opération interrompue sur le pack doit être récupérée avant de lancer le jeu.',
  stopped: 'Fermez le jeu avant de modifier le pack.',
  unsupported:
    'Ce premier candidat prend en charge le build testé et les résolutions 16:9 uniquement.',
}
const de: Messages = {
  update:
    'Ein neues Paket ist bereit. Deaktiviere diese Version und aktiviere anschließend das gesamte Paket erneut, um die neuen Dateien zu prüfen.',
  candidate: 'Lokales Testpaket · Kompatibilität im Spiel noch zu prüfen',
  note: 'Fix Enhancers, DXVK und ReShade werden gemeinsam verwaltet. ReShade nutzt Vulkan nur für diese Spielsitzung. Der erste Test enthält keine Shader-Voreinstellung.',
  preference: 'Leistungspaket verwenden',
  active: 'Installiert · drei Komponenten gemeinsam eingerichtet',
  inactive: 'Nicht installiert',
  reviewEnable: 'Aktivierung prüfen',
  reviewDisable: 'Deaktivierung prüfen',
  reviewSync: 'Anzeigeabgleich prüfen',
  reviewRecover: 'Wiederherstellung prüfen',
  review: 'Dateiänderungen prüfen',
  reviewHint:
    'Das Spiel muss geschlossen sein. Arachnid bewahrt geprüfte Originalkopien auf. Die Zustimmung gilt nur für diesen Vorgang.',
  approve: 'Änderungen genehmigen',
  cancel: 'Abbrechen',
  absent: 'Nicht vorhanden',
  before: 'Vorher',
  after: 'Nachher',
  done: 'Vorgang abgeschlossen.',
  failed: 'Der Vorgang konnte nicht abgeschlossen werden. Prüfe den Paketstatus.',
  conflict:
    'Eine Paketdatei wurde außerhalb von Arachnid geändert. Sie bleibt erhalten; automatisches Ersetzen ist gesperrt.',
  sync: 'Die Anzeigeeinstellungen haben sich geändert. Prüfe den Abgleich vor dem Spielen.',
  recovery: 'Ein unterbrochener Paketvorgang muss vor dem Spielstart wiederhergestellt werden.',
  stopped: 'Schließe das Spiel, bevor du das Paket änderst.',
  unsupported: 'Dieser erste Kandidat unterstützt nur den getesteten Build und 16:9-Auflösungen.',
}
const it: Messages = {
  update:
    'Un nuovo pacchetto è pronto. Disattiva questa versione, poi riattiva il pacchetto unico per esaminare i nuovi file.',
  candidate: 'Pacchetto di prova locale · compatibilità da verificare nel gioco',
  note: 'Fix Enhancers, DXVK e ReShade vengono gestiti insieme. ReShade usa Vulkan solo per questa sessione. La prima prova non include preset shader.',
  preference: 'Usa il pacchetto prestazioni',
  active: 'Installato · tre componenti configurati insieme',
  inactive: 'Non installato',
  reviewEnable: 'Esamina attivazione',
  reviewDisable: 'Esamina disattivazione',
  reviewSync: 'Esamina sincronizzazione video',
  reviewRecover: 'Esamina recupero',
  review: 'Esamina le modifiche ai file',
  reviewHint:
    'Il gioco deve essere chiuso. Arachnid conserva copie verificate degli originali. L’approvazione vale solo per questa operazione.',
  approve: 'Approva le modifiche',
  cancel: 'Annulla',
  absent: 'Assente',
  before: 'Prima',
  after: 'Dopo',
  done: 'Operazione completata.',
  failed: 'Impossibile completare l’operazione. Controlla lo stato del pacchetto.',
  conflict:
    'Un file è stato modificato fuori da Arachnid. È conservato; la sostituzione automatica è bloccata.',
  sync: 'Le impostazioni video sono cambiate. Esamina la sincronizzazione prima di giocare.',
  recovery: 'Un’operazione interrotta richiede il recupero prima di avviare il gioco.',
  stopped: 'Chiudi il gioco prima di modificare il pacchetto.',
  unsupported: 'Questo primo candidato supporta solo la build testata e le risoluzioni 16:9.',
}
const es: Messages = {
  update:
    'Hay un nuevo paquete listo. Desactiva esta versión y vuelve a activar el paquete único para examinar los nuevos archivos.',
  candidate: 'Paquete de prueba local · compatibilidad pendiente de verificar en el juego',
  note: 'Fix Enhancers, DXVK y ReShade se gestionan juntos. ReShade usa Vulkan solo durante esta sesión. La primera prueba no incluye un preset de shaders.',
  preference: 'Usar el paquete de rendimiento',
  active: 'Instalado · tres componentes configurados juntos',
  inactive: 'No instalado',
  reviewEnable: 'Examinar activación',
  reviewDisable: 'Examinar desactivación',
  reviewSync: 'Examinar sincronización de pantalla',
  reviewRecover: 'Examinar recuperación',
  review: 'Examinar cambios de archivos',
  reviewHint:
    'El juego debe estar cerrado. Arachnid conserva copias verificadas de los originales. La aprobación solo vale para esta operación.',
  approve: 'Aprobar cambios',
  cancel: 'Cancelar',
  absent: 'Ausente',
  before: 'Antes',
  after: 'Después',
  done: 'Operación completada.',
  failed: 'No se pudo completar la operación. Revisa el estado del paquete.',
  conflict:
    'Un archivo cambió fuera de Arachnid. Se conserva; la sustitución automática está bloqueada.',
  sync: 'Los ajustes de pantalla cambiaron. Examina su sincronización antes de jugar.',
  recovery: 'Una operación interrumpida requiere recuperación antes de iniciar el juego.',
  stopped: 'Cierra el juego antes de modificar el paquete.',
  unsupported: 'Este primer candidato solo admite la build probada y resoluciones 16:9.',
}
export function packText(key: keyof Messages): string {
  return { en, fr, de, it, es }[locale.value][key]
}
export function packError(code: string): string {
  if (code === 'modPackUpdateRequired') return packText('update')
  if (['modOwnedFileChanged', 'modUnmanagedFiles', 'rollbackConflict'].includes(code))
    return packText('conflict')
  if (code === 'modSettingsOutOfSync') return packText('sync')
  if (code === 'recoveryRequired') return packText('recovery')
  if (['gameAlreadyRunning', 'gameRunning'].includes(code)) return packText('stopped')
  if (['modDisplayUnsupported', 'modBuildUnsupported'].includes(code))
    return packText('unsupported')
  if (['invalidModState', 'invalidModPayload', 'modFileUnreadable'].includes(code))
    return `${packText('failed')} (${code})`
  const message = t(code)
  return message === t('unknownError') ? `${packText('failed')} (${code})` : message
}
