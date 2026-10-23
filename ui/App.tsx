import { Canvas } from "nsh/canvas";
import { effect, Input, Image, mount, Scroll, signal, Text, View } from "nsh";
import { nativeComponent } from "nsh/native";
import { keymap } from "nsh/keymap";
import { Storage } from "nsh/storage";
import { SettingsPanel } from "./SettingsPanel";
import { AboutIcon, CheckIcon, InboxIcon, LogbookIcon, MoreIcon, NextWeekIcon, PlusIcon, ProjectIcon, SettingsIcon, SomedayIcon, StatisticsIcon, TrashIcon, WeekIcon } from "./icons";

type Route = "inbox" | "week" | "next-week" | "someday" | "logbook" | "project" | "statistics" | "settings" | "about" | "trash";
type Todo = { id: string; title: string; is_completed: boolean; scheduled_date?: string; notes?: string };
type Project = { id: string; title: string; status: number };
type Appearance = { settings: { mode: string; light_theme: string; dark_theme: string; accent_source: string; accent_color?: string; material: string; font_size: number; font_family: string }; policy: { following: boolean; editable: boolean }; capabilities: { materials: string[]; wallpaper_accent: boolean } };
type Theme = { key: string; name: string };
type Colors = { background: string; foreground: string; muted: string; border: string; accent: string; card: string };
type Snapshot = { route: Route; title: string; todos: Todo[]; projects: Project[]; project?: Project; archivedCount: number; trashCount: number; todoCount: number; version: string; activity: { label: string; done: number }[]; appearance?: Appearance; appearanceEditable: boolean; themes: Theme[]; colors: Colors; sidebarVisible: boolean };

const initial: Snapshot = { route: "inbox", title: "Входящие", todos: [], projects: [], archivedCount: 0, trashCount: 0, todoCount: 0, version: "", activity: [], appearance: { settings: { mode: "dark", light_theme: "default", dark_theme: "default", accent_source: "theme", material: "default", font_size: 13, font_family: "Inter" }, policy: { following: false, editable: true }, capabilities: { materials: ["default", "frosted", "mica"], wallpaper_accent: false } }, appearanceEditable: true, themes: [{ key: "default", name: "Default" }, { key: "claude", name: "Claude" }, { key: "graphite", name: "Graphite" }], colors: { background: "#0d0c10", foreground: "#e7e5ea", muted: "#8d8a94", border: "#27262a", accent: "#af94f9", card: "#131216" }, sidebarVisible: true };
const snapshot = signal(initial);
const route = signal<Route>("inbox");
const selectedProjectId = signal("dev-proj-release");
const otherOpen = signal(false);
const addOpen = signal(false);
const newTitle = signal("");
const quickSearchOpen = signal(false);
const quickSearchQuery = signal("");
const sidebarVisible = signal(true);
const shortcutNames = ["sidebar", "create", "search", "reload"] as const;
type ShortcutName = typeof shortcutNames[number];
type Shortcuts = Record<ShortcutName, string>;
const defaultShortcuts: Shortcuts = { sidebar: "mod+b", create: "mod+n", search: "mod+k", reload: "mod+r" };
const shortcutOptions: Record<ShortcutName, string[]> = {
  sidebar: ["mod+b", "mod+shift+b"],
  create: ["mod+n", "mod+shift+n"],
  search: ["mod+k", "mod+shift+k"],
  reload: ["mod+r", "mod+shift+r"],
};
const shortcuts = signal<Shortcuts>(defaultShortcuts);
void Storage.getItem("mundus.agenda.shortcuts.v1").then((value) => {
  if (!value) return;
  try {
    const saved = JSON.parse(value) as Partial<Shortcuts>;
    shortcuts.set(Object.fromEntries(shortcutNames.map((name) => [name, shortcutOptions[name].includes(saved[name] ?? "") ? saved[name] : defaultShortcuts[name]])) as Shortcuts);
  } catch { /* A damaged preference falls back to the defaults. */ }
});
const Bridge = nativeComponent<{ route: Route; projectId: string }, { state: Snapshot }>("agenda.bridge", {
  props: ["route", "projectId"],
  events: ["state"],
});

