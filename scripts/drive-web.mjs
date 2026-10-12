// Minimal CDP driver for the wasm bundle — no dependencies (Node >= 22).
// Usage:
//   node scripts/drive-web.mjs <chrome-debug-port> <url> '<json-steps>'
// Steps: {"wait": ms} {"eval": "expr"} {"shot": "path.png"}
//        {"click": [x, y]} {"type": "text"} {"key": "Enter"}
// Console messages and eval results are printed to stdout.
import { writeFileSync } from "node:fs";

const [port, url, stepsJson] = process.argv.slice(2);
const steps = stepsJson ? JSON.parse(stepsJson) : [{ wait: 5000 }];

const tabs = await fetch(`http://127.0.0.1:${port}/json`).then((r) => r.json());
const page = tabs.find((t) => t.type === "page") ?? tabs[0];
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = rej;
});

let seq = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++seq;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((res, rej) => pending.set(id, { res, rej }));
}
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && pending.has(msg.id)) {
    const { res } = pending.get(msg.id);
    pending.delete(msg.id);
    res(msg.result);
    return;
  }
  if (msg.method === "Runtime.consoleAPICalled") {
    const text = msg.params.args
      .map((a) => a.value ?? a.description ?? "")
      .join(" ");
    console.log(`[console.${msg.params.type}] ${text}`);
  }
  if (msg.method === "Runtime.exceptionThrown") {
    console.log(`[exception] ${JSON.stringify(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text)}`);
  }
};

await send("Runtime.enable");
await send("Page.enable");
await send("Network.enable");
await send("Network.setCacheDisabled", { cacheDisabled: true });
await send("Page.navigate", { url });

for (const step of steps) {
  if (step.wait) {
    await new Promise((r) => setTimeout(r, step.wait));
  } else if (step.eval) {
    const r = await send("Runtime.evaluate", {
      expression: step.eval,
      returnByValue: true,
      awaitPromise: true,
    });
    console.log(`[eval] ${JSON.stringify(r?.result?.value ?? r)}`);
  } else if (step.shot) {
    const r = await send("Page.captureScreenshot", { format: "png" });
    writeFileSync(step.shot, Buffer.from(r.data, "base64"));
    console.log(`[shot] ${step.shot}`);
  } else if (step.click) {
    const [x, y] = step.click;
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await send("Input.dispatchMouseEvent", {
        type,
        x,
        y,
        button: "left",
        clickCount: 1,
      });
    }
    console.log(`[click] ${x},${y}`);
  } else if (step.type) {
    await send("Input.insertText", { text: step.type });
    console.log(`[type] ${step.type}`);
  } else if (step.key) {
    await send("Input.dispatchKeyEvent", { type: "keyDown", key: step.key });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: step.key });
    console.log(`[key] ${step.key}`);
  }
}
ws.close();
process.exit(0);
