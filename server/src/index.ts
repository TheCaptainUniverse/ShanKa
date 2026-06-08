import { Hono } from "hono";
import { healthRoute } from "./routes/health";
import { refineRoute } from "./routes/refine";

const app = new Hono();

app.route("/health", healthRoute);
app.route("/refine", refineRoute);

app.onError((error, context) => {
  console.error("[server:error]", error);
  return context.json(
    {
      ok: false,
      durationMs: 0,
      errorCode: "API_ERROR",
      message: "Internal server error",
    },
    500,
  );
});

const port = Number(Bun.env.SHANKA_SERVER_PORT ?? 4317);

export default {
  port,
  hostname: "127.0.0.1",
  fetch: app.fetch,
};

console.info(`[server] Shanka local bus listening on http://127.0.0.1:${port}`);
