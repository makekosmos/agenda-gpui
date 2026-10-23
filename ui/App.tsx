import { Input, mount, Scroll, signal, Text, View } from "nsh";
import { nativeComponent } from "nsh/native";

type Route = "inbox" | "week" | "next-week" | "someday" | "logbook" | "project" | "statistics" | "settings" | "about" | "trash";
type Todo = { id: string; title: string; is_completed: boolean; scheduled_date?: string; notes?: string };
type Project = { id: string; title: string; status: number };
type Snapshot = { route: Route; title: string; todos: Todo[]; projects: Project[]; project?: Project; archivedCount: number; trashCount: number; todoCount: number; version: string; activity: { label: string; done: number }[] };

const initial: Snapshot = { route: "inbox", title: "Входящие", todos: [], projects: [], archivedCount: 0, trashCount: 0, todoCount: 0, version: "", activity: [] };
const snapshot = signal(initial);
const route = signal<Route>("inbox");
const selectedProjectId = signal("dev-proj-release");
const otherOpen = signal(false);
const addOpen = signal(false);
const newTitle = signal("");
const Bridge = nativeComponent<{ route: Route; projectId: string }, { state: Snapshot }>("agenda.bridge", {
  props: ["route", "projectId"],
  events: ["state"],
});

const navigation: [Route, string, string][] = [
  ["inbox", "▱", "Входящие"],
  ["week", "▦", "Эта неделя"],
  ["next-week", "▦", "Следующая неделя"],
  ["someday", "◷", "Потом"],
];
const otherPages: [Route, string, string][] = [
  ["logbook", "▤", "Архив"],
  ["statistics", "▥", "Активность"],
  ["settings", "⚙", "Настройки"],
  ["about", "ⓘ", "О приложении"],
  ["trash", "⌫", "Корзина"],
];

function NavRow(props: { item: [Route, string, string] }) {
  const [target, icon, label] = props.item;
  return (
    <View
      onPress={() => route.set(target)}
      role="button"
      accessibilityState={{ selected: route() === target }}
      class={`h-8 flex-row items-center gap-2 rounded-lg px-2.5 ${route() === target ? "bg-[#28282c]" : ""}`}
    >
      <Text class="w-4 text-center text-[15px] text-[#96969e]">{icon}</Text>
      <Text class={`text-[13px] ${route() === target ? "text-[#f2f2f4]" : "text-[#a0a0aa]"}`}>{label}</Text>
    </View>
  );
}

function TaskRows() {
  return <Each of={snapshot().todos}>{(todo) => (
    <View class="h-[37px] flex-row items-center border-b border-[#242428] px-3">
      <View onPress={() => commandRef?.("complete", { id: todo().id })} role="checkbox" accessibilityLabel={todo().title} accessibilityState={{ checked: todo().is_completed }} class={`mr-3 h-[14px] w-[14px] rounded-full border ${todo().is_completed ? "border-[#b18aff] bg-[#b18aff]" : "border-[#74747d]"}`}>
        {todo().is_completed && <Text class="text-[9px] text-[#17171a]">✓</Text>}
      </View>
      <Text class={`text-[13px] ${todo().is_completed ? "text-[#85858e] line-through" : "text-[#f0f0f2]"}`}>{todo().title}</Text>
    </View>
  )}</Each>;
}

let commandRef: ((name: string, args?: unknown) => void) | undefined;

function SectionTitle(props: { children: string }) {
  return <Text class="mb-3 text-[13px] text-[#96969e]">{props.children}</Text>;
}

