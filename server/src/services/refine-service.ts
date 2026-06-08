import type { RefineRequest, RefineResponse } from "@shared";
import { getActivePersona } from "./persona-service";
import { getRuntimeConfig } from "./config-service";

export async function refineText(
  request: RefineRequest,
  startedAt = performance.now(),
): Promise<RefineResponse> {
  const config = await getRuntimeConfig();

  if (!config.apiKey) {
    return {
      ok: false,
      durationMs: elapsed(startedAt),
      errorCode: "API_CONFIG_MISSING",
      message: "API key is not configured",
    };
  }

  const persona = await getActivePersona(request.personaId);

  return {
    ok: true,
    text: `[${persona.name}] ${request.text}`,
    durationMs: elapsed(startedAt),
  };
}

function elapsed(startedAt: number) {
  return Math.round(performance.now() - startedAt);
}
