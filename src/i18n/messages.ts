export const LOCALES = ["zh-CN", "en-US"] as const;

export type Locale = (typeof LOCALES)[number];

export const DEFAULT_LOCALE: Locale = "zh-CN";

export const messages = {
  "zh-CN": {
    "app.name": "闪改",
    "settings.title": "设置",
    "settings.appearance": "外观",
    "settings.language": "界面语言",
    "settings.nav.general": "通用",
    "settings.nav.personas": "人格仓库",
    "settings.nav.hotkeys": "快捷键",
    "settings.field.apiKey": "API Key",
    "settings.field.baseUrl": "Base URL",
    "settings.field.activePersona": "当前人格",
    "settings.status.active": "已启用",
    "settings.locale.zh": "中文",
    "settings.locale.en": "English",
    "settings.theme.light": "亮色",
    "settings.theme.dark": "暗色",
    "persona.workplaceEq.name": "高情商职场",
    "persona.academicConcise.name": "学术精简",
    "persona.cleanCorrection.name": "纯净纠错",
    "hud.refining": "处理中...",
    "hud.ready": "已就绪",
    "hud.replaced": "已替换",
    "hud.error": "出错",
    "hud.savedToClipboard": "已保存到剪贴板",
  },
  "en-US": {
    "app.name": "Shanka",
    "settings.title": "Settings",
    "settings.appearance": "Appearance",
    "settings.language": "Language",
    "settings.nav.general": "General",
    "settings.nav.personas": "Personas",
    "settings.nav.hotkeys": "Hotkeys",
    "settings.field.apiKey": "API Key",
    "settings.field.baseUrl": "Base URL",
    "settings.field.activePersona": "Active Persona",
    "settings.status.active": "Active",
    "settings.locale.zh": "中文",
    "settings.locale.en": "English",
    "settings.theme.light": "Light",
    "settings.theme.dark": "Dark",
    "persona.workplaceEq.name": "High-EQ Workplace",
    "persona.academicConcise.name": "Academic Concise",
    "persona.cleanCorrection.name": "Clean Correction",
    "hud.refining": "Refining...",
    "hud.ready": "Ready",
    "hud.replaced": "Replaced",
    "hud.error": "Error",
    "hud.savedToClipboard": "Saved to Clipboard",
  },
} as const satisfies Record<Locale, Record<string, string>>;

export type TranslationKey = keyof (typeof messages)[typeof DEFAULT_LOCALE];