function SettingsPage() {
  return <Scroll class="min-h-0 flex-1 px-7 pt-4 pb-8">
    <View class="mx-auto w-full max-w-[720px]">
      <Text class="mb-6 text-[24px] font-bold text-[#f0f0f2]">Отображение</Text>
      <Text class="mb-4 text-[13px] text-[#82828c]">Внешний вид</Text>
      <View class="mb-6 flex-row gap-4">
        {[["▤", "Системная"], ["☼", "Светлая"], ["☾", "Тёмная"]].map(([icon, label]) => <View class="h-[138px] flex-1 rounded-lg border border-[#242428] bg-[#151518] p-3">
          <View class="mb-3 h-[75px] rounded-md bg-[#222226] p-2"><View class="h-2 w-1/2 rounded bg-[#45454b]" /><View class="mt-2 h-2 w-4/5 rounded bg-[#37373d]" /></View>
          <Text class="text-center text-[12px] text-[#888891]">{icon} {label}</Text>
        </View>)}
      </View>
      <View class="mb-6 rounded-xl bg-[#17171a] p-4">
        {["Светлая тема", "Тёмная тема", "Акцентный цвет", "Цвета обоев"].map((label) => <View class="h-11 flex-row items-center border-b border-[#252529]"><Text class="flex-1 text-[13px] text-[#e1e1e5]">{label}</Text><Text class="text-[12px] text-[#85858e]">По умолчанию</Text></View>)}
      </View>
      <SectionTitle>Материал</SectionTitle>
      <View class="mb-6 h-14 flex-row items-center rounded-xl bg-[#17171a] px-4"><Text class="flex-1 text-[13px] text-[#e1e1e5]">Стекло</Text><Text class="text-[12px] text-[#85858e]">Сплошные поверхности</Text></View>
      <SectionTitle>Шрифты</SectionTitle>
      <View class="mb-6 h-14 flex-row items-center rounded-xl bg-[#17171a] px-4"><Text class="flex-1 text-[13px] text-[#e1e1e5]">Шрифт интерфейса</Text><Text class="text-[12px] text-[#85858e]">Системный</Text></View>
      <SectionTitle>Горячие клавиши</SectionTitle>
      <View class="rounded-xl bg-[#17171a] px-4">
        {["Боковая панель — открыть/скрыть", "Новая задача — панель справа", "Быстрый поиск", "Обновить данные из Engine", "Сохранить задачу / название", "Новая строка в поле", "Закрыть панель / меню / поиск"].map((label, i) => <View class="h-10 flex-row items-center"><Text class="flex-1 text-[13px] text-[#e1e1e5]">{label}</Text><Text class="text-[12px] text-[#85858e]">{["Win+B", "Win+N", "Win+K", "Win+R", "Enter", "Shift+Enter", "Esc"][i]}</Text></View>)}
      </View>
    </View>
  </Scroll>;
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
      <View class="mt-3 h-[200px] border-b border-[#3a3a40]">
        {["20%", "40%", "60%", "80%"].map((_) => <View class="h-1/4 border-b border-dashed border-[#252529]" />)}
      </View>
      <View class="mt-2 flex-row justify-between px-4"><Text class="text-[10px] text-[#85858e]">1 дек.</Text><Text class="text-[10px] text-[#85858e]">12 янв.</Text><Text class="text-[10px] text-[#85858e]">23 февр.</Text><Text class="text-[10px] text-[#85858e]">6 апр.</Text><Text class="text-[10px] text-[#85858e]">18 мая</Text><Text class="text-[10px] text-[#85858e]">29 июня</Text><Text class="text-[10px] text-[#85858e]">10 авг.</Text><Text class="text-[10px] text-[#85858e]">21 сент.</Text></View>
    </View>
  </View>;
}

function InfoPage() {
  return <Scroll class="flex-1 px-7 pt-12">
    <View class="mx-auto w-full max-w-[760px]">
      <View class="mb-6 items-center">
        <Text class="mb-2 text-[64px] text-[#b18aff]">▦</Text>
        <Text class="mb-2 text-[23px] font-bold text-[#f0f0f2]">Agenda</Text>
        <Text class="mb-2 text-[13px] text-[#a0a0aa]">Версия {snapshot().version}</Text>
        <Text class="text-[12px] text-[#85858e]">© Yoso Industries 2025–2026</Text>
      </View>
      <Text class="mb-3 text-[13px] text-[#e1e1e5]">Диагностика</Text>
      <View class="h-10 flex-row items-center rounded-xl bg-[#17171a] px-4">
        <View class="mr-3 h-4 w-4 items-center justify-center rounded-full border border-[#8ec07c]"><Text class="text-[10px] text-[#8ec07c]">✓</Text></View>
        <Text class="text-[13px] text-[#e1e1e5]">Всё в порядке</Text>
      </View>
    </View>
  </Scroll>;
}

function SummaryPage() {
  const route = snapshot().route;
  const count = route === "logbook" ? snapshot().archivedCount : snapshot().trashCount;
  return <View class="flex-1 items-center justify-center">
    <Text class="mb-2 text-[14px] text-[#a0a0aa]">{route === "logbook" ? "Архив пуст" : "Корзина пуста"}</Text>
    <Text class="text-[12px] text-[#777780]">{count ? `${count} задач` : route === "logbook" ? "Завершённые задачи хранятся здесь" : "Удалённые задачи попадают сюда"}</Text>
  </View>;
}

