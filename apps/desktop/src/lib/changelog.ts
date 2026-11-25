import type { Lang } from './i18n/engine';

export type ChangeKind = 'added' | 'improved' | 'fixed' | 'design';

export interface LocalizedText {
  en: string;
  pt: string;
  es: string;
}

export interface ReleaseChange {
  kind: ChangeKind;
  title: LocalizedText;
  detail: LocalizedText;
  toolId?: string;
  toolLabel?: string;
}

export interface ReleaseNote {
  version: string;
  date: string;
  codename: LocalizedText;
  summary: LocalizedText;
  highlights: LocalizedText[];
  changes: ReleaseChange[];
  latest?: boolean;
}

/** Picks the best localized string for the active UI language. */
export function pickText(text: LocalizedText, lang: Lang): string {
  return text[lang] ?? text.en;
}

/** Complete, chronological release history for NomNom (newest first). */
export const RELEASES: ReleaseNote[] = [
  {
    version: '0.7.1',
    date: '2026-11-25',
    latest: true,
    codename: {
      en: 'Patch Notes, Tool Sets, Complete Guide & Responsive Polish',
      pt: 'Patch Notes, Conjuntos de Ferramentas, Guia Completo e Menu Responsivo',
      es: 'Notas de Versión, Conjuntos de Herramientas, Guía Completa y Menú Adaptable',
    },
    summary: {
      en: 'Adds a dedicated Patch Notes hub, 8 workflow Tool Sets, a step-by-step Guide for all 28 tools, sharper interface corners, a responsive navigation menu for every resolution and 20 full-app reliability fixes.',
      pt: 'Adiciona uma aba dedicada de Patch Notes, 8 conjuntos prontos de ferramentas por perfil de uso, guia passo a passo de todas as 28 ferramentas, cantos menos arredondados, menu responsivo para qualquer resolução e 20 correções de estabilidade.',
      es: 'Añade una pestaña dedicada de Notas de Versión, 8 conjuntos de herramientas por perfil, guía paso a paso de las 28 herramientas, bordes más rectos, menú adaptable a cualquier resolución y 20 correcciones de estabilidad.',
    },
    highlights: [
      { en: 'Dedicated Patch Notes tab with filters & search', pt: 'Aba dedicada de Patch Notes com filtros e busca', es: 'Pestaña de Notas de Versión con filtros y búsqueda' },
      { en: 'Responsive menu across all 6 theme layouts', pt: 'Menu responsivo em todos os 6 layouts de tema', es: 'Menú adaptable en los 6 diseños de tema' },
      { en: '8 Tool Sets & complete 28-tool Guide', pt: '8 conjuntos de ferramentas e guia das 28 funções', es: '8 conjuntos y guía completa de 28 herramientas' },
      { en: '20 backend & UI reliability fixes', pt: '20 correções de estabilidade no backend e UI', es: '20 correcciones de estabilidad en backend y UI' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Dedicated Patch Notes tab', pt: 'Aba dedicada de Patch Notes', es: 'Pestaña dedicada de Notas de Versión' },
        detail: {
          en: 'Browse every NomNom release from 0.1.0 to today with version jump navigation, category filters, keyword search and direct links to open each tool.',
          pt: 'Consulte todas as versões do NomNom desde a 0.1.0 até hoje com índice lateral de versões, filtros por tipo, busca por palavra-chave e atalhos diretos para abrir cada ferramenta.',
          es: 'Consulta cada versión de NomNom desde la 0.1.0 hasta hoy con índice de versiones, filtros por tipo, búsqueda y enlaces directos para abrir cada herramienta.',
        },
        toolId: 'changelog',
        toolLabel: 'Patch notes',
      },
      {
        kind: 'added',
        title: { en: '8 curated Tool Sets', pt: '8 conjuntos prontos de ferramentas', es: '8 conjuntos de herramientas listos' },
        detail: {
          en: 'Switch your sidebar in one click between Essentials, Creator & Media, Developer & Power, Gamer & Focus, Work & Study, Cleanup & Storage, Phone & Wireless or All tools — available on Tools and First Run.',
          pt: 'Alterne a barra lateral em um clique entre Essencial, Criador e Mídia, Dev e Avançado, Gamer e Foco, Trabalho e Estudo, Limpeza e Espaço, Celular e Sem Fio ou Todas as ferramentas — na aba Tools e na configuração inicial.',
          es: 'Cambia la barra lateral en un clic entre Esencial, Creador y Medios, Dev y Avanzado, Gamer y Enfoque, Trabajo y Estudio, Limpieza y Espacio, Móvil e Inalámbrico o Todas las herramientas.',
        },
        toolId: 'tools',
        toolLabel: 'Tools',
      },
      {
        kind: 'added',
        title: { en: 'Complete interactive NomNom Guide', pt: 'Guia interativo completo do NomNom', es: 'Guía interactiva completa de NomNom' },
        detail: {
          en: 'Step-by-step instructions, tips, keyboard shortcuts and requirements for all 28 tools plus 4 core workflows. Open it from any tool card, the Tools header, Home or Ctrl+K.',
          pt: 'Instruções passo a passo, dicas, atalhos e requisitos para todas as 28 ferramentas e 4 fluxos centrais. Abra por qualquer card de ferramenta, pelo topo da aba Tools, pela Home ou pelo Ctrl+K.',
          es: 'Instrucciones paso a paso, consejos, atajos y requisitos para las 28 herramientas y 4 flujos clave. Ábrela desde cualquier tarjeta, la cabecera de Tools, Inicio o Ctrl+K.',
        },
        toolId: 'tools',
        toolLabel: 'Tools',
      },
      {
        kind: 'design',
        title: { en: 'Responsive navigation for every resolution', pt: 'Menu responsivo em todas as resoluções', es: 'Menú adaptable en todas las resoluciones' },
        detail: {
          en: 'On narrow or split-screen windows, sidebars collapse smoothly into a compact icon bar with a slide-over drawer toggle; the Top Bar adds horizontal scroll buttons and compact mode; Icon Rail slims cleanly and page padding adapts fluidly.',
          pt: 'Em janelas estreitas ou tela dividida, a barra lateral recolhe automaticamente para ícones com gaveta deslizante sob demanda; a Barra Superior ganha botões de rolagem horizontal e modo compacto; o Trilho de Ícones e as margens se adaptam à tela.',
          es: 'En ventanas estrechas o pantalla dividida, la barra lateral pasa a iconos compactos con panel desplegable; la Barra Superior añade flechas de desplazamiento horizontal y modo compacto; el Rail y los márgenes se ajustan al tamaño.',
        },
      },
      {
        kind: 'design',
        title: { en: 'Sharper, less rounded interface corners', pt: 'Design mais reto e menos arredondado', es: 'Bordes más rectos y menos redondeados' },
        detail: {
          en: 'Tightened corner radii across cards, inputs, tabs, chips and all 6 theme layouts (including Floating Island, Top Bar and Icon Rail) while keeping switches and progress bars crisp.',
          pt: 'Raios de borda reduzidos em cards, campos, abas, botões e em todos os 6 layouts de tema (incluindo Ilha Flutuante, Barra Superior e Trilho de Ícones), deixando o visual mais limpo e preciso.',
          es: 'Radios de esquina reducidos en tarjetas, campos, pestañas, botones y en los 6 diseños de tema, logrando una apariencia más limpia y definida.',
        },
        toolId: 'settings',
        toolLabel: 'Settings',
      },
      {
        kind: 'fixed',
        title: { en: 'Full-app backend & frontend reliability audit', pt: 'Auditoria completa de estabilidade (Rust e Svelte)', es: 'Auditoría completa de estabilidad (Rust y Svelte)' },
        detail: {
          en: 'Fixed 20 real edge-case bugs: UTF-8 slicing and clean scrcpy teardown in Mobile Manager; pipe deadlock guards in LibreOffice, FFmpeg, Actions and Downloads; PDF merge page order; Disk Analyzer lock safety; cross-drive Organizer undo; ZIP duplicate keys; reactive Home reminders; instant theme sync from global Ctrl+K; and 8 undefined CSS variable references.',
          pt: 'Corrigidos 20 bugs de casos extremos: slice UTF-8 e encerramento limpo do scrcpy no Mobile Manager; proteção contra deadlock de pipe no LibreOffice, FFmpeg, Ações e Downloads; ordem de páginas ao juntar PDFs; trava segura no Analisador de Disco; desfazer entre unidades no Organizador; chaves duplicadas em ZIPs; lembretes ao vivo na Home; sincronização instantânea de tema via Ctrl+K global; e 8 variáveis CSS indefinidas.',
          es: 'Corregidos 20 errores críticos: cortes UTF-8 y cierre limpio de scrcpy en Mobile Manager; prevención de bloqueos de pipe en LibreOffice, FFmpeg, Acciones y Descargas; orden de páginas al unir PDF; bloqueo seguro en Analizador de Disco; deshacer entre discos en Organizador; claves duplicadas en ZIP; recordatorios reactivos en Inicio; sincronización de tema desde Ctrl+K global; y 8 variables CSS sin definir.',
        },
      },
    ],
  },
  {
    version: '0.7.0',
    date: '2026-11-23',
    codename: {
      en: 'Mobile Manager, Global Ctrl+K & 6 Full-App Layouts',
      pt: 'Mobile Manager, Ctrl+K Global e 6 Layouts Completos',
      es: 'Mobile Manager, Ctrl+K Global y 6 Diseños Completos',
    },
    summary: {
      en: 'Introduces NomNom Mobile Manager with USB/Wi-Fi mirroring and Webcam Studio, makes Ctrl+K work outside NomNom, ties themes to 6 structural layouts and organizes all tools into categories.',
      pt: 'Estreia o NomNom Mobile Manager com espelhamento USB/Wi-Fi e Estúdio de Webcam, faz o Ctrl+K funcionar fora do NomNom, conecta temas a 6 layouts estruturais e organiza todas as ferramentas em categorias.',
      es: 'Estrena NomNom Mobile Manager con espejo USB/Wi-Fi y Estudio de Webcam, permite usar Ctrl+K fuera de NomNom, enlaza los temas a 6 diseños completos y organiza todas las herramientas por categorías.',
    },
    highlights: [
      { en: 'Android Mobile Manager + Wireless Webcam Studio', pt: 'Mobile Manager Android + Estúdio de Webcam sem fio', es: 'Mobile Manager Android + Estudio de Webcam Wi-Fi' },
      { en: 'Global Ctrl+K launcher anywhere on your PC', pt: 'Ctrl+K global em qualquer programa do PC', es: 'Lanzador global Ctrl+K en todo el PC' },
      { en: '6 structural theme layouts (Studio, Floating, Top Bar, Rail, HUD, TUI)', pt: '6 layouts de tema (Studio, Flutuante, Topo, Rail, HUD, TUI)', es: '6 diseños de tema (Studio, Flotante, Superior, Rail, HUD, TUI)' },
      { en: 'Tools page organized into 6 categories', pt: 'Página Tools organizada em 6 categorias', es: 'Página Tools organizada en 6 categorías' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'NomNom Mobile Manager (USB & Wireless)', pt: 'NomNom Mobile Manager (USB e Wi-Fi)', es: 'NomNom Mobile Manager (USB y Wi-Fi)' },
        detail: {
          en: 'Full Android companion built on ADB and scrcpy: QR wireless pairing, mDNS auto-discovery, auto-reconnect, low-latency screen mirroring, phone controls, notifications, messages, call log, photo gallery, contacts .vcf export, Files explorer, App manager, System tweaks, ADB Console, Mic forwarding and screen recording.',
          pt: 'Central completa para Android com ADB e scrcpy: pareamento Wi-Fi por QR Code, descoberta automática mDNS, reconexão, espelhamento de baixa latência, controles, notificações, mensagens, chamadas, galeria de fotos, exportação de contatos .vcf, explorador de arquivos, gerenciador de apps, ajustes de sistema, console ADB, microfone e gravação de tela.',
          es: 'Centro completo para Android con ADB y scrcpy: emparejamiento Wi-Fi por QR, detección mDNS, reconexión, espejo de baja latencia, controles, notificaciones, mensajes, llamadas, fotos, exportación de contactos .vcf, explorador de archivos, gestor de apps, ajustes, consola ADB, micrófono y grabación.',
        },
        toolId: 'mobile',
        toolLabel: 'Mobile Manager',
      },
      {
        kind: 'added',
        title: { en: 'Webcam Studio, Virtual Display & UHID Gaming', pt: 'Estúdio de Webcam, Monitor Virtual e Jogos UHID', es: 'Estudio de Webcam, Pantalla Virtual y Juegos UHID' },
        detail: {
          en: 'Use your phone camera as a high-bitrate PC webcam over USB or Wi-Fi with resolution/FPS tuning and Wi-Fi smoothing buffer, create independent Android virtual displays and control mobile games with UHID mouse/keyboard/gamepad input.',
          pt: 'Use a câmera do celular como webcam de alta qualidade via USB ou Wi-Fi com ajuste de resolução/FPS e buffer de suavização Wi-Fi, crie monitores virtuais Android e jogue com teclado, mouse ou controle via UHID.',
          es: 'Usa la cámara del móvil como webcam de alta calidad por USB o Wi-Fi con ajuste de resolución/FPS y búfer de suavizado, crea pantallas virtuales Android y juega con entrada UHID.',
        },
        toolId: 'mobile',
        toolLabel: 'Mobile Manager',
      },
      {
        kind: 'added',
        title: { en: 'Global Ctrl+K quick launcher', pt: 'Lançador global com Ctrl+K fora do app', es: 'Lanzador global con Ctrl+K fuera de la app' },
        detail: {
          en: 'Press Ctrl+K from any window to search tools, programs, files, calculations, unit conversions, reminders, notes, snippets and instant theme switching without leaving what you were doing.',
          pt: 'Pressione Ctrl+K em qualquer janela para buscar ferramentas, programas, arquivos, cálculos, conversões de unidade, lembretes, notas, snippets e troca rápida de tema sem sair do que está fazendo.',
          es: 'Pulsa Ctrl+K desde cualquier ventana para buscar herramientas, programas, archivos, cálculos, conversiones, recordatorios, notas, snippets y cambio rápido de tema.',
        },
        toolId: 'actions',
        toolLabel: 'My actions',
      },
      {
        kind: 'design',
        title: { en: '6 full-app theme layouts & calmer base UI', pt: '6 layouts estruturais de tema e interface mais calma', es: '6 diseños estructurales de tema e interfaz más serena' },
        detail: {
          en: 'Switching themes now transforms navigation structure, spacing, shadows and typography across Studio, Floating Island, Frosted Top Bar, Icon Rail, Right Cyber HUD and Terminal TUI.',
          pt: 'Trocar de tema agora muda toda a estrutura de navegação, espaçamento, sombras e tipografia entre Studio, Ilha Flutuante, Barra Superior, Trilho de Ícones, HUD Direito e Terminal TUI.',
          es: 'Cambiar de tema transforma la estructura de navegación, el espaciado, las sombras y la tipografía entre Studio, Isla Flotante, Barra Superior, Rail de Iconos, HUD Derecho y Terminal TUI.',
        },
        toolId: 'settings',
        toolLabel: 'Settings',
      },
      {
        kind: 'improved',
        title: { en: '6 categorized tool groups & sidebar filtering', pt: 'Ferramentas divididas em 6 categorias com busca e filtros', es: 'Herramientas organizadas en 6 categorías con búsqueda y filtros' },
        detail: {
          en: 'Grouped all 28 tools into Files & Storage, Media & Capture, Phone & Sharing, Productivity & Text, Focus & Gaming, and System & Developer with instant search and category toggles.',
          pt: 'Todas as 28 ferramentas agrupadas em Arquivos e Armazenamento, Mídia e Captura, Celular e Compartilhamento, Produtividade e Texto, Foco e Jogos, e Sistema e Desenvolvedor, com busca e ativação por grupo.',
          es: 'Las 28 herramientas organizadas en 6 categorías temáticas con búsqueda instantánea y activación por grupo.',
        },
        toolId: 'tools',
        toolLabel: 'Tools',
      },
      {
        kind: 'fixed',
        title: { en: 'Windows ADB 4KB pipe deadlock & scrcpy 5.0 support', pt: 'Deadlock de pipe ADB no Windows e suporte a scrcpy 5.0', es: 'Bloqueo de pipe ADB en Windows y soporte para scrcpy 5.0' },
        detail: {
          en: 'Drained stdout and stderr concurrently on Windows ADB calls to stop wireless preview freezes, added Samsung device-name lookup and updated CLI flags for scrcpy 5.0.',
          pt: 'Leitura simultânea de stdout e stderr nas chamadas ADB do Windows para eliminar travamentos no preview Wi-Fi, leitura de nome em aparelhos Samsung e compatibilidade com scrcpy 5.0.',
          es: 'Lectura simultánea de stdout y stderr en ADB en Windows para evitar bloqueos en Wi-Fi, soporte de nombres Samsung y compatibilidad con scrcpy 5.0.',
        },
        toolId: 'mobile',
        toolLabel: 'Mobile Manager',
      },
    ],
  },
  {
    version: '0.6.2',
    date: '2026-11-20',
    codename: {
      en: 'Screen Recorder, ZIP Files, Compare Folders & PATH Editor',
      pt: 'Gravador de Tela, Arquivos ZIP, Comparar Pastas e Editor de PATH',
      es: 'Grabador de Pantalla, Archivos ZIP, Comparar Carpetas y Editor de PATH',
    },
    summary: {
      en: 'Adds five everyday desktop utilities: screen recording to MP4 or GIF, safe ZIP packing/unpacking, side-by-side folder comparison, Windows PATH cleanup and search-box unit conversion.',
      pt: 'Adiciona cinco utilitários práticos: gravador de tela em MP4 ou GIF, empacotamento e extração segura de ZIP, comparação de pastas, editor do PATH do Windows e conversor de unidades na busca.',
      es: 'Añade cinco utilidades diarias: grabador de pantalla en MP4 o GIF, compresión/extracción segura de ZIP, comparación de carpetas, editor de PATH y conversor de unidades en la búsqueda.',
    },
    highlights: [
      { en: 'Screen recorder (MP4 & GIF)', pt: 'Gravador de tela (MP4 e GIF)', es: 'Grabador de pantalla (MP4 y GIF)' },
      { en: 'Safe ZIP packer & inspector', pt: 'Compactador e inspetor ZIP seguro', es: 'Compresor e inspector ZIP seguro' },
      { en: 'Side-by-side folder compare', pt: 'Comparação lado a lado de pastas', es: 'Comparación de carpetas lado a lado' },
      { en: 'Windows PATH editor & search unit converter', pt: 'Editor de PATH e conversor de unidades na busca', es: 'Editor de PATH y conversor de unidades' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Screen recorder (MP4 or GIF)', pt: 'Gravador de tela (MP4 ou GIF)', es: 'Grabador de pantalla (MP4 o GIF)' },
        detail: {
          en: 'Capture the full display or a selected area into a clean MP4 video or compact GIF using FFmpeg.',
          pt: 'Grave a tela inteira ou uma região selecionada em vídeo MP4 ou GIF leve usando o FFmpeg.',
          es: 'Graba la pantalla completa o una región en vídeo MP4 o GIF compacto con FFmpeg.',
        },
        toolId: 'recorder',
        toolLabel: 'Screen recorder',
      },
      {
        kind: 'added',
        title: { en: 'ZIP files tool', pt: 'Ferramenta de arquivos ZIP', es: 'Herramienta de archivos ZIP' },
        detail: {
          en: 'Pack files and folders into ZIP archives, inspect archive contents before extracting and unpack safely into a dedicated folder with zip-slip protection.',
          pt: 'Compacte arquivos e pastas em ZIP, inspecione o conteúdo antes de extrair e descompacte com segurança em uma pasta própria.',
          es: 'Empaqueta archivos y carpetas en ZIP, inspecciona el contenido antes de extraer y descomprime de forma segura.',
        },
        toolId: 'archives',
        toolLabel: 'ZIP files',
      },
      {
        kind: 'added',
        title: { en: 'Compare folders', pt: 'Comparar pastas', es: 'Comparar carpetas' },
        detail: {
          en: 'Diff two directories, see newer or missing files and copy only what you pick while keeping a backup of any replaced file.',
          pt: 'Compare duas pastas, veja arquivos novos ou diferentes e copie apenas o que escolher mantendo cópia de segurança dos substituídos.',
          es: 'Compara dos carpetas, revisa diferencias y copia solo lo elegido guardando copia de lo reemplazado.',
        },
        toolId: 'compare',
        toolLabel: 'Compare folders',
      },
      {
        kind: 'added',
        title: { en: 'PATH editor', pt: 'Editor de PATH', es: 'Editor de PATH' },
        detail: {
          en: 'Inspect user and system PATH folders, flag duplicate or missing entries, clean them safely and restore from automatic snapshots.',
          pt: 'Veja as pastas do PATH, identifique entradas duplicadas ou ausentes, organize tudo e restaure facilmente com cópias salvas.',
          es: 'Inspecciona las rutas de PATH, detecta duplicadas o ausentes y límpialas con copias de respaldo.',
        },
        toolId: 'paths',
        toolLabel: 'PATH editor',
      },
      {
        kind: 'added',
        title: { en: 'Instant unit conversion in search', pt: 'Conversão instantânea de unidades na busca', es: 'Conversión instantánea de unidades en la búsqueda' },
        detail: {
          en: 'Type conversions such as "12 km to mi", "32 c to f" or "500 mb to gb" directly in Ctrl+K to copy the answer.',
          pt: 'Digite conversões como "12 km to mi", "32 c to f" ou "500 mb to gb" direto no Ctrl+K para copiar o resultado.',
          es: 'Escribe conversiones como "12 km to mi", "32 c to f" o "500 mb to gb" en Ctrl+K para copiar el resultado.',
        },
      },
    ],
  },
  {
    version: '0.6.1',
    date: '2026-11-19',
    codename: {
      en: 'Screenshots Markup, Game Mode, QR Codes & Hardware Health',
      pt: 'Editor de Prints, Modo Jogo, QR Codes e Saúde do Hardware',
      es: 'Editor de Capturas, Modo Juego, Códigos QR y Salud del Hardware',
    },
    summary: {
      en: 'Brings an annotation screenshot editor, automated Game Mode, QR generator and screen scanner, reusable text snippets and GPU/battery/disk health sensors.',
      pt: 'Traz editor de capturas com anotações, Modo Jogo automático, gerador e leitor de QR Code, snippets de texto reutilizáveis e sensores de GPU, bateria e disco.',
      es: 'Incorpora editor de capturas con anotaciones, Modo Juego automático, generador y lector QR, snippets de texto y sensores de GPU, batería y disco.',
    },
    highlights: [
      { en: 'Screenshot capture & markup editor', pt: 'Captura e editor de anotações em prints', es: 'Captura y editor de anotaciones' },
      { en: 'Automated Game Mode', pt: 'Modo Jogo automático', es: 'Modo Juego automático' },
      { en: 'QR Code generator & screen reader', pt: 'Gerador e leitor de QR Code na tela', es: 'Generador y lector de códigos QR' },
      { en: 'Snippets + GPU, battery & disk health', pt: 'Snippets + saúde de GPU, bateria e disco', es: 'Snippets + salud de GPU, batería y disco' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Screenshots with markup tools', pt: 'Capturas de tela com anotações', es: 'Capturas de pantalla con anotaciones' },
        detail: {
          en: 'Capture any region and annotate with arrows, rectangles, highlighter, redaction blur, numbered callouts and text before copying or saving.',
          pt: 'Capture qualquer região da tela e marque com setas, caixas, marca-texto, ocultação de dados sensíveis, passos numerados e texto.',
          es: 'Captura cualquier área y anota con flechas, cajas, resaltador, difuminado, pasos numerados y texto.',
        },
        toolId: 'shots',
        toolLabel: 'Screenshots',
      },
      {
        kind: 'added',
        title: { en: 'Game mode', pt: 'Modo Jogo', es: 'Modo Juego' },
        detail: {
          en: 'Automatically quiets notices, switches to the fastest power plan and closes chosen background apps while a game is active, then restores everything afterward.',
          pt: 'Silencia avisos, ativa o plano de energia de alto desempenho e fecha programas em segundo plano escolhidos durante o jogo, restaurando tudo ao sair.',
          es: 'Silencia avisos, activa el plan de alto rendimiento y cierra programas elegidos mientras juegas, restaurando todo después.',
        },
        toolId: 'gamemode',
        toolLabel: 'Game mode',
      },
      {
        kind: 'added',
        title: { en: 'QR codes generator & screen scanner', pt: 'Gerador e leitor de QR Code na tela', es: 'Generador y lector de códigos QR' },
        detail: {
          en: 'Generate crisp SVG QR codes from text, links or Wi-Fi credentials, or decode any QR code currently visible on your display.',
          pt: 'Crie QR Codes em SVG a partir de texto, links ou redes Wi-Fi, ou leia qualquer QR Code exibido na tela.',
          es: 'Genera códigos QR desde texto, enlaces o Wi-Fi, o lee cualquier QR visible en pantalla.',
        },
        toolId: 'qr',
        toolLabel: 'QR codes',
      },
      {
        kind: 'added',
        title: { en: 'Text snippets with variables', pt: 'Snippets de texto com variáveis', es: 'Snippets de texto con variables' },
        detail: {
          en: 'Save reusable replies, signatures or templates with dynamic date and clipboard placeholders and paste them straight from search.',
          pt: 'Salve textos repetidos com variáveis automáticas de data e área de transferência para colar direto pela busca.',
          es: 'Guarda textos reutilizables con variables de fecha y portapapeles para pegarlos desde la búsqueda.',
        },
        toolId: 'snippets',
        toolLabel: 'Snippets',
      },
      {
        kind: 'improved',
        title: { en: 'GPU, battery & disk health in System Monitor', pt: 'Saúde de GPU, bateria e disco no Monitor do Sistema', es: 'Salud de GPU, batería y disco en el Monitor' },
        detail: {
          en: 'Added graphics card metrics, battery wear/charge stats and scheduled daily disk health checks.',
          pt: 'Adicionados dados de placa de vídeo, bateria e verificação diária de saúde dos discos.',
          es: 'Añadidos indicadores de GPU, batería y comprobación diaria de salud de discos.',
        },
        toolId: 'monitor',
        toolLabel: 'System Monitor',
      },
    ],
  },
  {
    version: '0.6.0',
    date: '2026-11-19',
    codename: {
      en: 'Command Palette, Folder Rules, Reminders & Everyday Toolbox',
      pt: 'Paleta de Comandos, Regras de Pasta, Lembretes e Caixa de Ferramentas',
      es: 'Paleta de Comandos, Reglas de Carpeta, Recordatorios y Utilidades',
    },
    summary: {
      en: 'A major productivity expansion introducing Ctrl+K search, file-index lookup, automated Folder Rules, natural-language Reminders, Quick Notes, Time tools, Text toolbox, Mini tools and Home widgets.',
      pt: 'Grande expansão de produtividade com busca Ctrl+K, busca indexada de arquivos, Regras de Pasta automáticas, Lembretes em linguagem natural, Notas, Ferramentas de Tempo, Caixa de Texto, Mini ferramentas e widgets na Home.',
      es: 'Gran expansión de productividad con búsqueda Ctrl+K, búsqueda de archivos, Reglas de Carpeta, Recordatorios naturales, Notas, Tiempo, Caja de Texto, Mini herramientas y widgets en Inicio.',
    },
    highlights: [
      { en: 'Command palette, Launcher & custom Actions', pt: 'Paleta de comandos, Lançador e Ações próprias', es: 'Paleta de comandos, Lanzador y Acciones' },
      { en: 'Automated Folder Rules with undo', pt: 'Regras de Pasta automáticas com desfazer', es: 'Reglas de Carpeta automáticas con deshacer' },
      { en: 'Natural-language Reminders & Notes', pt: 'Lembretes em texto natural e Notas rápidas', es: 'Recordatorios naturales y Notas rápidas' },
      { en: 'Time tools, Text toolbox, Mini tools & Inbox', pt: 'Tempo, Caixa de texto, Mini ferramentas e Inbox', es: 'Tiempo, Caja de texto, Mini herramientas e Inbox' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Command Palette, Launcher, Actions & File Search', pt: 'Paleta de Comandos, Lançador, Ações e Busca de Arquivos', es: 'Paleta de Comandos, Lanzador, Acciones y Búsqueda de Archivos' },
        detail: {
          en: 'Search pages, installed apps, indexed files, calculator math and your own custom actions (links, folders, programs or parameterized commands) with custom shortcuts.',
          pt: 'Busque páginas, programas, arquivos pelo índice do sistema, contas na calculadora e suas próprias ações com atalhos e parâmetros.',
          es: 'Busca páginas, programas, archivos indexados, cálculos y tus propias acciones personalizadas con atajos.',
        },
        toolId: 'actions',
        toolLabel: 'My actions',
      },
      {
        kind: 'added',
        title: { en: 'Automated Folder Rules', pt: 'Regras de Pasta automáticas', es: 'Reglas de Carpeta automáticas' },
        detail: {
          en: 'Watch folders for new files matching extensions or name patterns and automatically move, copy, rename or convert images to WebP — never deleting anything and always undoable.',
          pt: 'Monitore pastas para mover, copiar, renomear ou converter imagens em WebP automaticamente, sem nunca excluir arquivos e com desfazer completo.',
          es: 'Vigila carpetas para mover, copiar, renombrar o convertir imágenes a WebP automáticamente, siempre reversible.',
        },
        toolId: 'rules',
        toolLabel: 'Folder rules',
      },
      {
        kind: 'added',
        title: { en: 'Reminders, Notes & Time tools', pt: 'Lembretes, Notas e Ferramentas de Tempo', es: 'Recordatorios, Notas y Herramientas de Tiempo' },
        detail: {
          en: 'Type natural reminders in English, Portuguese or Spanish ("in 20 minutes stretch"), jot local notes with a shortcut, and track world clocks, time zones, stopwatches and background timers.',
          pt: 'Crie lembretes em inglês, português ou espanhol ("em 20 minutos alongar"), escreva notas locais rápidas e use relógios mundiais, fusos, cronômetro e timer.',
          es: 'Crea recordatorios naturales en español, inglés o portugués, guarda notas rápidas y usa relojes mundiales, husos, cronómetro y temporizador.',
        },
        toolId: 'reminders',
        toolLabel: 'Reminders',
      },
      {
        kind: 'added',
        title: { en: 'Text toolbox, Mini tools & Notification Inbox', pt: 'Caixa de Texto, Mini ferramentas e Histórico de Notificações', es: 'Caja de Texto, Mini herramientas e Historial de Notificaciones' },
        detail: {
          en: 'Offline JSON/Base64/URL/hash/UUID/regex/timestamp/diff utilities, always-on-top window pin, screen color picker, plain paste, link tracker cleaner, notification history and smart clipboard actions.',
          pt: 'Utilitários locais de JSON/Base64/URL/hash/UUID/regex/timestamp/diff, fixar janela no topo, conta-gotas de cor, colar sem formatação, limpar rastreadores de links, histórico de avisos e ações inteligentes no clipboard.',
          es: 'Utilidades locales de JSON/Base64/URL/hash/UUID/regex/diff, fijar ventana arriba, selector de color, pegar sin formato, limpiador de enlaces e historial de avisos.',
        },
        toolId: 'toolbox',
        toolLabel: 'Text toolbox',
      },
      {
        kind: 'added',
        title: { en: 'Home Glance widgets & settings backup', pt: 'Widgets ao vivo na Home e exportar/importar configurações', es: 'Widgets en Inicio e importar/exportar ajustes' },
        detail: {
          en: 'Added live memory, disk, next reminder and latest note cards to Home, plus one-click export and import of your NomNom settings.',
          pt: 'Adicionados cartões de memória, disco, próximo lembrete e última nota na Home, além de exportação e importação das configurações.',
          es: 'Añadidos widgets de memoria, disco, próximo recordatorio y última nota en Inicio, además de exportar/importar ajustes.',
        },
        toolId: 'home',
        toolLabel: 'Home',
      },
    ],
  },
  {
    version: '0.5.1',
    date: '2026-11-19',
    codename: {
      en: 'Compact Window Scroll Fixes',
      pt: 'Correções de Rolagem em Janelas Menores',
      es: 'Ajustes de Desplazamiento en Ventanas Pequeñas',
    },
    summary: {
      en: 'Refines scrolling behavior in the sidebar and Playtime history so shorter laptop displays remain comfortable to navigate.',
      pt: 'Ajusta a rolagem independente na barra lateral e no histórico do Tempo de Jogo em telas de menor altura.',
      es: 'Mejora el desplazamiento independiente en la barra lateral y el historial de Tiempo de Juego en pantallas más bajas.',
    },
    highlights: [
      { en: 'Scrollable sidebar on short screens', pt: 'Barra lateral com rolagem em telas baixas', es: 'Barra lateral desplazable en pantallas bajas' },
      { en: 'Balanced Playtime history scrolling', pt: 'Histórico do Playtime alinhado com a lista', es: 'Historial de Playtime equilibrado' },
    ],
    changes: [
      {
        kind: 'fixed',
        title: { en: 'Sidebar & Playtime independent scrolling', pt: 'Rolagem independente na barra lateral e Playtime', es: 'Desplazamiento independiente en barra lateral y Playtime' },
        detail: {
          en: 'Made the sidebar navigation list scroll inside short windows without hiding footer items, and matched the Playtime history panel height to the apps list.',
          pt: 'A lista principal da barra lateral agora rola internamente sem empurrar o rodapé para fora, e o histórico do Playtime acompanha a altura da lista de apps.',
          es: 'La lista lateral ahora se desplaza internamente en ventanas bajas y el historial de Playtime iguala la altura de la lista de apps.',
        },
        toolId: 'activity',
        toolLabel: 'Playtime',
      },
    ],
  },
  {
    version: '0.5.0',
    date: '2026-11-18',
    codename: {
      en: 'Clipboard, Translator, Screen OCR, Phone Drop & Subscriptions',
      pt: 'Área de Transferência, Tradutor, OCR, Envio pelo Celular e Assinaturas',
      es: 'Portapapeles, Traductor, OCR, Transferencia Móvil y Suscripciones',
    },
    summary: {
      en: 'Adds nine daily workflow tools: local Clipboard history, DeepL Translator, offline Screen OCR, Wi-Fi Phone transfer, Downloader channel subscriptions, Startup manager, Batch rename, Playtime goals and themed corner notifications.',
      pt: 'Adiciona nove ferramentas de uso diário: histórico da Área de Transferência, Tradutor DeepL, OCR offline de tela, transferência Wi-Fi com celular, assinaturas de canais, Apps de inicialização, renomeação em lote, metas do Playtime e notificações em cards.',
      es: 'Añade nueve herramientas diarias: historial del Portapapeles, Traductor DeepL, OCR offline, transferencia móvil por Wi-Fi, suscripciones del descargador, apps de inicio, renombrado en lote, metas de Playtime y notificaciones temáticas.',
    },
    highlights: [
      { en: 'Local Clipboard history & shortcut popup', pt: 'Histórico local do Clipboard com popup rápido', es: 'Historial local del Portapapeles' },
      { en: 'DeepL Translator & offline Screen OCR', pt: 'Tradutor DeepL e OCR offline de tela', es: 'Traductor DeepL y OCR offline' },
      { en: 'QR Phone transfer & Channel subscriptions', pt: 'Envio por QR com celular e canais seguidos', es: 'Transferencia QR móvil y suscripciones' },
      { en: 'Startup apps, Batch rename & Playtime goals', pt: 'Apps de inicialização, renomear em lote e metas', es: 'Apps de inicio, renombrado y metas' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Clipboard history & shortcut popup', pt: 'Histórico da Área de Transferência', es: 'Historial del Portapapeles' },
        detail: {
          en: 'Search everything you copied, pin frequent snippets, open a floating history popup via shortcut and configure automatic cleanup by days.',
          pt: 'Busque tudo o que copiou, fixe itens frequentes, abra o histórico flutuante por atalho e escolha quantos dias manter.',
          es: 'Busca todo lo copiado, fija elementos frecuentes, abre el historial rápido con un atajo y define días de retención.',
        },
        toolId: 'clipboard',
        toolLabel: 'Clipboard',
      },
      {
        kind: 'added',
        title: { en: 'DeepL Translator & offline Screen OCR', pt: 'Tradutor DeepL e OCR offline de tela', es: 'Traductor DeepL y OCR offline de pantalla' },
        detail: {
          en: 'Translate selected text in place or beside the pointer with DeepL, and extract text from any region on screen offline with optional instant translation.',
          pt: 'Traduza texto selecionado no lugar ou ao lado do cursor com DeepL, e leia texto de qualquer parte da tela offline com tradução opcional.',
          es: 'Traduce texto seleccionado al instante con DeepL y captura texto de cualquier parte de la pantalla sin conexión.',
        },
        toolId: 'text',
        toolLabel: 'Translator',
      },
      {
        kind: 'added',
        title: { en: 'Two-way Wi-Fi Phone transfer', pt: 'Transferência Wi-Fi com celular por QR Code', es: 'Transferencia Wi-Fi móvil con código QR' },
        detail: {
          en: 'Send files in both directions between your phone browser and PC over your local network — no cables, cloud accounts or phone app needed.',
          pt: 'Envie arquivos nos dois sentidos entre o navegador do celular e o PC pela rede local, sem cabos, nuvem ou app extra.',
          es: 'Envía archivos en ambos sentidos entre el móvil y el PC por Wi-Fi local, sin nube ni apps extra.',
        },
        toolId: 'phone',
        toolLabel: 'Phone transfer',
      },
      {
        kind: 'added',
        title: { en: 'Downloader subscriptions & Startup apps', pt: 'Assinaturas no Downloader e Apps de inicialização', es: 'Suscripciones en Downloader y Apps de inicio' },
        detail: {
          en: 'Follow channels and playlists to grab new videos automatically, and inspect Windows startup apps with real measured boot delays.',
          pt: 'Siga canais e playlists para baixar novos vídeos automaticamente e gerencie apps de inicialização vendo o impacto real no boot do Windows.',
          es: 'Sigue canales y listas para descargar nuevos vídeos automáticamente y gestiona las apps de inicio con su impacto real.',
        },
        toolId: 'startup',
        toolLabel: 'Startup apps',
      },
      {
        kind: 'added',
        title: { en: 'Batch rename, Playtime goals & themed toasts', pt: 'Renomear em lote, metas do Playtime e toasts com tema', es: 'Renombrado en lote, metas de Playtime y avisos con tema' },
        detail: {
          en: 'Added pattern-based bulk file renaming with preview and undo, daily/weekly Playtime limits/targets with weekly reports, and themed corner toast cards.',
          pt: 'Adicionados renomeação em lote com prévia e desfazer, metas e limites no Playtime com relatório semanal e notificações estilizadas no canto da tela.',
          es: 'Añadidos renombrado masivo con vista previa y deshacer, metas/límites de Playtime con informe semanal y tarjetas de notificación.',
        },
        toolId: 'filetools',
        toolLabel: 'File Tools',
      },
    ],
  },
  {
    version: '0.4.5',
    date: '2026-11-18',
    codename: {
      en: 'Linux Graphics Path & Quick Popup Stability',
      pt: 'Desempenho Gráfico e Popup Rápido no Linux',
      es: 'Rendimiento Gráfico y Ventana Rápida en Linux',
    },
    summary: {
      en: 'Covers releases 0.4.4 and 0.4.5: improves Linux WebKit rendering defaults, adds a Graphics renderer setting, lowers background CPU scans and makes the quick-download popup render cleanly.',
      pt: 'Reúne as versões 0.4.4 e 0.4.5: aprimora o renderizador gráfico no Linux, adiciona opção de Gráficos nas configurações, reduz varreduras em segundo plano e corrige o redesenho do popup rápido.',
      es: 'Reúne las versiones 0.4.4 y 0.4.5: optimiza la ruta gráfica en Linux, añade ajuste de gráficos, reduce escaneos en segundo plano y corrige el popup rápido.',
    },
    highlights: [
      { en: 'Solid quick-download popup on Linux', pt: 'Popup rápido sólido e sem artefatos no Linux', es: 'Popup rápido sólido en Linux' },
      { en: 'Graphics renderer selector & lower background CPU', pt: 'Seletor de renderizador gráfico e menos uso de CPU', es: 'Selector gráfico y menor uso de CPU' },
    ],
    changes: [
      {
        kind: 'improved',
        title: { en: 'Fast Linux graphics defaults & lower idle CPU', pt: 'Rota gráfica rápida no Linux e menos varreduras', es: 'Ruta gráfica rápida en Linux y menos consumo' },
        detail: {
          en: 'Keeps hardware acceleration active on Linux unless a fallback is needed, adds an explicit Graphics setting and reduces background process scanning frequency.',
          pt: 'Mantém a aceleração gráfica ativa no Linux quando compatível, inclui ajuste de Gráficos nas configurações e reduz a frequência de varredura de processos.',
          es: 'Mantiene la aceleración gráfica en Linux cuando es compatible, añade el ajuste de gráficos y reduce la frecuencia de escaneo.',
        },
        toolId: 'settings',
        toolLabel: 'Settings',
      },
      {
        kind: 'fixed',
        title: { en: 'Linux quick download popup redraw', pt: 'Redesenho do popup rápido de download no Linux', es: 'Redibujado del popup de descarga rápida en Linux' },
        detail: {
          en: 'Sized the Linux quick-download popup cleanly to its card so it no longer redraws in visible pieces.',
          pt: 'Ajustado o tamanho sólido da janela do popup rápido no Linux para abrir de uma vez sem redesenhar em partes.',
          es: 'Ajustada la ventana del popup rápido en Linux al tamaño exacto de su tarjeta.',
        },
        toolId: 'downloads',
        toolLabel: 'NomNom Downloader',
      },
    ],
  },
  {
    version: '0.4.3',
    date: '2026-11-18',
    codename: {
      en: 'Custom Sidebar Tools, Title Bar & Extension Upgrades',
      pt: 'Personalização da Barra Lateral, Barra de Título e Extensão',
      es: 'Personalización de Barra Lateral, Barra de Título y Extensión',
    },
    summary: {
      en: 'Covers releases 0.4.1 through 0.4.3: introduces the Tools picker page, custom Back/Forward title bar, Wayland/KDE/GNOME Playtime tracking, Chrome/Edge extension build and automatic DB recovery.',
      pt: 'Reúne as versões 0.4.1 a 0.4.3: cria a página Tools para escolher o que aparece na barra lateral, barra de título própria com voltar/avançar, rastreamento no KDE/GNOME Wayland, extensão para Chrome/Edge e recuperação automática do banco de dados.',
      es: 'Reúne las versiones 0.4.1 a 0.4.3: estrena la página Tools, barra de título propia con historial, seguimiento en Wayland, extensión para Chrome/Edge y recuperación de base de datos.',
    },
    highlights: [
      { en: 'Tools page & first-time sidebar picker', pt: 'Página Tools para personalizar a barra lateral', es: 'Página Tools para personalizar el menú' },
      { en: 'Custom title bar with Back/Forward history', pt: 'Barra de título própria com Voltar e Avançar', es: 'Barra de título con historial Atrás/Adelante' },
      { en: 'Chrome, Edge & Firefox extension builds', pt: 'Extensão para Chrome, Edge e Firefox', es: 'Extensión para Chrome, Edge y Firefox' },
      { en: 'Idle memory release & automatic DB recovery', pt: 'Liberação de memória ociosa e recuperação de banco', es: 'Liberación de memoria y recuperación de BD' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Tools catalog & custom title bar', pt: 'Catálogo Tools e barra de título própria', es: 'Catálogo Tools y barra de título personalizada' },
        detail: {
          en: 'Pick which tools appear in the sidebar, navigate visited pages with Back/Forward buttons and Alt+Arrows, and record shortcuts using the Super key.',
          pt: 'Escolha quais ferramentas aparecem na barra lateral, navegue entre páginas com Voltar/Avançar e Alt+Setas, e grave atalhos com a tecla Super.',
          es: 'Elige qué herramientas mostrar en el menú, navega con Atrás/Adelante y graba atajos con la tecla Super.',
        },
        toolId: 'tools',
        toolLabel: 'Tools',
      },
      {
        kind: 'improved',
        title: { en: 'Browser extension, Wayland Playtime & lower idle RAM', pt: 'Extensão, Playtime no Wayland e menos uso de RAM', es: 'Extensión, Playtime en Wayland y menor uso de RAM' },
        detail: {
          en: 'Added Chrome/Edge extension packaging and broader video player detection, KDE Plasma Wayland window tracking, idle memory trimming and unfocused animation pausing.',
          pt: 'Empacotamento para Chrome/Edge com detecção em mais sites de vídeo, leitura de janelas no KDE Plasma Wayland, liberação de memória ociosa e pausa de animações fora de foco.',
          es: 'Paquete para Chrome/Edge con mayor detección de vídeo, soporte de ventanas en KDE Plasma Wayland, liberación de memoria y pausa de animaciones.',
        },
      },
      {
        kind: 'fixed',
        title: { en: 'Automatic damaged-database recovery', pt: 'Recuperação automática de banco de dados corrompido', es: 'Recuperación automática de base de datos dañada' },
        detail: {
          en: 'If the local SQLite database is ever damaged, NomNom moves the original aside and rebuilds a healthy database on startup.',
          pt: 'Se o banco SQLite local for danificado, o NomNom preserva a cópia original ao lado e recria um banco saudável na inicialização.',
          es: 'Si la base SQLite se daña, NomNom guarda la original aparte y reconstruye una base limpia al iniciar.',
        },
      },
    ],
  },
  {
    version: '0.4.0',
    date: '2026-11-18',
    codename: {
      en: 'System Monitor, Backups, Focus Mode, PDF Tools & i18n',
      pt: 'Monitor do Sistema, Backups, Modo Foco, PDFs e Tradução PT/ES',
      es: 'Monitor del Sistema, Copias, Modo Enfoque, PDF y Multilingüe',
    },
    summary: {
      en: 'Adds live System Monitor, versioned Backups, Focus mode, PDF merge/split, Quick cleanup, Downloader scheduling/subtitles, 10 new animated themes and full Portuguese & Spanish translations.',
      pt: 'Adiciona Monitor do Sistema ao vivo, Backup com versões, Modo Foco, ferramentas de PDF, Limpeza rápida, legendas e agendamento no Downloader, 10 novos temas animados e tradução completa em português e espanhol.',
      es: 'Añade Monitor del Sistema, Copias con versiones, Modo Enfoque, herramientas PDF, Limpieza rápida, subtítulos y programación en Downloader, 10 temas animados e interfaz en español y portugués.',
    },
    highlights: [
      { en: 'Live System Monitor & tray tooltip', pt: 'Monitor do Sistema ao vivo e resumo na bandeja', es: 'Monitor del Sistema en vivo' },
      { en: 'Scheduled versioned Backups with verify', pt: 'Backups agendados com versões e verificação', es: 'Copias programadas con versiones' },
      { en: 'Focus mode, PDF tools & Quick cleanup', pt: 'Modo foco, ferramentas de PDF e limpeza rápida', es: 'Modo enfoque, PDF y limpieza rápida' },
      { en: 'Full Portuguese & Spanish UI + 10 themes', pt: 'Interface completa em PT/ES + 10 novos temas', es: 'Interfaz en ES/PT + 10 temas nuevos' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Live System Monitor & Scheduled Backups', pt: 'Monitor do Sistema ao vivo e Backups agendados', es: 'Monitor del Sistema y Copias programadas' },
        detail: {
          en: 'Track real-time CPU, memory, disk, network and busiest apps, and run scheduled folder backups with version history, restore and integrity checks.',
          pt: 'Acompanhe CPU, memória, discos, rede e apps mais pesados em tempo real, e agende backups de pastas com histórico de versões, restauração e checagem de integridade.',
          es: 'Supervisa CPU, memoria, discos, red y procesos activos, y programa copias de carpetas con versiones y verificación.',
        },
        toolId: 'monitor',
        toolLabel: 'System Monitor',
      },
      {
        kind: 'added',
        title: { en: 'Focus mode, PDF tools & Quick cleanup', pt: 'Modo Foco, ferramentas de PDF e Limpeza rápida', es: 'Modo Enfoque, herramientas PDF y Limpieza rápida' },
        detail: {
          en: 'Timed work/break blocks with streaks and app breakdowns, PDF joining and page extraction, EXIF-clean photo presets, and preview-first cache/temp cleanup.',
          pt: 'Blocos cronometrados de foco e pausa com sequência de dias, junção e extração de páginas PDF, presets de fotos sem metadados EXIF e limpeza rápida de caches com revisão prévia.',
          es: 'Bloques de trabajo/descanso con rachas, unión y extracción de PDF, fotos sin metadatos EXIF y limpieza rápida de cachés con revisión.',
        },
        toolId: 'filetools',
        toolLabel: 'File Tools',
      },
      {
        kind: 'added',
        title: { en: 'Portuguese & Spanish translations + 10 animated themes', pt: 'Interface em Português e Espanhol + 10 temas animados', es: 'Interfaz en Español y Portugués + 10 temas animados' },
        detail: {
          en: 'Translated the whole interface, tray menu and notifications to Portuguese and Spanish, added embedded subtitles & scheduled downloads, and expanded the theme collection by 10 themes.',
          pt: 'Tradução completa da interface, bandeja e notificações para português e espanhol, legendas e agendamento no Downloader, e 10 novos temas com fundos animados.',
          es: 'Traducción completa de la interfaz, bandeja y notificaciones al español y portugués, subtítulos y descargas programadas, y 10 temas nuevos.',
        },
        toolId: 'settings',
        toolLabel: 'Settings',
      },
    ],
  },
  {
    version: '0.3.0',
    date: '2026-11-18',
    codename: {
      en: 'Animated Themes, Lucide Icons & Settings Redesign',
      pt: 'Temas Animados, Ícones Lucide e Novas Configurações',
      es: 'Temas Animados, Iconos Lucide y Nuevo Diseño de Ajustes',
    },
    summary: {
      en: 'Refreshes the entire visual identity with 5 animated themes, Lucide icons, a new NomNom app icon and a searchable two-column Settings view.',
      pt: 'Renova toda a identidade visual com 5 temas animados, biblioteca de ícones Lucide, novo ícone do app e tela de Configurações pesquisável em duas colunas.',
      es: 'Renueva la identidad visual con 5 temas animados, iconos Lucide, nuevo icono de app y ajustes con búsqueda en dos columnas.',
    },
    highlights: [
      { en: '5 animated themes & new app icon', pt: '5 temas animados e novo ícone do app', es: '5 temas animados y nuevo icono' },
      { en: 'Lucide icon set & searchable Settings', pt: 'Ícones Lucide e Configurações com busca', es: 'Iconos Lucide y Ajustes con búsqueda' },
    ],
    changes: [
      {
        kind: 'design',
        title: { en: 'Lucide icons, animated themes & reworked Settings', pt: 'Ícones Lucide, temas animados e Configurações renovadas', es: 'Iconos Lucide, temas animados y Ajustes renovados' },
        detail: {
          en: 'Replaced custom glyphs with unified Lucide icons, shipped 5 animated themes and redesigned Settings with instant keyword search and categorized navigation.',
          pt: 'Ícones unificados com Lucide, 5 temas com fundos animados e Configurações reorganizadas com barra de busca instantânea por seção.',
          es: 'Iconos unificados con Lucide, 5 temas animados y Ajustes rediseñados con búsqueda instantánea.',
        },
        toolId: 'settings',
        toolLabel: 'Settings',
      },
    ],
  },
  {
    version: '0.2.0',
    date: '2026-11-18',
    codename: {
      en: 'Disk Analyzer, Playtime, Linux Port & In-App Updates',
      pt: 'Analisador de Disco, Playtime, Suporte ao Linux e Auto-Update',
      es: 'Analizador de Disco, Playtime, Soporte Linux y Autoactualización',
    },
    summary: {
      en: 'Covers 0.2.0 and 0.2.1: launches the full Disk Analyzer, Playtime active app/game tracker, in-app self-updates and full Linux desktop packaging.',
      pt: 'Reúne 0.2.0 e 0.2.1: lança o Analisador de Disco completo, rastreador Playtime de apps e jogos, atualizações automáticas dentro do app e porte completo para Linux.',
      es: 'Reúne 0.2.0 y 0.2.1: lanza el Analizador de Disco completo, Playtime para apps y juegos, actualizaciones integradas y soporte completo para Linux.',
    },
    highlights: [
      { en: 'Disk Analyzer (Treemap, Sunburst, Duplicates, Snapshots)', pt: 'Analisador de Disco (Treemap, Sunburst, Duplicados, Snapshots)', es: 'Analizador de Disco (Treemap, Sunburst, Duplicados)' },
      { en: 'Playtime active app & game hours', pt: 'Playtime para horas ativas de apps e jogos', es: 'Playtime de horas activas en apps y juegos' },
      { en: 'Full Linux port (.deb, AppImage, Arch) & in-app updates', pt: 'Suporte completo ao Linux e atualizações no app', es: 'Soporte completo Linux y actualizaciones integradas' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'Disk Analyzer (stages 1–5)', pt: 'Analisador de Disco completo', es: 'Analizador de Disco completo' },
        detail: {
          en: 'Parallel disk scanner with TreeGrid, cushioned Treemap, Sunburst chart, duplicate file finder, search, batch actions, snapshots, comparison, scheduled scans, NTFS details and Explorer menu integration.',
          pt: 'Varredura paralela de disco com TreeGrid, Treemap, gráfico Sunburst, detector de arquivos duplicados, busca, snapshots comparáveis, varreduras agendadas, detalhes NTFS e ação no menu do Explorador.',
          es: 'Escáner paralelo de disco con TreeGrid, Treemap, gráfico Sunburst, detector de duplicados, instantáneas comparables, escaneos programados e integración con el Explorador.',
        },
        toolId: 'storage',
        toolLabel: 'Disk Analyzer',
      },
      {
        kind: 'added',
        title: { en: 'Playtime tracker & silent in-app updates', pt: 'Rastreador Playtime e atualizações no próprio app', es: 'Rastreador Playtime y actualizaciones integradas' },
        detail: {
          en: 'Automatically tracks active foreground hours across apps and games in SQLite, paired with GitHub release checks and one-click in-place self-updates.',
          pt: 'Registra automaticamente horas ativas de apps e jogos no banco local e verifica/instala novas versões do GitHub sem sair do app.',
          es: 'Registra automáticamente las horas activas de apps y juegos y permite actualizar NomNom desde GitHub en un clic.',
        },
        toolId: 'activity',
        toolLabel: 'Playtime',
      },
      {
        kind: 'added',
        title: { en: 'Full Linux port & packaging', pt: 'Porte completo e pacotes para Linux', es: 'Soporte completo y paquetes para Linux' },
        detail: {
          en: 'Added XDG paths, Unix socket IPC, native messaging host, Linux Trash/pkexec disk support, --quick Wayland flag and .deb, AppImage and Arch PKGBUILD builds.',
          pt: 'Suporte a caminhos XDG, IPC via Unix socket, lixeira/pkexec no Linux, flag --quick para atalhos no Wayland e pacotes .deb, AppImage e Arch PKGBUILD.',
          es: 'Rutas XDG, IPC por socket Unix, soporte de Papelera/pkexec, flag --quick para Wayland y paquetes .deb, AppImage y Arch.',
        },
      },
    ],
  },
  {
    version: '0.1.0',
    date: '2026-11-06',
    codename: {
      en: 'First Release — Downloader, File Organizer & File Tools',
      pt: 'Primeira Versão — Downloader, Organizador e Conversor Local',
      es: 'Primera Versión — Descargador, Organizador y Conversor Local',
    },
    summary: {
      en: 'The foundation of NomNom: a private local desktop hub built in Rust, Tauri and Svelte with media downloads, preview-first folder sorting, local file conversions and browser integration.',
      pt: 'A base do NomNom: uma central local e privada em Rust, Tauri e Svelte com downloads de mídia, organização segura de pastas, conversão local de arquivos e extensão de navegador.',
      es: 'La base de NomNom: un centro privado local en Rust, Tauri y Svelte con descargas de medios, organización segura de carpetas, conversión local y extensión de navegador.',
    },
    highlights: [
      { en: 'NomNom Downloader + Ctrl+Alt+D quick popup', pt: 'NomNom Downloader + popup rápido Ctrl+Alt+D', es: 'NomNom Downloader + popup rápido Ctrl+Alt+D' },
      { en: 'Cautious File Organizer with full undo', pt: 'Organizador de Arquivos com prévia e desfazer', es: 'Organizador de Archivos con vista previa y deshacer' },
      { en: 'Local File Tools converter & Firefox extension', pt: 'Conversor local File Tools e extensão Firefox', es: 'Conversor local File Tools y extensión Firefox' },
    ],
    changes: [
      {
        kind: 'added',
        title: { en: 'NomNom Downloader & Firefox extension', pt: 'NomNom Downloader e extensão para Firefox', es: 'NomNom Downloader y extensión para Firefox' },
        detail: {
          en: 'Download videos and audio with queue, history, format presets, managed yt-dlp/FFmpeg binaries, global Ctrl+Alt+D quick popup and native-messaging browser extension.',
          pt: 'Baixe vídeos e áudios com fila, histórico, presets de formato, gerenciamento automático de yt-dlp/FFmpeg, popup rápido Ctrl+Alt+D e extensão de navegador.',
          es: 'Descarga vídeos y audio con cola, historial, gestión de yt-dlp/FFmpeg, popup Ctrl+Alt+D y extensión para navegador.',
        },
        toolId: 'downloads',
        toolLabel: 'NomNom Downloader',
      },
      {
        kind: 'added',
        title: { en: 'Cautious File Organizer', pt: 'Organizador de Arquivos seguro', es: 'Organizador de Archivos seguro' },
        detail: {
          en: 'Preview every move before touching disk, skip program/code directories automatically, never overwrite files, record a SQLite journal and undo in one click.',
          pt: 'Visualize cada movimentação antes de aplicar, ignore pastas de programas/código, nunca sobrescreva arquivos e desfaça tudo em um clique.',
          es: 'Previsualiza cada movimiento antes de aplicarlo, omite carpetas de código/programas y deshaz todo en un clic.',
        },
        toolId: 'organizer',
        toolLabel: 'File Organizer',
      },
      {
        kind: 'added',
        title: { en: 'Local File Tools & Windows right-click menu', pt: 'Conversor local File Tools e menu de clique direito no Windows', es: 'Conversor local File Tools y menú contextual en Windows' },
        detail: {
          en: 'Convert images, audio, video, PDF and Office documents locally on your computer, accessible directly from the app or the Windows Explorer context menu.',
          pt: 'Converta imagens, áudio, vídeo, PDFs e documentos Office localmente, tanto pelo app quanto pelo menu de clique direito do Explorador do Windows.',
          es: 'Convierte imágenes, audio, vídeo, PDF y documentos Office localmente desde la app o el menú contextual de Windows.',
        },
        toolId: 'filetools',
        toolLabel: 'File Tools',
      },
    ],
  },
];

