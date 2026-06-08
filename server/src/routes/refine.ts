import { Hono } from "hono";
import { z } from "zod";
import type { RefineResponse } from "@shared";
import { refineText } from "../services/refine-service";

const refineRequestSchema = z.object({
  text: z.string().trim().min(1),
  mode: z.enum(["safe", "magic"]),
  personaId: z.string().optional(),
});

export const refineRoute = new Hono();

refineRoute.post("/", async (context) => {
  const startedAt = performance.now();
  const parsed = refineRequestSchema.safeParse(await context.req.json());

  if (!parsed.success) {
    const response: RefineResponse = {
      ok: false,
      durationMs: Math.round(performance.now() - startedAt),
      errorCode: "NO_TEXT_SELECTED",
      message: "No text selected",
    };

    return context.json(response, 400);
  }

  const response = await refineText(parsed.data, startedAt);
  return context.json(response, response.ok ? 200 : 422);
});
