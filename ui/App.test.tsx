import { expect, open, test } from "nsh/test";

test("shows the main Agenda routes", async () => {
  const app = await open("./App.tsx");
  expect(app.texts()).toContain("Входящие");
  expect(app.texts()).toContain("Эта неделя");
  expect(app.texts()).toContain("Следующая неделя");
  expect(app.texts()).toContain("Потом");
});

test("opens secondary routes from the sidebar", async () => {
  const app = await open("./App.tsx");
  await app.press("Другое");
  expect(app.texts()).toContain("Архив");
  expect(app.texts()).toContain("Активность");
  expect(app.texts()).toContain("Настройки");
  expect(app.texts()).toContain("О приложении");
  expect(app.texts()).toContain("Корзина");
});

test("opens and closes a task draft", async () => {
  const app = await open("./App.tsx");
  await app.press("Новая задача");
  expect(app.texts()).toContain("Отмена");
  await app.press("Добавить");
  expect(app.texts()).not.toContain("Отмена");
  expect(app.texts()).not.toContain("Добавить");
});

test("submits a task draft from the title field", async () => {
  const app = await open("./App.tsx");
  await app.press("Новая задача");
  await app.type("Название задачи", "Проверить интерфейс");
  await app.keyboard.press("enter");
  expect(app.texts()).not.toContain("Отмена");
});