export const CHANGE_KINDS: ChangeKind[] = ['added', 'improved', 'design', 'fixed'];

export interface ChangelogStats {
  versions: number;
  totalChanges: number;
  added: number;
  improved: number;
  design: number;
  fixed: number;
}

/** Summarizes total versions and change counts across the changelog. */
export function changelogStats(list: ReleaseNote[] = RELEASES): ChangelogStats {
  const stats: ChangelogStats = {
    versions: list.length,
    totalChanges: 0,
    added: 0,
    improved: 0,
    design: 0,
    fixed: 0,
  };
  for (const r of list) {
    for (const c of r.changes) {
      stats.totalChanges += 1;
      stats[c.kind] += 1;
    }
  }
  return stats;
}

/** Filters releases by change category and free-text query across all languages. */
export function filterReleases(
  query: string,
  kind: ChangeKind | 'all',
  list: ReleaseNote[] = RELEASES,
): ReleaseNote[] {
  const q = query.trim().toLowerCase();
  const verQ = q.startsWith('v') ? q.slice(1) : q;
  const out: ReleaseNote[] = [];
  for (const r of list) {
    const versionMatches =
      q.length > 0 &&
      ((verQ.length > 0 && r.version.toLowerCase().includes(verQ)) ||
        `v${r.version}`.toLowerCase().includes(q) ||
        r.date.includes(q) ||
        r.codename.en.toLowerCase().includes(q) ||
        r.codename.pt.toLowerCase().includes(q) ||
        r.codename.es.toLowerCase().includes(q) ||
        r.summary.en.toLowerCase().includes(q) ||
        r.summary.pt.toLowerCase().includes(q) ||
        r.summary.es.toLowerCase().includes(q));

    const changes = r.changes.filter((c) => {
      if (kind !== 'all' && c.kind !== kind) return false;
      if (!q || versionMatches) return true;
      const hay = [
        c.title.en,
        c.title.pt,
        c.title.es,
        c.detail.en,
        c.detail.pt,
        c.detail.es,
        c.toolId ?? '',
        c.toolLabel ?? '',
      ]
        .join(' ')
        .toLowerCase();
      return hay.includes(q);
    });

    if (changes.length > 0) {
      out.push({ ...r, changes });
    }
  }
  return out;
}
