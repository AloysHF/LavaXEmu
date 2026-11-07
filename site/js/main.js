// ============================================================
// LavaXEmu — Landing Page Scripts
// i18n + Carousel Gallery + Animations
// ============================================================

(function () {
  'use strict';

  // ================================================================
  // GAME DATA — selection of validated LavaX programs
  // Images live in docs/images/, matching docs/Game-Compatibility.md.
  // ================================================================
  var GAMES = [
    // Action
    { cat: 'act', zh: '空间战争', en: 'SpaceWar', img: 'docs/images/SpaceWar.png', descZh: '太空背景的射击对战游戏', descEn: 'Space-themed battle shooter' },
    { cat: 'act', zh: '文曲星版Zelda', en: 'Zelda (Wenquxing)', img: 'docs/images/文曲星版Zelda.png', descZh: '像素风格的塞尔达式冒险游戏', descEn: 'Zelda-style pixel adventure' },
    { cat: 'act', zh: 'SnowFight', en: 'SnowFight', img: 'docs/images/SnowFight.png', descZh: '雪仗主题的对抗小游戏', descEn: 'Snowball fight mini-game' },
    { cat: 'act', zh: '灵剑射击', en: 'Spirit Sword', img: 'docs/images/灵剑射击.png', descZh: '手绘风格的飞行射击游戏', descEn: 'Hand-drawn style shooting game' },
    { cat: 'act', zh: '决战娜美克', en: 'Namek Showdown', img: 'docs/images/决战娜美克.png', descZh: '龙珠题材的格斗对战游戏', descEn: 'Dragon Ball themed fighting game' },
    { cat: 'act', zh: 'Fighter', en: 'Fighter', img: 'docs/images/Fighter.png', descZh: '致敬 FC 平台的射击游戏', descEn: 'A shooter tribute to classic hardware' },

    // Puzzle
    { cat: 'puz', zh: '魔塔', en: 'Magic Tower', img: 'docs/images/魔塔.png', descZh: '经典魔塔爬塔解谜游戏', descEn: 'Classic Magic Tower puzzle RPG' },
    { cat: 'puz', zh: 'sudoku', en: 'Sudoku', img: 'docs/images/sudoku.png', descZh: '标准数独益智游戏', descEn: 'Standard Sudoku puzzle' },
    { cat: 'puz', zh: '俄罗斯方块黑', en: 'Tetris (Dark)', img: 'docs/images/俄罗斯方块黑.png', descZh: '黑底配色的俄罗斯方块', descEn: 'Tetris on a dark background' },
    { cat: 'puz', zh: '白金贪吃蛇', en: 'Snake Platinum', img: 'docs/images/白金贪吃蛇.png', descZh: '支持速度设置与英雄榜的贪吃蛇', descEn: 'Snake with speed settings and hiscores' },
    { cat: 'puz', zh: 'IQ大作战', en: 'IQ Battle', img: 'docs/images/IQ大作战.png', descZh: '脑筋急转弯式益智问答', descEn: 'Brain-teaser quiz game' },
    { cat: 'puz', zh: 'Rush Out (16K)', en: 'Rush Out (16K)', img: 'docs/images/Rush_Out_For_Lava1(16k).png', descZh: '16K 版钻隧道闯关游戏', descEn: '16K tunnel rush puzzle game' },

    // Board & Cards
    { cat: 'board', zh: '象棋90', en: 'Chinese Chess', img: 'docs/images/象棋90.png', descZh: '传统中国象棋棋盘游戏', descEn: 'Traditional Chinese chess board' },
    { cat: 'board', zh: '中国麻将', en: 'MahJong', img: 'docs/images/中国麻将.png', descZh: '麻将牌桌小游戏', descEn: 'Mahjong table mini-game' },
    { cat: 'board', zh: '扑克大战', en: 'Poker Battle', img: 'docs/images/扑克大战.png', descZh: '经典扑克纸牌游戏', descEn: 'Classic poker card game' },
    { cat: 'board', zh: '空当接龙', en: 'FreeCell', img: 'docs/images/空当接龙.png', descZh: '经典空当接龙纸牌游戏', descEn: 'Classic FreeCell solitaire' },
    { cat: 'board', zh: 'Fivet', en: 'Fivet', img: 'docs/images/Fivet.png', descZh: '卡通画风的连珠类游戏', descEn: 'Gomoku-style game with cartoon art' },

    // Sports
    { cat: 'sport', zh: 'boxing', en: 'Boxing', img: 'docs/images/boxing.png', descZh: '拳击对抗小游戏', descEn: 'Boxing match mini-game' },
    { cat: 'sport', zh: '撞球高手', en: 'Cue Master', img: 'docs/images/撞球高手.png', descZh: '台球瞄准与走位练习', descEn: 'Billiards aiming and positioning' },
    { cat: 'sport', zh: '凌空抽射', en: 'Volley Shot', img: 'docs/images/凌空抽射v1.5.png', descZh: '足球题材的体育竞技游戏', descEn: 'Football-themed sports game' },

    // Tools
    { cat: 'tool', zh: '数电模拟器', en: 'Circuit Lab', img: 'docs/images/数电模拟器.png', descZh: '数字电路实验模拟工具', descEn: 'Digital circuit lab simulator' },
    { cat: 'tool', zh: '资源管理器', en: 'File Explorer', img: 'docs/images/资源管理器.png', descZh: '机器内置的文件管理工具', descEn: 'Built-in file manager utility' },
    { cat: 'tool', zh: '9stars', en: 'Nine Stars', img: 'docs/images/9stars.png', descZh: '奇门九星诀排盘工具', descEn: 'Qimen star divination tool' }
  ];

  var CATEGORIES = [
    { id: 'all', zh: '全部', en: 'All' },
    { id: 'act', zh: '动作', en: 'Action' },
    { id: 'puz', zh: '益智', en: 'Puzzle' },
    { id: 'board', zh: '棋牌', en: 'Board' },
    { id: 'sport', zh: '运动', en: 'Sports' },
    { id: 'tool', zh: '工具', en: 'Tools' }
  ];

  // ================================================================
  // i18n — Translations
  // ================================================================
  var translations = {
    zh: {
      // meta
      'meta-title': 'LavaXEmu — 让字节码游戏重获新生',
      'meta-desc': '用 Rust 编写的 LavaX 虚拟机与模拟器，支持 160×240 索引色屏上的经典 .lav 程序，兼容 RetroArch',
      // nav
      'nav-features': '核心特性',
      'nav-games': '游戏库',
      'nav-arch': '技术架构',
      'nav-quickstart': '快速开始',
      'nav-play': '在线体验',
      // hero
      'hero-subtitle': '让字节码游戏重获新生',
      'hero-desc': '用 Rust 编写的 LavaX 虚拟机与模拟器，完整运行电子词典上的经典 .lav 程序',
      'hero-download': '下载',
      'hero-github': '查看源码',
      'hero-play': '在线试玩',
      'hero-scroll': '向下滚动探索',
      // play online
      'play-badge': '无需安装 · 浏览器直接运行',
      'play-title': '在线试玩 LavaX 游戏',
      'play-desc': '选择本机的 .lav 程序或打包好的 ZIP，即可在浏览器中运行，支持键盘、手柄和手机触屏。',
      'play-cta': '开始试玩',
      // about
      'about-title': '什么是 LavaX？',
      'about-p1': 'LavaX 是早期电子词典与学习机上流行的 <strong>LAVA 字节码应用平台</strong>。游戏以 .lav 程序分发，运行在设备内置的栈式虚拟机上，使用索引色屏幕与按键/触摸交互。',
      'about-p2': '这些曾经只能在专用硬件上运行的游戏，如今通过 LavaXEmu 在现代设备上重获新生。',
      'about-chip': '电子词典设备',
      'about-chip-sub': '内置 LAVA 运行时',
      'about-game': '.lav 程序',
      'about-game-sub': 'LavaX 游戏',
      'about-emu-sub': 'Rust 模拟器',
      // stats
      'stat-games': '通过启动验证',
      'stat-games-sub': '含完整兼容性截图',
      'stat-opcodes': 'VM 指令',
      'stat-opcodes-sub': '栈式字节码虚拟机',
      'stat-apis': '系统 API',
      'stat-apis-sub': '固件服务桥接',
      'stat-colors': '同屏颜色',
      'stat-colors-sub': '160×240 索引色',
      // features
      'feat-title': '核心特性',
      'feat-subtitle': '从字节码虚拟机到虚拟文件系统，全栈 Rust 实现',
      'feat-vm-title': 'LAV 字节码虚拟机',
      'feat-vm-desc': '117 个操作码的栈式虚拟机，覆盖算术、控制流、内存访问和系统调用，纯 Rust 实现。',
      'feat-api-title': '系统 API 桥',
      'feat-api-desc': '87 项固件服务分派，覆盖显示、输入、文件、内存、音频与工具函数，完整还原运行时行为。',
      'feat-color-title': '索引色显示',
      'feat-color-desc': '160×240 分辨率、256 色调色板的帧缓冲管理，还原文曲星掌上屏幕的观感。',
      'feat-vfs-title': '沙箱虚拟文件系统',
      'feat-vfs-desc': '受限路径访问与选择性写回，游戏数据文件与存档安全落盘，不会逃出沙箱。',
      'feat-headless-title': '无头确定性模式',
      'feat-headless-desc': '无窗口运行 N 帧并截图，配套确定性时钟，方便批量截图与回归测试。',
      'feat-frontends-title': '多前端支持',
      'feat-frontends-desc': 'Standalone 桌面端、RetroArch libretro 核心与 EmulatorJS 浏览器核心共享同一套核心逻辑。',
      // gallery
      'gallery-title': '游戏库',
      'gallery-subtitle': '启动验证通过的 LavaX 程序精选，完整列表见兼容性文档',
      // architecture
      'arch-title': '技术架构',
      'arch-subtitle': '清晰的三层架构，平台无关的核心引擎',
      'arch-frontends': '前端',
      'arch-standalone': 'Standalone 可执行文件',
      'arch-standalone-sub': 'minifb 窗口 + 键盘/指针',
      'arch-libretro': 'libretro cdylib',
      'arch-libretro-sub': 'RetroArch 核心 · wasm 浏览器核心',
      'arch-core': '核心引擎',
      'arch-core-sub': '平台无关的库',
      'arch-platforms': '目标平台',
      // quickstart
      'qs-title': '快速开始',
      'qs-subtitle': '几行命令，即刻体验',
      'qs-standalone': 'Standalone',
      'qs-standalone-1': '下载最新版本',
      'qs-standalone-1-sub': '从 Releases 页面下载对应平台的二进制文件',
      'qs-standalone-2': '运行游戏',
      'qs-standalone-3': '或使用无头模式截图',
      'qs-retro': 'RetroArch',
      'qs-retro-1': '下载 libretro 核心',
      'qs-retro-1-sub': '从 Releases 页面下载对应平台的核心文件',
      'qs-retro-2': '安装核心',
      'qs-retro-2-sub': '重命名为 lavaxemu_libretro 并放入 cores/ 目录',
      'qs-retro-3': '加载核心并启动',
      'qs-build': '从源码编译',
      'qs-build-1': '克隆仓库',
      'qs-build-2': '编译 Standalone',
      'qs-build-3': '或编译 RetroArch 核心',
      // footer
      'footer-desc': '用 Rust 编写的 LavaX 虚拟机与模拟器',
      'footer-project': '项目',
      'footer-contributing': '贡献指南',
      'footer-community': '社区',
      'footer-docs': '文档',
      'footer-cli': '独立模拟器',
      'footer-core': 'RetroArch Core',
      'footer-gamelist': '游戏列表',
      'footer-copy': 'GPL-2.0-or-later License &copy; 2026 Aloys. Built with 🦀 Rust.'
    },
    en: {
      // meta
      'meta-title': 'LavaXEmu — Bring bytecode games back to life',
      'meta-desc': 'A LavaX virtual machine and emulator written in Rust, running classic .lav programs on a 160×240 indexed-color display, with RetroArch support',
      // nav
      'nav-features': 'Features',
      'nav-games': 'Games',
      'nav-arch': 'Architecture',
      'nav-quickstart': 'Quick Start',
      'nav-play': 'Play Online',
      // hero
      'hero-subtitle': 'Bring bytecode games back to life',
      'hero-desc': 'A LavaX virtual machine and emulator written in Rust, running classic .lav programs from electronic dictionaries',
      'hero-download': 'Download',
      'hero-github': 'View Source',
      'hero-play': 'Play Online',
      'hero-scroll': 'Scroll to explore',
      // play online
      'play-badge': 'No install · runs in your browser',
      'play-title': 'Play LavaX Games Online',
      'play-desc': 'Pick a .lav program or a bundled ZIP from your device and play it in the browser, with keyboard, gamepad, and touch controls.',
      'play-cta': 'Start Playing',
      // about
      'about-title': 'What is LavaX?',
      'about-p1': 'LavaX is the <strong>LAVA bytecode platform</strong> popular on early electronic dictionaries and handheld learning devices. Games ship as .lav programs that run on a stack-based virtual machine built into the device, using an indexed-color display with keyboard/touch input.',
      'about-p2': 'Games that once could only run on dedicated hardware now come back to life on modern devices through LavaXEmu.',
      'about-chip': 'Dictionary Device',
      'about-chip-sub': 'Built-in LAVA runtime',
      'about-game': '.lav Program',
      'about-game-sub': 'LavaX Games',
      'about-emu-sub': 'Rust Emulator',
      // stats
      'stat-games': 'Validated Programs',
      'stat-games-sub': 'with full compatibility screenshots',
      'stat-opcodes': 'VM Opcodes',
      'stat-opcodes-sub': 'Stack-based bytecode VM',
      'stat-apis': 'System APIs',
      'stat-apis-sub': 'Firmware service bridge',
      'stat-colors': 'Display Colors',
      'stat-colors-sub': '160×240 indexed color',
      // features
      'feat-title': 'Core Features',
      'feat-subtitle': 'From bytecode VM to virtual file system — full-stack Rust implementation',
      'feat-vm-title': 'LAV Bytecode VM',
      'feat-vm-desc': 'A 117-opcode stack-based virtual machine covering arithmetic, control flow, memory access, and system calls — pure Rust.',
      'feat-api-title': 'System API Bridge',
      'feat-api-desc': '87 firmware services dispatched for display, input, files, memory, audio, and utilities, faithfully recreating the runtime.',
      'feat-color-title': 'Indexed-Color Display',
      'feat-color-desc': 'Framebuffer and palette management for the 160×240, 256-color display, recreating the handheld look.',
      'feat-vfs-title': 'Sandboxed Virtual FS',
      'feat-vfs-desc': 'Restricted path access with selective write-back — game data and saves land safely without escaping the sandbox.',
      'feat-headless-title': 'Headless Determinism',
      'feat-headless-desc': 'Run N frames without a window and capture screenshots, backed by a deterministic clock for batch regression testing.',
      'feat-frontends-title': 'Multiple Frontends',
      'feat-frontends-desc': 'The standalone desktop app, the RetroArch libretro core, and the EmulatorJS browser core all share the same engine.',
      // gallery
      'gallery-title': 'Game Gallery',
      'gallery-subtitle': 'A selection of validated LavaX programs — see the compatibility doc for the full list',
      // architecture
      'arch-title': 'Architecture',
      'arch-subtitle': 'Clean three-layer architecture with a platform-independent core engine',
      'arch-frontends': 'Frontends',
      'arch-standalone': 'lavaxemu',
      'arch-standalone-sub': 'minifb window + keyboard/pointer',
      'arch-libretro': 'lavaxemu-libretro',
      'arch-libretro-sub': 'libretro cdylib · wasm browser core',
      'arch-core': 'Core Engine',
      'arch-core-sub': 'Platform-independent library',
      'arch-platforms': 'Platforms',
      // quickstart
      'qs-title': 'Quick Start',
      'qs-subtitle': 'A few commands to get started',
      'qs-standalone': 'Standalone',
      'qs-standalone-1': 'Download latest release',
      'qs-standalone-1-sub': 'Get the binary for your platform from the Releases page',
      'qs-standalone-2': 'Run a game',
      'qs-standalone-3': 'Or capture headless screenshots',
      'qs-retro': 'RetroArch',
      'qs-retro-1': 'Download libretro core',
      'qs-retro-1-sub': 'Get the core for your platform from the Releases page',
      'qs-retro-2': 'Install the core',
      'qs-retro-2-sub': 'Rename to lavaxemu_libretro and copy to RetroArch\'s cores/ directory',
      'qs-retro-3': 'Load core and start',
      'qs-build': 'Build from Source',
      'qs-build-1': 'Clone the repository',
      'qs-build-2': 'Build Standalone',
      'qs-build-3': 'Or build RetroArch core',
      // footer
      'footer-desc': 'A LavaX virtual machine and emulator written in Rust',
      'footer-project': 'Project',
      'footer-contributing': 'Contributing',
      'footer-community': 'Community',
      'footer-docs': 'Docs',
      'footer-cli': 'Standalone Emulator',
      'footer-core': 'RetroArch Core',
      'footer-gamelist': 'Game List',
      'footer-copy': 'GPL-2.0-or-later License &copy; 2026 Aloys. Built with 🦀 Rust.'
    }
  };

  var currentLang = localStorage.getItem('lavx-lang') || (navigator.language.startsWith('zh') ? 'zh' : 'en');

  // ================================================================
  // i18n — Apply translations
  // ================================================================
  function applyLang(lang) {
    currentLang = lang;
    localStorage.setItem('lavx-lang', lang);
    document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';

    var t = translations[lang];

    // Update text content for elements with data-i18n
    document.querySelectorAll('[data-i18n]').forEach(function (el) {
      var key = el.getAttribute('data-i18n');
      if (t[key] === undefined) return;
      // Skip title/meta — handled separately below
      if (el.tagName === 'TITLE' || el.tagName === 'META') return;
      el.innerHTML = t[key];
    });

    // Update <title> and meta description
    if (t['meta-title']) document.title = t['meta-title'];
    var metaDesc = document.querySelector('meta[name="description"]');
    if (metaDesc && t['meta-desc']) metaDesc.setAttribute('content', t['meta-desc']);

    // Update language toggle button text
    var langBtn = document.getElementById('lang-toggle');
    if (langBtn) langBtn.textContent = lang === 'zh' ? 'EN' : '中';

    // Rebuild gallery with correct language
    buildGallery();
  }

  // ================================================================
  // GALLERY — Tabs + Carousel
  // ================================================================
  var currentTab = 'all';

  function getCatCount(catId) {
    if (catId === 'all') return GAMES.length;
    return GAMES.filter(function (g) { return g.cat === catId; }).length;
  }

  function buildGallery() {
    var container = document.getElementById('gallery-dynamic');
    if (!container) return;

    var lang = currentLang;
    var html = '';

    // Tab bar
    html += '<div class="gallery-tabs">';
    CATEGORIES.forEach(function (cat) {
      var count = getCatCount(cat.id);
      var label = lang === 'zh' ? cat.zh : cat.en;
      var active = cat.id === currentTab ? ' active' : '';
      html += '<button class="gallery-tab' + active + '" data-cat="' + cat.id + '">' + label + ' (' + count + ')</button>';
    });
    html += '</div>';

    // Carousel for the active tab
    CATEGORIES.forEach(function (cat) {
      if (cat.id !== currentTab) return;
      var games = cat.id === 'all' ? GAMES : GAMES.filter(function (g) { return g.cat === cat.id; });
      var catLabel = lang === 'zh' ? cat.zh : cat.en;

      html += '<div class="carousel-wrapper">';
      html += '<button class="carousel-btn carousel-prev" aria-label="Previous">&#8249;</button>';
      html += '<div class="carousel-viewport">';
      html += '<div class="carousel-track" data-cat="' + cat.id + '">';

      games.forEach(function (g) {
        var name = lang === 'zh' ? g.zh : (g.en || g.zh);
        var desc = lang === 'zh' ? g.descZh : g.descEn;
        html += '<div class="carousel-card">';
        html += '  <img src="' + g.img + '" alt="' + (g.en || g.zh) + '">';
        html += '  <div class="carousel-card-overlay">';
        html += '    <span class="gallery-tag">' + catLabel + '</span>';
        html += '    <h4>' + name + '</h4>';
        html += '    <p>' + desc + '</p>';
        html += '  </div>';
        html += '</div>';
      });

      html += '</div>';
      html += '</div>';
      html += '<button class="carousel-btn carousel-next" aria-label="Next">&#8250;</button>';

      // Dots
      var cardsPerView = window.innerWidth > 768 ? 4 : (window.innerWidth > 480 ? 2 : 1);
      var totalPages = Math.ceil(games.length / cardsPerView);
      html += '<div class="carousel-dots">';
      for (var d = 0; d < totalPages; d++) {
        html += '<span class="carousel-dot' + (d === 0 ? ' active' : '') + '" data-page="' + d + '"></span>';
      }
      html += '</div>';

      html += '</div>';
    });

    container.innerHTML = html;

    // Bind tab clicks
    container.querySelectorAll('.gallery-tab').forEach(function (tab) {
      tab.addEventListener('click', function () {
        currentTab = tab.getAttribute('data-cat');
        buildGallery();
      });
    });

    // Bind carousel controls
    initCarousel();
  }

  function initCarousel() {
    document.querySelectorAll('.carousel-wrapper').forEach(function (wrapper) {
      var viewport = wrapper.querySelector('.carousel-viewport');
      var track = wrapper.querySelector('.carousel-track');
      var prevBtn = wrapper.querySelector('.carousel-prev');
      var nextBtn = wrapper.querySelector('.carousel-next');
      var dots = wrapper.querySelectorAll('.carousel-dot');
      if (!viewport || !track) return;

      var page = 0;

      function getCardsPerView() {
        return window.innerWidth > 768 ? 4 : (window.innerWidth > 480 ? 2 : 1);
      }

      function getTotalPages() {
        var cards = track.querySelectorAll('.carousel-card');
        return Math.ceil(cards.length / getCardsPerView());
      }

      function goTo(p) {
        var total = getTotalPages();
        page = Math.max(0, Math.min(p, total - 1));
        var cpv = getCardsPerView();
        var card = track.querySelector('.carousel-card');
        var gap = parseFloat(window.getComputedStyle(track).columnGap) || 0;
        var pageWidth = card ? cpv * (card.offsetWidth + gap) : viewport.offsetWidth;
        var maxOffset = Math.max(0, track.scrollWidth - viewport.clientWidth);
        var offset = Math.min(page * pageWidth, maxOffset);
        track.style.transform = 'translateX(-' + offset + 'px)';

        dots.forEach(function (d, i) {
          d.classList.toggle('active', i === page);
        });
      }

      if (prevBtn) prevBtn.addEventListener('click', function () { goTo(page - 1); });
      if (nextBtn) nextBtn.addEventListener('click', function () { goTo(page + 1); });

      dots.forEach(function (dot) {
        dot.addEventListener('click', function () {
          goTo(parseInt(dot.getAttribute('data-page'), 10));
        });
      });

      // Touch/swipe support
      var startX = 0;
      var isDragging = false;
      viewport.addEventListener('touchstart', function (e) {
        startX = e.touches[0].clientX;
        isDragging = true;
      }, { passive: true });
      viewport.addEventListener('touchend', function (e) {
        if (!isDragging) return;
        isDragging = false;
        var diff = startX - e.changedTouches[0].clientX;
        if (Math.abs(diff) > 50) {
          goTo(page + (diff > 0 ? 1 : -1));
        }
      }, { passive: true });
    });
  }

  // ================================================================
  // NAVBAR — Scroll effect
  // ================================================================
  var navbar = document.getElementById('navbar');

  function onScroll() {
    navbar.classList.toggle('scrolled', window.scrollY > 50);
  }

  window.addEventListener('scroll', onScroll, { passive: true });
  onScroll();

  // ---- Mobile nav toggle ----
  var toggle = document.querySelector('.nav-toggle');
  var navLinks = document.querySelector('.nav-links');

  if (toggle && navLinks) {
    toggle.addEventListener('click', function () {
      navLinks.classList.toggle('open');
    });
    navLinks.querySelectorAll('a').forEach(function (a) {
      a.addEventListener('click', function () { navLinks.classList.remove('open'); });
    });
  }

  // ================================================================
  // SCROLL REVEAL — Intersection Observer
  // ================================================================
  var fadeEls = document.querySelectorAll('.fade-in-up');

  if ('IntersectionObserver' in window) {
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.1, rootMargin: '0px 0px -40px 0px' });

    fadeEls.forEach(function (el) { observer.observe(el); });
  } else {
    fadeEls.forEach(function (el) { el.classList.add('visible'); });
  }

  // ================================================================
  // ANIMATED COUNTER
  // ================================================================
  var statNumbers = document.querySelectorAll('.stat-number[data-target]');

  function animateCounter(el) {
    var target = parseInt(el.dataset.target, 10);
    var suffix = el.dataset.suffix || '';
    var duration = 1800;
    var start = performance.now();

    function tick(now) {
      var elapsed = now - start;
      var progress = Math.min(elapsed / duration, 1);
      var eased = 1 - Math.pow(1 - progress, 3);
      var current = Math.round(eased * target);
      el.textContent = current.toLocaleString() + suffix;
      if (progress < 1) requestAnimationFrame(tick);
    }

    requestAnimationFrame(tick);
  }

  if ('IntersectionObserver' in window) {
    var statObserver = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          animateCounter(entry.target);
          statObserver.unobserve(entry.target);
        }
      });
    }, { threshold: 0.5 });

    statNumbers.forEach(function (el) { statObserver.observe(el); });
  } else {
    statNumbers.forEach(function (el) { animateCounter(el); });
  }

  // ================================================================
  // PARTICLE CANVAS — Hero background
  // ================================================================
  var canvas = document.getElementById('circuit-canvas');
  if (canvas && canvas.getContext) {
    var ctx = canvas.getContext('2d');
    var w, h, particles;
    var PARTICLE_COUNT = 60;
    var LINE_DIST = 120;

    function resize() {
      w = canvas.width = canvas.offsetWidth;
      h = canvas.height = canvas.offsetHeight;
    }

    function initParticles() {
      particles = [];
      for (var i = 0; i < PARTICLE_COUNT; i++) {
        particles.push({
          x: Math.random() * w,
          y: Math.random() * h,
          vx: (Math.random() - 0.5) * 0.4,
          vy: (Math.random() - 0.5) * 0.4,
          r: Math.random() * 2 + 1
        });
      }
    }

    function draw() {
      ctx.clearRect(0, 0, w, h);

      for (var i = 0; i < particles.length; i++) {
        for (var j = i + 1; j < particles.length; j++) {
          var dx = particles[i].x - particles[j].x;
          var dy = particles[i].y - particles[j].y;
          var dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < LINE_DIST) {
            var alpha = (1 - dist / LINE_DIST) * 0.4;
            ctx.strokeStyle = 'rgba(0, 212, 255, ' + alpha + ')';
            ctx.lineWidth = 0.5;
            ctx.beginPath();
            ctx.moveTo(particles[i].x, particles[i].y);
            ctx.lineTo(particles[j].x, particles[j].y);
            ctx.stroke();
          }
        }
      }

      particles.forEach(function (p) {
        ctx.fillStyle = 'rgba(123, 44, 191, 0.6)';
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fill();

        p.x += p.vx;
        p.y += p.vy;

        if (p.x < 0) p.x = w;
        if (p.x > w) p.x = 0;
        if (p.y < 0) p.y = h;
        if (p.y > h) p.y = 0;
      });

      requestAnimationFrame(draw);
    }

    resize();
    initParticles();
    draw();

    window.addEventListener('resize', function () {
      resize();
      initParticles();
    });
  }

  // ================================================================
  // SMOOTH SCROLL
  // ================================================================
  document.querySelectorAll('a[href^="#"]').forEach(function (anchor) {
    anchor.addEventListener('click', function (e) {
      var target = document.querySelector(anchor.getAttribute('href'));
      if (target) {
        e.preventDefault();
        target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    });
  });

  // ================================================================
  // LANGUAGE TOGGLE
  // ================================================================
  var langBtn = document.getElementById('lang-toggle');
  if (langBtn) {
    langBtn.addEventListener('click', function () {
      applyLang(currentLang === 'zh' ? 'en' : 'zh');
    });
  }

  // ================================================================
  // INIT
  // ================================================================
  applyLang(currentLang);
  buildGallery();

  // Re-init scroll reveal for dynamically added elements
  if ('IntersectionObserver' in window) {
    var revealObserver = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          revealObserver.unobserve(entry.target);
        }
      });
    }, { threshold: 0.1, rootMargin: '0px 0px -40px 0px' });

    // Observe new elements after gallery build
    setTimeout(function () {
      document.querySelectorAll('.fade-in-up:not(.visible)').forEach(function (el) {
        revealObserver.observe(el);
      });
    }, 100);
  }

})();
