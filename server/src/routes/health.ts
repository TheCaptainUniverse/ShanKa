import { Hono } from "hono";

export const healthRoute = new Hono();

healthRoute.get("/", (context) => {
  return context.json({
    ok: true,
    service: "shanka-local-bus",
    timestamp: new Date().toISOString(),
  });
});