const navigation: [Route, string, string][] = [
  ["inbox", InboxIcon, "Входящие"],
  ["week", WeekIcon, "Эта неделя"],
  ["next-week", NextWeekIcon, "Следующая неделя"],
  ["someday", SomedayIcon, "Потом"],
];
const otherPages: [Route, string, string][] = [
  ["logbook", LogbookIcon, "Архив"],
  ["statistics", StatisticsIcon, "Активность"],
  ["settings", SettingsIcon, "Настройки"],
  ["about", AboutIcon, "О приложении"],
  ["trash", TrashIcon, "Корзина"],
];

function NavRow(props: { item: [Route, ImageAsset, string] }) {
  const [target, icon, label] = props.item;
  return (
    <View
      onPress={() => route.set(target)}
      role="button"
      accessibilityState={{ selected: route() === target }}
      class="h-8 flex-row items-center gap-2 rounded-lg px-2.5"
      style={{ backgroundColor: route() === target ? snapshot().colors.card : "transparent" }}
    >
      <Image src={icon} style={{ width: 16, height: 16, tintColor: snapshot().colors.muted }} />
      <Text class="text-[13px]" style={{ color: route() === target ? snapshot().colors.foreground : snapshot().colors.muted }}>{label}</Text>
    </View>
  );
}

function TaskRows() {
  return <Each of={snapshot().todos}>{(todo) => (
    <View class="flex-row items-center border-b px-3" style={{ height: 37 * (snapshot().appearance?.settings.font_size ?? 13) / 13, borderColor: snapshot().colors.border }}>
      <View onPress={() => commandRef?.("complete", { id: todo().id })} role="checkbox" accessibilityLabel={todo().title} accessibilityState={{ checked: todo().is_completed }} class="mr-3 h-[14px] w-[14px] items-center justify-center rounded-full border" style={{ borderColor: todo().is_completed ? snapshot().colors.accent : snapshot().colors.muted, backgroundColor: todo().is_completed ? snapshot().colors.accent : "transparent" }}>
        {todo().is_completed && <Image src={CheckIcon} style={{ width: 10, height: 10, tintColor: snapshot().colors.background }} />}
      </View>
      <Text class={todo().is_completed && snapshot().route !== "logbook" ? "line-through" : ""} style={{ fontSize: snapshot().appearance?.settings.font_size ?? 13, color: todo().is_completed && snapshot().route !== "logbook" ? snapshot().colors.muted : snapshot().colors.foreground }}>{todo().title}</Text>
    </View>
  )}</Each>;
}

let commandRef: ((name: string, args?: unknown) => void) | undefined;

function setAppearance(patch: Record<string, unknown>) {
  const current = snapshot();
  if (current.appearance && current.appearanceEditable) {
    snapshot.set({
      ...current,
      appearance: {
        ...current.appearance,
        settings: { ...current.appearance.settings, ...patch } as Appearance["settings"],
      },
    });
  }
  commandRef?.("appearance.set", patch);
}

function cycleShortcut(name: ShortcutName) {
  const current = shortcuts();
  const choices = shortcutOptions[name];
  const next = choices[(choices.indexOf(current[name]) + 1) % choices.length];
  const updated = { ...current, [name]: next };
  shortcuts.set(updated);
  void Storage.setItem("mundus.agenda.shortcuts.v1", JSON.stringify(updated));
}

