import type { Entry } from '../engine';

/** Smaller pieces found by walking every screen: card text, status lines, shortcuts, weekdays, drives. */
export const MISC: Entry[] = [
  // Sidebar
  ['tracking active', 'registro ativo', 'registro activo'],
  ['studio hub', 'central de ferramentas', 'centro de herramientas'],

  // Home cards
  ['Grab videos and music with queue, history and format choice.', 'Baixe vídeos e músicas com fila, histórico e escolha de formato.', 'Descarga vídeos y música con cola, historial y elección de formato.'],
  ['See active hours for your daily apps, creative tools and games.', 'Veja as horas ativas dos seus apps do dia a dia, ferramentas criativas e jogos.', 'Mira las horas activas de tus apps diarias, herramientas creativas y juegos.'],
  ['Visual treemap, sunburst, duplicates and deep space cleanup.', 'Mapa de árvore visual, gráfico radial, duplicados e limpeza profunda de espaço.', 'Mapa de árbol visual, gráfico radial, duplicados y limpieza profunda de espacio.'],
  ['Convert images, audio, video and Office documents locally.', 'Converta imagens, áudio, vídeo e documentos do Office localmente.', 'Convierte imágenes, audio, vídeo y documentos de Office localmente.'],
  ['Preview-first folder sorting with full one-click undo.', 'Organização de pastas com prévia primeiro e desfazer total em um clique.', 'Orden de carpetas con vista previa primero y deshacer total en un clic.'],

  // Weekdays and months used in charts
  ['Mon', 'Seg', 'Lun'],
  ['Tue', 'Ter', 'Mar'],
  ['Wed', 'Qua', 'Mié'],
  ['Thu', 'Qui', 'Jue'],
  ['Fri', 'Sex', 'Vie'],
  ['Sat', 'Sáb', 'Sáb'],
  ['Sun', 'Dom', 'Dom'],
  ['Mon: {}', 'Seg: {}', 'Lun: {}'],
  ['Tue: {}', 'Ter: {}', 'Mar: {}'],
  ['Wed: {}', 'Qua: {}', 'Mié: {}'],
  ['Thu: {}', 'Qui: {}', 'Jue: {}'],
  ['Fri: {}', 'Sex: {}', 'Vie: {}'],
  ['Sat: {}', 'Sáb: {}', 'Sáb: {}'],
  ['Sun: {}', 'Dom: {}', 'Dom: {}'],
  ['Jan', 'Jan', 'Ene'],
  ['Feb', 'Fev', 'Feb'],
  ['Mar', 'Mar', 'Mar'],
  ['Apr', 'Abr', 'Abr'],
  ['May', 'Mai', 'May'],
  ['Jun', 'Jun', 'Jun'],
  ['Jul', 'Jul', 'Jul'],
  ['Aug', 'Ago', 'Ago'],
  ['Sep', 'Set', 'Sep'],
  ['Oct', 'Out', 'Oct'],
  ['Nov', 'Nov', 'Nov'],
  ['Dec', 'Dez', 'Dic'],

  // Organizer / words
  ['off', 'desligada', 'desactivada'],
  ['on', 'ligada', 'activada'],

  // Disk Analyzer start screen
  ['Ask Windows for Administrator permission to read protected folders', 'Pedir ao Windows permissão de administrador para ler pastas protegidas', 'Pedir a Windows permiso de administrador para leer carpetas protegidas'],
  ['Network drive · {}', 'Unidade de rede · {}', 'Unidad de red · {}'],
  ['SSD · {}', 'SSD · {}', 'SSD · {}'],
  ['HDD · {}', 'HDD · {}', 'HDD · {}'],
  ['Fixed disk · {}', 'Disco fixo · {}', 'Disco fijo · {}'],
  ['Removable · {}', 'Removível · {}', 'Extraíble · {}'],
  ['Video · {}', 'Vídeo · {}', 'Vídeo · {}'],
  ['Audio · {}', 'Áudio · {}', 'Audio · {}'],

  // Browser settings
  ['Connected', 'Conectado', 'Conectado'],
  ["from the extension's", 'da pasta da extensão', 'de la carpeta de la extensión'],

  // Shortcuts
  ['Quick download from the clipboard', 'Download rápido a partir da área de transferência', 'Descarga rápida desde el portapapeles'],
  ['Open NomNom', 'Abrir o NomNom', 'Abrir NomNom'],
  ['{} is used by another app. Pick a different shortcut.', '{} já é usado por outro app. Escolha outro atalho.', '{} ya lo usa otra app. Elige otro atajo.'],
  ['Quick download from the clipboard: {}. Press to change.', 'Download rápido a partir da área de transferência: {}. Pressione para mudar.', 'Descarga rápida desde el portapapeles: {}. Pulsa para cambiar.'],
  ['Open NomNom: {}. Press to change.', 'Abrir o NomNom: {}. Pressione para mudar.', 'Abrir NomNom: {}. Pulsa para cambiar.'],
  ['Press the new keys…', 'Pressione as novas teclas…', 'Pulsa las teclas nuevas…'],
  ['Hold Ctrl, Alt, Shift or Win while you press the key.', 'Segure Ctrl, Alt, Shift ou Win enquanto pressiona a tecla.', 'Mantén Ctrl, Alt, Shift o Win mientras pulsas la tecla.'],

  // System
  ["Off. NomNom isn't in the right-click menu.", 'Desligado. O NomNom não está no menu de clique direito.', 'Apagado. NomNom no está en el menú de clic derecho.'],
  ['In the Windows 11 right-click menu.', 'No menu de clique direito do Windows 11.', 'En el menú de clic derecho de Windows 11.'],
  ['Installed for KDE Dolphin, GNOME Files (Nautilus) and Nemo.', 'Instalado para KDE Dolphin, Arquivos do GNOME (Nautilus) e Nemo.', 'Instalado para KDE Dolphin, Archivos de GNOME (Nautilus) y Nemo.'],
  ['In the classic menu: right-click, then choose Show more options.', 'No menu clássico: clique com o botão direito e escolha Mostrar mais opções.', 'En el menú clásico: clic derecho y luego elige Mostrar más opciones.'],
  ['Turned on, but not added to the menu yet. Try Repair.', 'Ligado, mas ainda não adicionado ao menu. Tente Reparar.', 'Activado, pero aún no añadido al menú. Prueba Reparar.'],
  ['NomNom starts only when you open it.', 'O NomNom só inicia quando você o abre.', 'NomNom solo se inicia cuando lo abres.'],
  ['NomNom starts on login, in the tray, without a window.', 'O NomNom inicia ao entrar, na bandeja, sem janela.', 'NomNom se inicia al iniciar sesión, en la bandeja, sin ventana.'],
  ['NomNom starts with Windows, in the tray, without a window.', 'O NomNom inicia com o Windows, na bandeja, sem janela.', 'NomNom se inicia con Windows, en la bandeja, sin ventana.'],
  ['Start NomNom on login', 'Iniciar o NomNom ao entrar', 'Iniciar NomNom al iniciar sesión'],
];