function App() {
  return (
    <View class="h-full w-full flex-row bg-[#0c0c0e]">
      <Bridge
        class="h-0 w-0"
        ref={(node) => { commandRef = (name, args) => node.command(name, args); }}
        route={route()}
        projectId={selectedProjectId()}
        onState={(next) => {
          route.set(next.route);
          snapshot.set(next);
        }}
      />
      <View class="w-[240px] shrink-0 border-r border-[#222226] px-2 pt-1">
        <View class="gap-0.5">
          {navigation.map((item) => <NavRow item={item} />)}
        </View>
        <View class="mt-5 rounded-lg bg-[#17171a] px-2 pt-2">
          <View class="mb-1 h-8 flex-row items-center gap-2 px-0.5">
            <Text class="text-[13px] text-[#a0a0aa]">▱</Text>
            <Text class="flex-1 text-[13px] text-[#a0a0aa]">Проекты</Text>
            <Text class="text-[17px] text-[#a0a0aa]">＋</Text>
          </View>
          <Each of={snapshot().projects}>
            {(project) => (
              <View onPress={() => { selectedProjectId.set(project().id); route.set("project"); }} role="button" class="h-8 flex-row items-center gap-2">
                <View class="h-2 w-2 rounded-full bg-[#66666f]" />
                <Text class="text-[13px] text-[#a0a0aa]">{project().title}</Text>
              </View>
            )}
          </Each>
        </View>
        <View class="mt-auto mb-2">
          {otherOpen() && (
            <View class="mb-2 rounded-lg bg-[#17171a] p-1">
              {otherPages.map((item) => <NavRow item={item} />)}
            </View>
          )}
          <View onPress={() => otherOpen.update((open) => !open)} role="button" class="h-8 flex-row items-center gap-2 px-2.5">
            <Text class="w-4 text-center text-[15px] text-[#96969e]">···</Text>
            <Text class="text-[13px] text-[#a0a0aa]">Другое</Text>
          </View>
        </View>
      </View>
      <View class="relative min-w-0 flex-1">
        {(route() === "inbox" || route() === "week" || route() === "next-week" || route() === "someday" || route() === "project") && <View class="flex-1">
          {route() === "project" && <View class="px-4 pt-[36px] pb-2"><Text class="text-[24px] font-bold text-[#f0f0f2]">{snapshot().project?.title ?? "Проект"}</Text></View>}
          <TaskRows />
        </View>}
        {route() === "logbook" && <View class="flex-1"><View class="h-8 flex-row items-center px-7"><Text class="text-[13px] text-[#96969e]">Задачи · {snapshot().todos.length}</Text></View><TaskRows /></View>}
        {route() === "trash" && <TaskRows />}
        {route() === "statistics" && <StatisticsPage />}
        {route() === "settings" && <SettingsPage />}
        {route() === "about" && <InfoPage />}
        {(route() === "inbox" || route() === "week" || route() === "next-week" || route() === "someday" || route() === "project") && <View onPress={() => { addOpen.set(true); newTitle.set(""); }} role="button" accessibilityLabel="Новая задача" class="absolute right-6 bottom-6 h-11 w-11 items-center justify-center rounded-full bg-[#f0f0f2]">
          <Text class="text-[22px] text-[#0c0c0e]">＋</Text>
        </View>}
        {addOpen() && <View class="absolute inset-0 items-center justify-center bg-[#00000080] px-4">
          <View class="w-full max-w-[440px] rounded-xl border border-[#303036] bg-[#17171a] p-4">
            <Text class="mb-3 text-[14px] text-[#f0f0f2]">Новая задача</Text>
            <Input value={newTitle()} onChange={(text) => newTitle.set(text)} onSubmit={(event) => { commandRef?.("create", { title: event.text }); addOpen.set(false); }} placeholder="Название задачи" class="h-10 rounded-lg bg-[#222226] px-3 text-[13px] text-[#f0f0f2]" />
            <View class="mt-3 flex-row justify-end gap-2">
              <View onPress={() => addOpen.set(false)} role="button" class="rounded-md px-3 py-2"><Text class="text-[13px] text-[#a0a0aa]">Отмена</Text></View>
              <View onPress={() => { commandRef?.("create", { title: newTitle() }); addOpen.set(false); }} role="button" accessibilityLabel="Добавить" class="h-9 w-[90px] items-center justify-center rounded-md bg-[#b18aff]"><Text class="text-[13px] text-[#17171a]">Добавить</Text></View>
            </View>
          </View>
        </View>}
      </View>
    </View>
  );
}

mount(App);