effect(() => {
  const active = shortcuts();
  const unbind = [
    keymap.bind(active.sidebar, () => commandRef?.("toggle-sidebar"), { when: "notEditing" }),
    keymap.bind(active.create, () => { newTitle.set(""); addOpen.set(true); }, { when: "notEditing" }),
    keymap.bind(active.search, () => { quickSearchQuery.set(""); quickSearchOpen.set(true); }, { when: "notEditing" }),
    keymap.bind(active.reload, () => commandRef?.("reload"), { when: "notEditing" }),
  ];
  return () => unbind.forEach((remove) => remove());
});

function SectionTitle(props: { children: string }) {
  return <Text class="mb-3 text-[13px]" style={{ color: snapshot().colors.muted }}>{props.children}</Text>;
}

function SettingsPage() {
  return <SettingsPanel
    appearance={snapshot().appearance}
    editable={snapshot().appearanceEditable}
    themes={snapshot().themes}
    colors={snapshot().colors}
    shortcuts={shortcuts()}
    onPatch={setAppearance}
    onChangeShortcut={cycleShortcut}
  />;
}

function StatisticsPage() {
  return <View class="flex-1 px-7 pt-0">
    <View class="mx-auto w-full max-w-[665px]">
      <View class="h-[90px] flex-row items-end justify-between border-b border-dashed border-[#28282c] pb-2">
        <Each of={snapshot().activity}>{(point) => <View class="h-[55px] flex-1 flex-col justify-between px-px">
          {Array.from({ length: 5 }, (_, j) => {
            const active = j >= 5 - Math.min(5, Math.max(0, Math.floor(point().done)));
            return <View class={`h-[7px] w-[7px] rounded-full ${active ? (j === 4 ? "bg-[#8ec07c]" : "bg-[#d79921]") : "bg-[#29292e]"}`} />;
          })}
        </View>}</Each>
      </View>
      <View class="mt-3 h-[200px] border-b" style={{ borderColor: snapshot().colors.border }}>
        {["20%", "40%", "60%", "80%"].map((_) => <View class="h-1/4 border-b border-dashed" style={{ borderColor: snapshot().colors.border }} />)}
        <Canvas class="absolute inset-0" draw={(ctx, size) => {
          const values = snapshot().activity;
          if (values.length < 2) return;
          const maximum = Math.max(1, ...values.map((point) => point.done));
          ctx.beginPath();
          values.forEach((point, index) => {
            const x = (index / (values.length - 1)) * size.width;
            const y = size.height - (point.done / maximum) * size.height;
            if (index === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
          });
          ctx.strokeStyle = snapshot().colors.accent;
          ctx.lineWidth = 1.5;
          ctx.stroke();
        }} />
      </View>
      <View class="mt-2 flex-row justify-between px-4"><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>1 дек.</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>12 янв.</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>23 февр.</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>6 апр.</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>18 мая</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>29 июня</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>10 авг.</Text><Text class="text-[10px]" style={{ color: snapshot().colors.muted }}>21 сент.</Text></View>
    </View>
  </View>;
}

function InfoPage() {
  return <Scroll class="flex-1 px-7 pt-12">
    <View class="mx-auto w-full max-w-[760px]">
      <View class="mb-6 items-center">
        <Image src={WeekIcon} style={{ width: 64, height: 64, tintColor: snapshot().colors.accent }} />
        <Text class="mb-2 text-[23px] font-bold" style={{ color: snapshot().colors.foreground }}>Agenda</Text>
        <Text class="mb-2 text-[13px]" style={{ color: snapshot().colors.muted }}>Версия {snapshot().version}</Text>
        <Text class="text-[12px]" style={{ color: snapshot().colors.muted }}>© Yoso Industries 2025–2026</Text>
      </View>
      <Text class="mb-3 text-[13px]" style={{ color: snapshot().colors.foreground }}>Диагностика</Text>
      <View class="h-10 flex-row items-center rounded-xl px-4" style={{ backgroundColor: snapshot().colors.card }}>
        <View class="mr-3 h-4 w-4 items-center justify-center rounded-full border" style={{ borderColor: snapshot().colors.accent }}><Image src={CheckIcon} style={{ width: 10, height: 10, tintColor: snapshot().colors.accent }} /></View>
        <Text class="text-[13px]" style={{ color: snapshot().colors.foreground }}>Всё в порядке</Text>
      </View>
    </View>
  </Scroll>;
}

function SummaryPage() {
  const route = snapshot().route;
  const count = route === "logbook" ? snapshot().archivedCount : snapshot().trashCount;
  return <View class="flex-1 items-center justify-center">
    <Text class="mb-2 text-[14px]" style={{ color: snapshot().colors.muted }}>{route === "logbook" ? "Архив пуст" : "Корзина пуста"}</Text>
    <Text class="text-[12px]" style={{ color: snapshot().colors.muted }}>{count ? `${count} задач` : route === "logbook" ? "Завершённые задачи хранятся здесь" : "Удалённые задачи попадают сюда"}</Text>
  </View>;
}

function App() {
  return (
    <View class="h-full w-full flex-row" style={{ backgroundColor: snapshot().colors.background }}>
      <Bridge
        class="absolute left-0 top-0 h-px w-px opacity-0"
        ref={(node) => {
          commandRef = (name, args) => node.command(name, args);
          setTimeout(() => commandRef?.("snapshot"), 50);
        }}
        route={route()}
        projectId={selectedProjectId()}
        onState={(next) => {
          route.set(next.route);
          snapshot.set(next);
          sidebarVisible.set(next.sidebarVisible);
        }}
      />
      {sidebarVisible() && <View class="w-[240px] shrink-0 border-r px-2 pt-1" style={{ borderColor: snapshot().colors.border }}>
        <View class="gap-0.5">
          {navigation.map((item) => <NavRow item={item} />)}
        </View>
        <View class="mt-5 rounded-lg px-2 pt-2" style={{ backgroundColor: snapshot().colors.card }}>
          <View class="mb-1 h-8 flex-row items-center gap-2 px-0.5">
            <Image src={ProjectIcon} style={{ width: 16, height: 16, tintColor: snapshot().colors.muted }} />
            <Text class="flex-1 text-[13px]" style={{ color: snapshot().colors.muted }}>Проекты</Text>
            <Image src={PlusIcon} style={{ width: 16, height: 16, tintColor: snapshot().colors.muted }} />
          </View>
          <Each of={snapshot().projects}>
            {(project) => (
              <View onPress={() => { selectedProjectId.set(project().id); route.set("project"); }} role="button" class="h-8 flex-row items-center gap-2">
                <View class="h-2 w-2 rounded-full" style={{ backgroundColor: snapshot().colors.muted }} />
                <Text class="text-[13px]" style={{ color: snapshot().colors.muted }}>{project().title}</Text>
              </View>
            )}
          </Each>
        </View>
        <View class="mt-auto mb-2">
          {otherOpen() && (
            <View class="mb-2 rounded-lg p-1" style={{ backgroundColor: snapshot().colors.card }}>
              {otherPages.map((item) => <NavRow item={item} />)}
            </View>
          )}
          <View onPress={() => otherOpen.update((open) => !open)} role="button" class="h-8 flex-row items-center gap-2 px-2.5">
            <Image src={MoreIcon} style={{ width: 16, height: 16, tintColor: snapshot().colors.muted }} />
            <Text class="text-[13px]" style={{ color: snapshot().colors.muted }}>Другое</Text>
          </View>
        </View>
      </View>}
      <View class="relative min-w-0 flex-1">
        {(route() === "inbox" || route() === "week" || route() === "next-week" || route() === "someday" || route() === "project") && <View class="flex-1">
          {route() === "project" && <View class="px-6 pt-6 pb-4"><Text class="text-[24px] font-bold" style={{ color: snapshot().colors.foreground }}>{snapshot().project?.title ?? "Проект"}</Text></View>}
          <TaskRows />
        </View>}
        {route() === "logbook" && <View class="flex-1"><View class="h-8 flex-row items-center px-7 pt-1"><Text class="text-[13px]" style={{ color: snapshot().colors.muted }}>Задачи · {snapshot().todos.length}</Text></View><TaskRows /></View>}
        {route() === "trash" && <TaskRows />}
        {route() === "statistics" && <StatisticsPage />}
        {route() === "settings" && <SettingsPage />}
        {route() === "about" && <InfoPage />}
        {(route() === "inbox" || route() === "week" || route() === "next-week" || route() === "someday" || route() === "project") && <View onPress={() => { addOpen.set(true); newTitle.set(""); }} role="button" accessibilityLabel="Новая задача" class="absolute right-6 bottom-6 h-11 w-11 items-center justify-center rounded-full" style={{ backgroundColor: snapshot().colors.foreground }}>
          <Image src={PlusIcon} style={{ width: 20, height: 20, tintColor: snapshot().colors.background }} />
        </View>}
        {addOpen() && <View class="absolute inset-0 items-center justify-center bg-[#00000080] px-4">
          <View class="w-full max-w-[440px] rounded-xl border p-4" style={{ borderColor: snapshot().colors.border, backgroundColor: snapshot().colors.card }}>
            <Text class="mb-3 text-[14px]" style={{ color: snapshot().colors.foreground }}>Новая задача</Text>
            <Input value={newTitle()} onChange={(text) => newTitle.set(text)} onSubmit={(event) => { commandRef?.("create", { title: event.text }); addOpen.set(false); }} placeholder="Название задачи" class="h-10 rounded-lg px-3 text-[13px]" style={{ backgroundColor: snapshot().colors.background, color: snapshot().colors.foreground }} />
            <View class="mt-3 flex-row justify-end gap-2">
              <View onPress={() => addOpen.set(false)} role="button" class="rounded-md px-3 py-2"><Text class="text-[13px]" style={{ color: snapshot().colors.muted }}>Отмена</Text></View>
              <View onPress={() => { commandRef?.("create", { title: newTitle() }); addOpen.set(false); }} role="button" accessibilityLabel="Добавить" class="h-9 w-[90px] items-center justify-center rounded-md" style={{ backgroundColor: snapshot().colors.accent }}><Text class="text-[13px]" style={{ color: snapshot().colors.background }}>Добавить</Text></View>
            </View>
          </View>
        </View>}
        {quickSearchOpen() && <View class="absolute inset-0 items-center justify-center bg-[#00000080] px-4">
          <View class="w-full max-w-[440px] rounded-xl border p-4" style={{ borderColor: snapshot().colors.border, backgroundColor: snapshot().colors.card }}>
            <Text class="mb-3 text-[14px]" style={{ color: snapshot().colors.foreground }}>Быстрый поиск</Text>
            <Input value={quickSearchQuery()} onChange={(text) => quickSearchQuery.set(text)} placeholder="Найти задачу на этом экране" class="h-10 rounded-lg px-3 text-[13px]" style={{ backgroundColor: snapshot().colors.background, color: snapshot().colors.foreground }} />
            <Each of={snapshot().todos.filter((todo) => todo.title.toLowerCase().includes(quickSearchQuery().toLowerCase()))}>{(todo) => <View onPress={() => quickSearchOpen.set(false)} role="button" class="h-9 justify-center border-b" style={{ borderColor: snapshot().colors.border }}><Text class="text-[13px]" style={{ color: snapshot().colors.foreground }}>{todo().title}</Text></View>}</Each>
            <View onPress={() => quickSearchOpen.set(false)} role="button" class="mt-3 self-end rounded-md px-3 py-2"><Text class="text-[13px]" style={{ color: snapshot().colors.muted }}>Закрыть</Text></View>
          </View>
        </View>}
      </View>
    </View>
  );
}

mount(App);
