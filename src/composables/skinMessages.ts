import { locale } from './i18n'

const en = {
  title: 'Costumes',
  hint: 'Choose one texture pack, or keep the original costume. The game must be closed to apply changes.',
  original: 'Original game textures',
  label: 'Texture pack',
  review: 'Apply this costume?',
  explanation:
    'Arachnid installs its texture loader without changing Game.exe or your graphics mods. Choosing the original textures removes the loader.',
  ready: 'Costume applied. It will load on the next launch.',
  testing: 'Local preview · compatibility with TASM 1 is being tested.',
  unavailable: 'Local costume resources are unavailable.',
}
type Messages = Record<keyof typeof en, string>
const fr: Messages = {
  title: 'Costumes',
  hint: 'Choisissez un pack de textures ou conservez le costume d’origine. Le jeu doit être fermé pour appliquer les changements.',
  original: 'Textures d’origine du jeu',
  label: 'Pack de textures',
  review: 'Appliquer ce costume ?',
  explanation:
    'Arachnid installe son chargeur de textures sans modifier Game.exe ni vos mods graphiques. Choisir les textures d’origine retire le chargeur.',
  ready: 'Costume appliqué. Il sera chargé au prochain lancement.',
  testing: 'Aperçu local · compatibilité avec TASM 1 en cours de test.',
  unavailable: 'Les ressources locales des costumes sont indisponibles.',
}
const de: Messages = {
  title: 'Kostüme',
  hint: 'Wähle ein Texturpaket oder behalte das Originalkostüm. Das Spiel muss zum Anwenden geschlossen sein.',
  original: 'Originaltexturen des Spiels',
  label: 'Texturpaket',
  review: 'Dieses Kostüm anwenden?',
  explanation:
    'Arachnid installiert den Texturlader, ohne Game.exe oder Grafikmods zu ändern. Die Originaltexturen entfernen den Lader.',
  ready: 'Kostüm angewendet. Es wird beim nächsten Start geladen.',
  testing: 'Lokale Vorschau · Kompatibilität mit TASM 1 wird getestet.',
  unavailable: 'Lokale Kostümdateien sind nicht verfügbar.',
}
const it: Messages = {
  title: 'Costumi',
  hint: 'Scegli un pacchetto di texture o conserva il costume originale. Chiudi il gioco prima di applicare le modifiche.',
  original: 'Texture originali del gioco',
  label: 'Pacchetto di texture',
  review: 'Applicare questo costume?',
  explanation:
    'Arachnid installa il caricatore di texture senza modificare Game.exe o le mod grafiche. Scegliendo le texture originali, il caricatore viene rimosso.',
  ready: 'Costume applicato. Verrà caricato al prossimo avvio.',
  testing: 'Anteprima locale · compatibilità con TASM 1 in fase di test.',
  unavailable: 'Le risorse locali dei costumi non sono disponibili.',
}
const es: Messages = {
  title: 'Trajes',
  hint: 'Elige un paquete de texturas o conserva el traje original. Cierra el juego antes de aplicar los cambios.',
  original: 'Texturas originales del juego',
  label: 'Paquete de texturas',
  review: '¿Aplicar este traje?',
  explanation:
    'Arachnid instala el cargador de texturas sin modificar Game.exe ni los mods gráficos. Elegir las texturas originales elimina el cargador.',
  ready: 'Traje aplicado. Se cargará en el próximo inicio.',
  testing: 'Vista previa local · compatibilidad con TASM 1 en pruebas.',
  unavailable: 'Los recursos locales de los trajes no están disponibles.',
}
export function skinText(key: keyof Messages): string {
  return { en, fr, de, it, es }[locale.value][key]
}
