import { Scroll, Text, View } from "nsh";
import { keymap } from "nsh/keymap";

type Appearance = {
  settings: {
    mode: string;
    light_theme: string;
    dark_theme: string;
    accent_source: string;
    accent_color?: string;
    material: string;
    font_size: number;
  };
  policy: { following: boolean };
  capabilities: { materials: string[]; wallpaper_accent: boolean };
};
type ShortcutName = "sidebar" | "create" | "search" | "reload";
type Colors = { background: string; card: string; foreground: string; muted: string; border: string; accent: string };

export function SettingsPanel(props: {
  appearance?: Appearance;
  editable: boolean;
  themes: { key: string; name: string }[];
  colors: Colors;
  shortcuts: Record<ShortcutName, string>;
  onPatch: (patch: Record<string, unknown>) => void;
  onChangeShortcut: (name: ShortcutName) => void;
}) {
  const appearance = () => props.appearance;
  const settings = () => appearance()?.settings;
  const colors = () => props.colors;
  const patch = (values: Record<string, unknown>) => {
    if (props.editable) props.onPatch(values);
  };
  const choice = (label: string, selected: boolean, onPress: () => void) => (
    <View onPress={onPress} role="button" accessibilityState={{ selected }}
      class="rounded-md border px-3 py-2"
      style={{ backgroundColor: selected ? colors().card : colors().background, borderColor: selected ? colors().accent : colors().border }}>
      <Text style={{ color: selected ? colors().foreground : colors().muted }}>{label}</Text>
    </View>
  );
  const shortcutRow = (name: ShortcutName, label: string) => (
    <View class="h-10 flex-row items-center border-b" style={{ borderColor: colors.border }}>
      <Text class="flex-1 text-[13px]" style={{ color: colors().foreground }}>{label}</Text>
      <View onPress={() => props.onChangeShortcut(name)} role="button"
        accessibilityLabel={`Изменить сочетание: ${label}`} class="rounded-md px-3 py-1" style={{ backgroundColor: colors().background }}>
        <Text class="text-[12px]" style={{ color: colors().accent }}>{keymap.label(props.shortcuts[name])}</Text>
      </View>
    </View>
  );
  if (!settings() || !appearance()) {
    return <Scroll class="min-h-0 flex-1 px-7 pt-4 pb-8"><Text class="text-[13px]" style={{ color: colors().muted }}>Загружаем настройки внешнего вида…</Text></Scroll>;
  }
  const accents = ["#8ec07c", "#d79921", "#b18aff", "#83a598", "#fb4934", "#d3869b", "#458588", "#fabd2f"];
  const modes = [["system", "Системная"], ["light", "Светлая"], ["dark", "Тёмная"]];
  return <Scroll class="min-h-0 flex-1 px-7 pt-4 pb-8" style={{ backgroundColor: colors.background }}>
    <View class="mx-auto w-full max-w-[720px]">
      <Text class="mb-6 text-[24px] font-bold" style={{ color: colors().foreground }}>Отображение</Text>
      {appearance()?.policy.following && <Text class="mb-4 text-[12px]" style={{ color: colors().accent }}>Общий стиль задаётся в Mundus Manager. Настройки доступны только для просмотра.</Text>}
      {!props.editable && !appearance()?.policy.following && <Text class="mb-4 text-[12px]" style={{ color: colors().accent }}>Настройки внешнего вида сейчас недоступны.</Text>}
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Режим</Text>
      <View class="mb-6 flex-row gap-2">{modes.map(([mode, label]) => choice(label, settings()!.mode === mode, () => patch({ mode })))}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Светлая тема</Text>
      <View class="mb-5 flex-row flex-wrap gap-2">{props.themes.map((theme) => choice(theme.name, settings()!.light_theme === theme.key, () => patch({ light_theme: theme.key })))}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Тёмная тема</Text>
      <View class="mb-5 flex-row flex-wrap gap-2">{props.themes.map((theme) => choice(theme.name, settings()!.dark_theme === theme.key, () => patch({ dark_theme: theme.key })))}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Акцентный цвет</Text>
      <View class="mb-3 flex-row gap-2">
        {choice("Цвет темы", settings()!.accent_source === "theme", () => patch({ accent_source: "theme", accent_color: null }))}
        {choice("Свой цвет", settings()!.accent_source === "custom", () => patch({ accent_source: "custom", accent_color: settings()!.accent_color ?? accents[0] }))}
        {appearance()!.capabilities.wallpaper_accent && choice("Обои", settings()!.accent_source === "wallpaper", () => patch({ accent_source: "wallpaper" }))}
      </View>
      <View class="mb-6 flex-row gap-2">{accents.map((accent) => <View onPress={() => patch({ accent_source: "custom", accent_color: accent })} role="button" accessibilityLabel={`Акцент ${accent}`} accessibilityState={{ selected: settings()!.accent_source === "custom" && settings()!.accent_color?.toLowerCase() === accent }} class="h-7 w-7 rounded-full border-2" style={{ backgroundColor: accent, borderColor: settings()!.accent_color?.toLowerCase() === accent ? colors().foreground : colors().border }} />)}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors.muted }}>Материал окна</Text>
      <View class="mb-6 flex-row gap-2">{["default", "frosted", "mica"].filter((material) => appearance()!.capabilities.materials.includes(material)).map((material) => choice(material === "default" ? "Сплошной" : material === "frosted" ? "Стекло" : "Системный", settings()!.material === material, () => patch({ material })))}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Плотность интерфейса</Text>
      <View class="mb-6 flex-row gap-2">{[[12, "Компактная"], [13, "Обычная"], [15, "Свободная"]].map(([size, label]) => choice(label, settings()!.font_size === size, () => patch({ font_size: size })))}</View>
      <Text class="mb-3 text-[13px]" style={{ color: colors().muted }}>Горячие клавиши</Text>
      <View class="rounded-xl px-4" style={{ backgroundColor: colors().card }}>
        {shortcutRow("sidebar", "Боковая панель — открыть/скрыть")}
        {shortcutRow("create", "Новая задача — панель справа")}
        {shortcutRow("search", "Быстрый поиск")}
        {shortcutRow("reload", "Обновить данные из Engine")}
        <View class="h-10 flex-row items-center"><Text class="flex-1 text-[13px]" style={{ color: colors().foreground }}>Закрыть панель / поиск</Text><Text class="text-[12px]" style={{ color: colors().muted }}>Esc</Text></View>
      </View>
    </View>
  </Scroll>;
}
